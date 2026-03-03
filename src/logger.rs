use std::env;

use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    registry, reload,
    util::SubscriberInitExt,
};

pub fn init_tracing() -> WorkerGuard {
    let filter = EnvFilter::new(env::var("RUST_LOG").unwrap_or_else(|_| "info".into()));
    let (filter_layer, _reload_handle) = reload::Layer::new(filter);

    let file_appender = rolling::daily("logs", "crock.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let json_writer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_current_span(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE);

    registry().with(filter_layer).with(json_writer).init();
    guard
}
