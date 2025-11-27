use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

    /// Path of the configuration file
    #[clap(long, short)]
    pub config: camino::Utf8PathBuf,
}
