use iced::widget::{button, column, text, row, progress_bar, container};
use iced::{Element, Task, Length};
use crate::core::config::Config;
use crate::core::photo::Photo;
use crate::core::importer::Importer;
use crate::ui::theme::{brutalist_button_style, brutalist_light_button_style, bold_font, brutalist_card_style, brutalist_card_shadow_style};
use std::path::PathBuf;

pub struct State {
    status: String,
    source_dir: Option<PathBuf>,
    photos_to_import: Vec<Photo>,
    is_importing: bool,
    import_progress: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectSource,
    SourceSelected(Option<PathBuf>),
    ScanComplete(Vec<Photo>),
    StartImport,
    ImportComplete(()),
}

impl State {
    pub fn new(_config: &Config) -> Self {
        Self {
            status: "Ready. Select a source directory.".to_string(),
            source_dir: None,
            photos_to_import: Vec::new(),
            is_importing: false,
            import_progress: 0.0,
        }
    }

    pub fn update_config(&mut self, _config: &Config) {}

    pub fn update(&mut self, message: Message, config: &Config) -> Task<Message> {
        match message {
            Message::SelectSource => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new().pick_folder().await.map(|f| f.path().to_path_buf())
                    },
                    Message::SourceSelected
                )
            }
            Message::SourceSelected(path) => {
                if let Some(p) = path {
                    self.source_dir = Some(p.clone());
                    self.status = format!("Scanning {:?}...", p);
                    
                    Task::perform(
                        async move {
                            Importer::scan_directory(&p)
                        },
                        Message::ScanComplete
                    )
                } else {
                    Task::none()
                }
            }
            Message::ScanComplete(photos) => {
                self.photos_to_import = photos;
                self.status = format!("Found {} photos. Ready to import.", self.photos_to_import.len());
                Task::none()
            }
            Message::StartImport => {
                if let Some(to_sort) = &config.to_sort_dir {
                    if !self.photos_to_import.is_empty() {
                        self.is_importing = true;
                        self.import_progress = 0.5; // Dummy progress
                        self.status = "Importing...".to_string();
                        
                        let target_dir = to_sort.clone();
                        let photos = self.photos_to_import.clone();
                        
                        Task::perform(
                            async move {
                                let _ = std::fs::create_dir_all(&target_dir);
                                for mut photo in photos {
                                    let _ = Importer::copy_and_rename(&mut photo, &target_dir);
                                }
                            },
                            |_| Message::ImportComplete(())
                        )
                    } else {
                        self.status = "No photos to import.".to_string();
                        Task::none()
                    }
                } else {
                    self.status = "Please configure a 'To Sort' directory in Settings first.".to_string();
                    Task::none()
                }
            }
            Message::ImportComplete(()) => {
                self.is_importing = false;
                self.import_progress = 1.0;
                self.status = "Import complete!".to_string();
                self.photos_to_import.clear();
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut import_info = column![
            row![
                button(text("SELECT SOURCE").font(bold_font()))
                    .on_press(Message::SelectSource)
                    .padding(10)
                    .style(brutalist_button_style),
                iced::widget::Space::new().width(10),
                text(self.source_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "NONE".to_string()))
                    .font(bold_font())
                    .size(14)
            ].align_y(iced::Alignment::Center),
            text(self.status.to_uppercase()).font(bold_font()).size(14),
        ].spacing(15);

        if !self.photos_to_import.is_empty() && !self.is_importing {
            import_info = import_info.push(
                button(text("START IMPORT").font(bold_font()))
                    .on_press(Message::StartImport)
                    .padding(iced::Padding {
                        top: 12.0,
                        bottom: 12.0,
                        left: 24.0,
                        right: 24.0,
                    })
                    .style(brutalist_light_button_style)
            );
        } else if self.is_importing {
            import_info = import_info.push(
                button(text("IMPORTING...").font(bold_font()))
                    .padding(10)
                    .style(brutalist_button_style)
            );
            import_info = import_info.push(progress_bar(0.0..=1.0, self.import_progress));
        }

        let inner_card = container(import_info)
            .padding(20)
            .width(Length::Fill)
            .style(brutalist_card_style);

        let card = container(inner_card)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                left: 0.0,
                bottom: 8.0,
                right: 8.0,
            })
            .style(brutalist_card_shadow_style);

        column![
            text("IMPORT PHOTOS").font(bold_font()).size(28),
            card
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}
