#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Setting up error reporting failed")]
    InstallingColorEyre(#[source] color_eyre::Report),

    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
}
