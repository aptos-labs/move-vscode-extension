#[allow(unused)]
#[test]
fn test_run_main_loop() -> anyhow::Result<()> {
    use crate::Config;
    use crate::global_state::GlobalState;
    use crate::tracing::LoggingConfig;
    use camino::Utf8PathBuf;
    use lsp_server::Connection;
    use lsp_types::WindowClientCapabilities;
    use paths::AbsPathBuf;
    use tracing::Level;
    use tracing_subscriber::fmt::writer::BoxMakeWriter;

    LoggingConfig {
        writer: BoxMakeWriter::new(std::io::stdout),
        default_level: Level::DEBUG,
    }
    .try_init()
    .unwrap();

    let (connection, io_threads) = Connection::stdio();

    let package_root = AbsPathBuf::assert(Utf8PathBuf::from(
        "/home/mkurnikov/code/aptos-core/aptos-move/framework/aptos-stdlib",
    ));
    let capabilities = lsp_types::ClientCapabilities {
        window: Some(WindowClientCapabilities {
            work_done_progress: Some(false),
            ..WindowClientCapabilities::default()
        }),
        ..lsp_types::ClientCapabilities::default()
    };
    let mut config = Config::new(package_root.clone(), capabilities, vec![package_root]);
    config.rediscover_packages();

    let global_state = GlobalState::new(connection.sender.clone(), config);
    {
        let vfs = &global_state.vfs.read().0;
        // dbg!(vfs.iter().collect::<Vec<_>>());
    }

    // connection
    //     .sender
    //     .send(lsp_server::Message::Request(lsp_server::Request::new(
    //         RequestId::from(1),
    //         "shutdown".to_string(),
    //         (),
    //     )))
    //     .unwrap();
    // connection
    //     .sender
    //     .send(lsp_server::Message::Notification(lsp_server::Notification::new(
    //         "exit".to_string(),
    //         (),
    //     )))
    //     .unwrap();

    // global_state.run(connection.receiver)?;

    // io_threads.join()?;
    // let db = global_state.analysis_host.raw_database();
    // dbg!(&db.all_package_ids().data(db));

    Ok(())
}
