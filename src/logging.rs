use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;

pub fn setup<Level>(verbosity: clap_verbosity_flag::Verbosity<Level>)
where
    Level: clap_verbosity_flag::LogLevel,
{
    let mut env_filter = tracing_subscriber::EnvFilter::from_default_env();

    if let Some(level_filter) = verbosity
        .is_present()
        .then(|| verbosity.tracing_level_filter())
    {
        let directive = tracing_subscriber::filter::Directive::from(level_filter);
        env_filter = env_filter.add_directive(directive);
    }

    let subscriber = tracing_subscriber::registry::Registry::default().with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(env_filter),
    );

    tracing::subscriber::set_global_default(subscriber).expect("Setting up logger works");
}
