#[derive(Debug, serde::Deserialize)]
pub struct Config {}

impl Config {
    pub async fn load(path: &camino::Utf8Path) -> Result<Self, ConfigError> {
        let config_str =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|source| ConfigError::ReadingFile {
                    path: path.to_path_buf(),
                    source,
                })?;

        toml::from_str(&config_str).map_err(ConfigError::ParsingConfig)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file from path '{}'", .path)]
    ReadingFile {
        path: camino::Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    ParsingConfig(#[from] toml::de::Error),
}
