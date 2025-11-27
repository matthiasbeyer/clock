mod cli;
mod config;
mod error;
mod logging;
mod systemd;

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::eyre::Result<()> {
    setup_panic();
    color_eyre::install().map_err(crate::error::Error::InstallingColorEyre)?;
    let cli = <crate::cli::Cli as clap::Parser>::parse();
    crate::logging::setup(cli.verbosity);
    let cfg = crate::config::Config::load(&cli.config).await?;

    run(cli, cfg).await?;
    Ok(())
}

fn setup_panic() {
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .authors("Matthias Beyer <mail@beyermatthias.de>"));
}

async fn run(
    _cli: crate::cli::Cli,
    _config: crate::config::Config,
) -> Result<(), crate::error::Error> {
    todo!()
}
