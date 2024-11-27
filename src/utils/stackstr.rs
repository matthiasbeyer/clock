use core::{fmt, str::from_utf8_unchecked};
use core::mem::MaybeUninit;

pub struct StackStr<const BUF_SIZE: usize> {
    buffer: [u8; BUF_SIZE],
    used: usize,
}

impl<const BUF_SIZE: usize> StackStr<BUF_SIZE> {
    /// Creates new buffer on the stack
    pub fn new() -> Self {
        // We don't need to initialize, because we write before we read
        let buffer: [u8; BUF_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        StackStr { buffer, used: 0 }
    }

    /// Format numbers and strings
    pub fn format(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.used = 0; // if format is used several times
        fmt::write(self, args)
    }

    /// Get a reference to the result as a slice inside the buffer as str
    pub fn as_str(&self) -> &str {
        // We are really sure, that the buffer contains only valid utf8 characters
        unsafe { from_utf8_unchecked(&self.buffer[..self.used]) }
    }

    /// Get a reference to the result as a slice inside the buffer as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.used]
    }
}

impl<const BUF_SIZE: usize> fmt::Write for StackStr<BUF_SIZE> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining_buf = &mut self.buffer[self.used..];
        let raw_s = s.as_bytes();

        // Treat imminent buffer overflow
        if raw_s.len() > remaining_buf.len() {
            remaining_buf.copy_from_slice(&raw_s[..remaining_buf.len()]);
            self.used += remaining_buf.len();
            Err(fmt::Error)
        } else {
            remaining_buf[..raw_s.len()].copy_from_slice(raw_s);
            self.used += raw_s.len();
            Ok(())
        }
    }
}

#[macro_export]
macro_rules! stackstr {
    ($size:expr, $($arg:tt)*) => {{
        let mut ss = $crate::utils::stackstr::StackStr::<$size>::new();

        // Panic on buffer overflow
        ss.format(core::format_args!($($arg)*)).expect("Buffer overflow");
        ss
    }}
}

