pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            // Start the indexer + watcher daemon on a
            // background thread.  `start()` blocks until
            // shutdown, which is fine — it's not on the
            // UI thread.
            std::thread::spawn(|| {
                blaze_daemon::start();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_files,
            commands::fetch_dir,
            commands::get_startup_status,
            commands::daemon_status,
            commands::start_daemon_service,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // When the app is about to exit, trigger a
            // graceful daemon shutdown so the indexer can
            // flush its last batch and persist the event ID.
            if let tauri::RunEvent::Exit = event {
                println!("[tauri] app exiting — shutting down daemon");
                blaze_daemon::shutdown();

                // Brief grace period for the indexer to
                // drain its last batch + commit.
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        });
}
