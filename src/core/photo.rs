use std::path::PathBuf;
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct Photo {
    pub source_path: PathBuf,
    pub camera_model: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
    pub hash: Option<String>,
}

impl Photo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            source_path: path,
            camera_model: None,
            date_taken: None,
            hash: None,
        }
    }
}
