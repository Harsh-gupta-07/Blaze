pub mod commands;
use commands::fetch_files;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![fetch_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
