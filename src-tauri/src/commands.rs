use std::collections::HashMap;

use blaze_core::db::{self};
use blaze_core::types::FileEntry;
use blaze_daemon;

#[tauri::command]
pub fn fetch_files() -> Result<Vec<FileEntry>, String> {
    let conn = db::get_connection().map_err(|err| err.to_string())?;

    let files = db::get_files(&conn).map_err(|err| err.to_string())?;

    Ok(files)
}

#[tauri::command]
pub fn fetch_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let conn = db::get_connection().map_err(|err| err.to_string())?;

    let files = db::get_dir_files(&conn, path).map_err(|err| err.to_string())?;

    Ok(files)
}

#[tauri::command]
pub fn daemon_status() -> HashMap<String, bool> {
    let map = blaze_daemon::start::get_status();
    return map;
}

#[tauri::command]
pub fn start_daemon_service() -> bool {
    let stat = blaze_daemon::start::tauri_start_service();
    return stat;
}
