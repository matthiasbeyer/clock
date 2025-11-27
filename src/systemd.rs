#[derive(Debug)]
pub struct ProcessState {
    pub span: tracing::Span,
}

#[expect(dead_code, reason = "Not all member fns used yet")]
impl ProcessState {
    pub fn set_starting(&self) {
        tracing::debug!(parent: &self.span, status = "starting", "Setting service status");
        if let Err(error) = notify(&[NotifyState::Status("starting")]) {
            tracing::error!(parent: &self.span, ?error, "Failed to notify systemd of state change");
        } else {
            tracing::info!(
                parent: &self.span,
                status = "starting",
                "Successfully notified systemd of service status"
            );
        }
    }

    pub fn set_running(&self) {
        tracing::debug!(parent: &self.span, status = "ready", "Setting service status");
        if let Err(error) = notify(&[NotifyState::Ready]) {
            tracing::error!(parent: &self.span, ?error, "Failed to notify systemd of state change");
        } else {
            tracing::info!(parent: &self.span,
                status = "ready",
                "Successfully notified systemd of service status"
            );
        }
    }

    pub fn set_failed(&self) {
        tracing::debug!(parent: &self.span, status = "failed,stopping", "Setting service status");
        if let Err(error) = notify(&[NotifyState::Status("failed"), NotifyState::Stopping]) {
            tracing::error!(parent: &self.span, ?error, "Failed to notify systemd of state change");
        } else {
            tracing::info!(
                parent: &self.span,
                status = "failed,stopping",
                "Successfully notified systemd of service status"
            );
        }
    }

    pub fn set_cancelled(&self) {
        tracing::debug!(parent: &self.span, status = "ECANCELED", "Setting service status");
        // verified highly scientificly by looking up https://docs.rs/nix/latest/nix/type.Error.html#variant.ECANCELED
        let ecanceled = 125;

        if let Err(error) = notify(&[NotifyState::Errno(ecanceled)]) {
            tracing::error!(parent: &self.span, ?error, "Failed to notify systemd of state change");
        } else {
            tracing::info!(
                parent: &self.span,
                status = "ECANCELED",
                "Successfully notified systemd of service status"
            );
        }
    }

    pub fn set_finished(&self) {
        tracing::debug!(parent: &self.span, status = "stopping", "Setting service status");
        if let Err(error) = notify(&[NotifyState::Stopping]) {
            tracing::error!(parent: &self.span, ?error, "Failed to notify systemd of state change");
        } else {
            tracing::info!(
                parent: &self.span,
                status = "stopping",
                "Successfully notified systemd of service status"
            );
        }
    }
}

/// Daemon notification for the service manager.
#[derive(Clone, Debug)]
#[allow(dead_code)] // TODO: Delete unused variants?
enum NotifyState<'a> {
    /// Service startup is finished.
    Ready,

    /// Service is reloading its configuration.
    ///
    /// On systemd v253 and newer, this message MUST be followed by a
    /// [`NotifyState::MonotonicUsec`] notification, or the reload will fail
    /// and the service will be terminated.
    Reloading,

    /// Service is stopping.
    Stopping,

    /// Free-form status message for the service manager.
    Status(&'a str),

    /// Service has failed with an `errno`-style error code, e.g. `2` for `ENOENT`.
    Errno(u32),

    /// Service has failed with a D-Bus-style error code, e.g. `org.freedesktop.DBus.Error.TimedOut`.
    BusError(&'a str),

    /// Main process ID (PID) of the service, in case it wasn't started directly by the service manager.
    MainPid(u32),

    /// Tells the service manager to update the watchdog timestamp.
    Watchdog,

    /// Tells the service manager to trigger a watchdog failure.
    WatchdogTrigger,

    /// Resets the configured watchdog value.
    WatchdogUsec(u32),

    /// Tells the service manager to extend the service timeout.
    ExtendTimeoutUsec(u32),

    /// Tells the service manager to store attached file descriptors.
    FdStore,

    /// Tells the service manager to remove stored file descriptors.
    FdStoreRemove,

    /// Tells the service manager to use this name for the attached file descriptor.
    FdName(&'a str),

    /// Notify systemd of the current monotonic time in microseconds.
    /// You can construct this value by calling [`NotifyState::monotonic_usec_now()`].
    MonotonicUsec(i128),

    /// Custom state.
    Custom(&'a str),
}

impl std::fmt::Display for NotifyState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyState::Ready => write!(f, "READY=1"),
            NotifyState::Reloading => write!(f, "RELOADING=1"),
            NotifyState::Stopping => write!(f, "STOPPING=1"),
            NotifyState::Status(msg) => write!(f, "STATUS={msg}"),
            NotifyState::Errno(err) => write!(f, "ERRNO={err}"),
            NotifyState::BusError(addr) => write!(f, "BUSERROR={addr}"),
            NotifyState::MainPid(pid) => write!(f, "MAINPID={pid}"),
            NotifyState::Watchdog => write!(f, "WATCHDOG=1"),
            NotifyState::WatchdogTrigger => write!(f, "WATCHDOG=trigger"),
            NotifyState::WatchdogUsec(usec) => write!(f, "WATCHDOG_USEC={usec}"),
            NotifyState::ExtendTimeoutUsec(usec) => write!(f, "EXTEND_TIMEOUT_USEC={usec}"),
            NotifyState::FdStore => write!(f, "FDSTORE=1"),
            NotifyState::FdStoreRemove => write!(f, "FDSTOREREMOVE=1"),
            NotifyState::FdName(name) => write!(f, "FDNAME={name}"),
            NotifyState::MonotonicUsec(usec) => write!(f, "MONOTONIC_USEC={usec}"),
            NotifyState::Custom(state) => write!(f, "{state}"),
        }
    }
}

fn connect_notify_socket() -> std::io::Result<Option<std::os::unix::net::UnixDatagram>> {
    let Some(socket_path) = std::env::var_os("NOTIFY_SOCKET") else {
        return Ok(None);
    };

    let sock = std::os::unix::net::UnixDatagram::unbound()?;

    sock.connect(socket_path)?;

    Ok(Some(sock))
}

fn notify(state: &[NotifyState]) -> std::io::Result<()> {
    use std::fmt::Write;

    let mut msg = String::new();

    let Some(sock) = connect_notify_socket()? else {
        return Ok(());
    };

    for s in state {
        let _ = writeln!(msg, "{s}");
    }

    let len = sock.send(msg.as_bytes())?;

    if len != msg.len() {
        Err(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            "incomplete write",
        ))
    } else {
        Ok(())
    }
}
