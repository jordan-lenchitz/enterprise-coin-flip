use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_subscriber::fmt::format::FmtSpan;

pub fn init_telemetry() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,enterprise_coin_flip=debug,actix_web=info"));
        
    let formatting_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::FULL);
        
    let _ = Registry::default()
        .with(env_filter)
        .with(formatting_layer)
        .try_init();
        
    tracing::info!("Enterprise telemetry pipeline initialized successfully");
}
