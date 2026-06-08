use std::fs;

use crate::types::UserData;
use crate::utils::app_data_dir;

pub fn read_user_data() -> Result<UserData, Box<dyn std::error::Error>> {
    let path = app_data_dir().join("data.json");

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<UserData>(&content) {
            Ok(user) => Ok(user),
            Err(_) => {
                let default = UserData::default();
                fs::write(&path, serde_json::to_string_pretty(&default)?)?;
                Ok(default)
            }
        },
        Err(_) => {
            let default = UserData::default();

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&path, serde_json::to_string_pretty(&default)?)?;
            Ok(default)
        }
    }
}
