use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use blake3::Hasher;
use crate::core::photo::Photo;
use std::io::Read;

pub struct Importer;

impl Importer {
    pub fn scan_directory(path: &Path) -> Vec<Photo> {
        let mut photos = Vec::new();
        
        let extensions = ["jpg", "jpeg", "png", "raw", "cr2", "nef", "arw", "tiff"];

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.components().any(|c| c.as_os_str() == ".thumbnail_cache") {
                continue;
            }
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext.to_lowercase().as_str()) {
                        let mut photo = Photo::new(path.to_path_buf());
                        Self::extract_exif(&mut photo);
                        photos.push(photo);
                    }
                }
            }
        }
        photos
    }

    fn extract_exif(photo: &mut Photo) {
        if let Ok(file) = std::fs::File::open(&photo.source_path) {
            let mut bufreader = std::io::BufReader::new(file);
            let exif_reader = exif::Reader::new();
            if let Ok(exif) = exif_reader.read_from_container(&mut bufreader) {
                
                if let Some(field) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(val) = vec.first() {
                            if let Ok(s) = std::str::from_utf8(val) {
                                photo.camera_model = Some(s.trim().to_string());
                            }
                        }
                    }
                }

                if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(val) = vec.first() {
                            if let Ok(s) = std::str::from_utf8(val) {
                                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y:%m:%d %H:%M:%S") {
                                    photo.date_taken = Some(dt);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn hash_file(path: &Path) -> std::io::Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Hasher::new();
        let mut buffer = [0; 8192];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        Ok(hasher.finalize().to_hex().to_string())
    }

    pub fn copy_and_rename(photo: &mut Photo, target_dir: &Path) -> std::io::Result<()> {
        let ext = photo.source_path.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
        
        let mut base_name = String::new();
        if let Some(camera) = &photo.camera_model {
            base_name.push_str(&camera.replace(" ", "_"));
            base_name.push('_');
        } else {
            base_name.push_str("UnknownCamera_");
        }

        if let Some(date) = &photo.date_taken {
            base_name.push_str(&date.format("%Y%m%d_%H%M%S").to_string());
        } else {
            base_name.push_str("UnknownDate");
        }

        let mut dest_path = target_dir.join(format!("{}.{}", base_name, ext));
        let mut sequence = 1;

        while dest_path.exists() {
            let src_meta = fs::metadata(&photo.source_path)?;
            let dest_meta = fs::metadata(&dest_path)?;

            if src_meta.len() == dest_meta.len() {
                if photo.hash.is_none() {
                    photo.hash = Self::hash_file(&photo.source_path).ok();
                }
                
                let dest_hash = Self::hash_file(&dest_path).ok();

                if photo.hash == dest_hash && photo.hash.is_some() {
                    // Exact duplicate
                    return Ok(());
                }
            }
            
            dest_path = target_dir.join(format!("{}_{}.{}", base_name, sequence, ext));
            sequence += 1;
        }

        fs::copy(&photo.source_path, &dest_path)?;
        Ok(())
    }
}
