use tracing::Level;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

pub fn init_tracing_for_test() {
    let config = aptos_analyzer::tracing::LoggingConfig {
        writer: BoxMakeWriter::new(std::io::stdout),
        default_level: Level::DEBUG,
    };
    config.try_init().unwrap();
}
