pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            // Start the indexer + watcher daemon in the
            // background before the window opens.
            // `blaze_daemon::start()` creates its own
            // tokio runtime, performs warm/cold startup,
            // spawns the watcher and indexer, then returns.
            std::thread::spawn(|| {
                blaze_daemon::start();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fetch_files,
            commands::fetch_dir,
            commands::get_startup_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
