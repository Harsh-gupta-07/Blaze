use blaze_core::db;
use blaze_core::types::FileEntry;

#[tauri::command]
pub fn fetch_files() -> Result<Vec<FileEntry>, String> {
    let conn = db::get_connection().map_err(|err| err.to_string())?;

    let files = db::get_files(&conn).map_err(|err| err.to_string())?;

    Ok(files)
}
