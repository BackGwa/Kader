use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const MIN_CONCURRENCY: u8 = 1;
const MAX_CONCURRENCY: u8 = 5;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Values {
    pub concurrency: u8,
    pub save_dir: PathBuf,
}

pub struct Settings {
    file: PathBuf,
    values: Mutex<Values>,
}

impl Settings {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let file = app.path().app_config_dir()?.join("settings.json");
        let stored = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Values>(&bytes).ok());
        let values = match stored {
            Some(values) => Values {
                concurrency: values.concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY),
                ..values
            },
            None => Values {
                concurrency: MIN_CONCURRENCY,
                save_dir: app.path().download_dir()?,
            },
        };
        Ok(Self {
            file,
            values: Mutex::new(values),
        })
    }

    pub fn values(&self) -> Values {
        self.values.lock().unwrap().clone()
    }

    pub fn set_concurrency(&self, concurrency: u8) -> Result<(), String> {
        let mut values = self.values.lock().unwrap();
        values.concurrency = concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        self.persist(&values)
    }

    pub fn set_save_dir(&self, save_dir: PathBuf) -> Result<(), String> {
        let mut values = self.values.lock().unwrap();
        values.save_dir = save_dir;
        self.persist(&values)
    }

    fn persist(&self, values: &Values) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(values).map_err(|error| error.to_string())?;
        if let Some(directory) = self.file.parent() {
            fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        }
        fs::write(&self.file, bytes).map_err(|error| error.to_string())
    }
}
