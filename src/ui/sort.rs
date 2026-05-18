use iced::widget::{button, column, row, text, image};
use iced::{Element, Task, Length};
use crate::core::config::Config;
use std::path::PathBuf;
use std::fs;

pub struct State {
    status: String,
    images: Vec<PathBuf>,
    current_index: usize,
    to_sort_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Keep,
    Discard,
    Refreshed(Vec<PathBuf>),
    ActionComplete(Result<(), String>),
}

impl State {
    pub fn new(config: &Config) -> Self {
        Self {
            status: "No images to sort".to_string(),
            images: Vec::new(),
            current_index: 0,
            to_sort_dir: config.to_sort_dir.clone(),
            library_dir: config.library_dir.clone(),
        }
    }

    pub fn update_config(&mut self, config: &Config) {
        self.to_sort_dir = config.to_sort_dir.clone();
        self.library_dir = config.library_dir.clone();
    }

    pub fn refresh(&mut self, config: &Config) -> Task<Message> {
        self.update_config(config);
        
        if let Some(to_sort) = &self.to_sort_dir {
            let dir = to_sort.clone();
            Task::perform(
                async move {
                    let mut found = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    let ext = ext.to_lowercase();
                                    if ["jpg", "jpeg", "png", "bmp", "tiff", "webp"].contains(&ext.as_str()) {
                                        found.push(path);
                                    }
                                }
                            }
                        }
                    }
                    found.sort();
                    found
                },
                Message::Refreshed
            )
        } else {
            self.status = "To Sort directory not configured.".to_string();
            Task::none()
        }
    }

    pub fn update(&mut self, message: Message, _config: &Config) -> Task<Message> {
        match message {
            Message::Refreshed(images) => {
                self.images = images;
                self.current_index = 0;
                if self.images.is_empty() {
                    self.status = "All caught up! No images to sort.".to_string();
                } else {
                    self.status = format!("{} images remaining.", self.images.len());
                }
                Task::none()
            }
            Message::Keep => {
                if let Some(library) = &self.library_dir {
                    if self.current_index < self.images.len() {
                        let path = self.images[self.current_index].clone();
                        let target = library.join(path.file_name().unwrap());
                        
                        self.current_index += 1;
                        self.update_status();

                        return Task::perform(
                            async move {
                                let _ = std::fs::create_dir_all(target.parent().unwrap());
                                if fs::rename(&path, &target).is_err() {
                                    if fs::copy(&path, &target).is_ok() {
                                        let _ = fs::remove_file(&path);
                                    }
                                }
                                Ok(())
                            },
                            Message::ActionComplete
                        );
                    }
                } else {
                    self.status = "Library directory not configured!".to_string();
                }
                Task::none()
            }
            Message::Discard => {
                if self.current_index < self.images.len() {
                    let path = self.images[self.current_index].clone();
                    
                    self.current_index += 1;
                    self.update_status();

                    return Task::perform(
                        async move {
                            let _ = fs::remove_file(&path);
                            Ok(())
                        },
                        Message::ActionComplete
                    );
                }
                Task::none()
            }
            Message::ActionComplete(_) => Task::none(),
        }
    }

    fn update_status(&mut self) {
        let remaining = self.images.len() - self.current_index;
        if remaining == 0 {
            self.status = "All caught up!".to_string();
        } else {
            self.status = format!("{} images remaining.", remaining);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut col = column![
            text("Sort Photos").size(24),
            text(&self.status),
        ].spacing(20);

        if self.current_index < self.images.len() {
            let current_path = &self.images[self.current_index];
            
            let img = image(current_path.to_string_lossy().to_string())
                .width(Length::Fill)
                .height(Length::Fill);

            col = col.push(img);
            
            col = col.push(
                row![
                    button("Discard (Left)").on_press(Message::Discard),
                    button("Keep (Right)").on_press(Message::Keep),
                ].spacing(20)
            );
        }

        col.into()
    }
}
