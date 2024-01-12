use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn tracing_fixture() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().pretty())
        .try_init();
}
