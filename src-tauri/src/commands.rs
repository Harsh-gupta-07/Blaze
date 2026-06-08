use std::collections::HashMap;

use blaze_core::db;
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

/// Returns the last persisted FSEvents checkpoint ID from the metadata table.
/// - `Some(id)` means we did a warm restart (journal replay from that event ID)
/// - `None` means we did a cold start (full bootstrap scan)
#[tauri::command]
pub fn get_startup_status() -> Result<StartupStatus, String> {
    let conn = db::get_connection().map_err(|err| err.to_string())?;

    let event_id = db::get_metadata(&conn, "last_fsevent_id")
        .map_err(|err| err.to_string())?;

    Ok(StartupStatus {
        kind: if event_id.is_some() { "warm".into() } else { "cold".into() },
        last_event_id: event_id.and_then(|s| s.parse::<u64>().ok()),
    })
}

#[derive(serde::Serialize)]
pub struct StartupStatus {
    pub kind: String,
    pub last_event_id: Option<u64>,
}

#[tauri::command]
pub fn daemon_status()->HashMap<String, bool>{
    let map = blaze_daemon::start::get_status();
    return map
}

#[tauri::command]
pub fn start_daemon_service()-> bool{
    let stat = blaze_daemon::start::tauri_start_service();
    return stat
}