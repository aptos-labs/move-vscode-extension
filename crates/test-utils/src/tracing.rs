use aptos_analyzer::tracing::LoggingConfig;
use tracing::Level;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

pub fn init_tracing_for_test() {
    let _ = LoggingConfig {
        writer: BoxMakeWriter::new(std::io::stdout),
        default_level: Level::DEBUG,
    }
    .try_init();
}
