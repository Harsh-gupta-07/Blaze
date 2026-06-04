pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::fetch_files,commands::fetch_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
