use iced::widget::{button, column, text, row, progress_bar, container, scrollable};
use iced::{Element, Task, Length};
use crate::core::config::Config;
use crate::core::photo::Photo;
use crate::core::importer::Importer;
use crate::ui::theme::{brutalist_button_style, brutalist_light_button_style, bold_font, brutalist_card_style, brutalist_card_shadow_style};
use std::path::PathBuf;

pub struct State {
    status: String,
    source_dir: Option<PathBuf>,
    photos_to_import: Vec<(Photo, bool)>,
    is_importing: bool,
    import_progress: f32,
    is_filetype_dropdown_open: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectSource,
    SourceSelected(Option<PathBuf>),
    ScanComplete(Vec<Photo>),
    ToggleSelect(usize),
    SelectAll,
    DeselectAll,
    ToggleFiletypeDropdown,
    SelectByFiletype(String),
    DeselectByFiletype(String),
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
            is_filetype_dropdown_open: false,
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
                self.photos_to_import = photos.into_iter().map(|p| (p, true)).collect();
                self.is_filetype_dropdown_open = false;
                self.update_status();
                Task::none()
            }
            Message::ToggleSelect(idx) => {
                if let Some(item) = self.photos_to_import.get_mut(idx) {
                    item.1 = !item.1;
                }
                self.update_status();
                Task::none()
            }
            Message::SelectAll => {
                for item in &mut self.photos_to_import {
                    item.1 = true;
                }
                self.update_status();
                Task::none()
            }
            Message::DeselectAll => {
                for item in &mut self.photos_to_import {
                    item.1 = false;
                }
                self.update_status();
                Task::none()
            }
            Message::ToggleFiletypeDropdown => {
                self.is_filetype_dropdown_open = !self.is_filetype_dropdown_open;
                Task::none()
            }
            Message::SelectByFiletype(ext) => {
                let target = ext.to_lowercase();
                for item in &mut self.photos_to_import {
                    if let Some(item_ext) = item.0.source_path.extension().and_then(|e| e.to_str()) {
                        if item_ext.to_lowercase() == target {
                            item.1 = true;
                        }
                    }
                }
                self.update_status();
                Task::none()
            }
            Message::DeselectByFiletype(ext) => {
                let target = ext.to_lowercase();
                for item in &mut self.photos_to_import {
                    if let Some(item_ext) = item.0.source_path.extension().and_then(|e| e.to_str()) {
                        if item_ext.to_lowercase() == target {
                            item.1 = false;
                        }
                    }
                }
                self.update_status();
                Task::none()
            }
            Message::StartImport => {
                if let Some(to_sort) = &config.to_sort_dir {
                    let selected_photos: Vec<Photo> = self.photos_to_import
                        .iter()
                        .filter(|(_, selected)| *selected)
                        .map(|(p, _)| p.clone())
                        .collect();

                    if !selected_photos.is_empty() {
                        self.is_importing = true;
                        self.import_progress = 0.5; // Dummy progress
                        self.status = "Importing...".to_string();
                        
                        let target_dir = to_sort.clone();
                        let photos = selected_photos;
                        
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
                        self.status = "No photos selected to import.".to_string();
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

    fn update_status(&mut self) {
        let total = self.photos_to_import.len();
        let selected = self.photos_to_import.iter().filter(|(_, sel)| *sel).count();
        self.status = format!("Found {} photos. {} selected. Ready to import.", total, selected);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected_count = self.photos_to_import.iter().filter(|(_, sel)| *sel).count();
        let total_count = self.photos_to_import.len();

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
                button(text(format!("START IMPORT ({}/{})", selected_count, total_count)).font(bold_font()))
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

        let mut main_col = column![
            text("IMPORT PHOTOS").font(bold_font()).size(28),
            card
        ]
        .spacing(20);

        if !self.photos_to_import.is_empty() {
            let mut filetypes: Vec<String> = self.photos_to_import.iter()
                .filter_map(|(photo, _)| photo.source_path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase()))
                .collect();
            filetypes.sort();
            filetypes.dedup();

            let mut list_col = column![].spacing(10);
            
            for (idx, (photo, selected)) in self.photos_to_import.iter().enumerate() {
                let relative_path = if let Some(source_dir) = &self.source_dir {
                    if let Ok(rel) = photo.source_path.strip_prefix(source_dir) {
                        rel.to_string_lossy().to_string()
                    } else {
                        photo.source_path.file_name().unwrap_or_default().to_string_lossy().to_string()
                    }
                } else {
                    photo.source_path.file_name().unwrap_or_default().to_string_lossy().to_string()
                };

                let file_size_str = if let Ok(meta) = std::fs::metadata(&photo.source_path) {
                    let bytes = meta.len();
                    if bytes >= 1_048_576 {
                        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
                    } else {
                        format!("{:.1} KB", bytes as f64 / 1024.0)
                    }
                } else {
                    "UNKNOWN SIZE".to_string()
                };

                let camera_str = photo.camera_model.as_ref().map(|c| c.to_uppercase()).unwrap_or_else(|| "NO EXIF CAMERA".to_string());
                let date_str = photo.date_taken.as_ref().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "NO EXIF DATE".to_string());

                let checkbox_btn = button(
                    text(if *selected { "✓" } else { " " })
                        .font(bold_font())
                        .size(14)
                        .align_x(iced::alignment::Horizontal::Center)
                )
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .style(if *selected { checked_style } else { unchecked_style })
                .on_press(Message::ToggleSelect(idx));

                let row_item = container(
                    row![
                        checkbox_btn,
                        iced::widget::Space::new().width(12),
                        column![
                            text(relative_path.to_uppercase()).font(bold_font()).size(14),
                            row![
                                text(file_size_str).size(11).font(bold_font()),
                                text(" | ").size(11).font(bold_font()),
                                text(camera_str).size(11).font(bold_font()),
                                text(" | ").size(11).font(bold_font()),
                                text(date_str).size(11).font(bold_font()),
                            ]
                        ]
                    ].align_y(iced::Alignment::Center)
                )
                .padding(10)
                .width(Length::Fill)
                .style(move |theme: &iced::Theme| {
                    let is_dark = theme.palette().background.r < 0.5;
                    let border_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
                    
                    let (bg, text_color) = if *selected {
                        if is_dark {
                            // Dark mode (light card): we want a light-themed highlight and dark text
                            (
                                iced::Color::from_rgb(0.90, 0.90, 0.88),
                                Some(crate::ui::theme::CHARCOAL_DEEP)
                            )
                        } else {
                            // Light mode (dark card): we want a dark-themed highlight and light text
                            (
                                iced::Color::from_rgb(0.18, 0.18, 0.18),
                                Some(crate::ui::theme::PAPER_WHITE)
                            )
                        }
                    } else {
                        // Unselected
                        if is_dark {
                            (iced::Color::TRANSPARENT, Some(crate::ui::theme::CHARCOAL_DEEP))
                        } else {
                            (iced::Color::TRANSPARENT, Some(crate::ui::theme::PAPER_WHITE))
                        }
                    };
                    
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(bg)),
                        text_color,
                        border: iced::Border {
                            color: border_color,
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }
                });

                list_col = list_col.push(row_item);
            }

            let mut dropdown_panel = None;
            if self.is_filetype_dropdown_open && !filetypes.is_empty() {
                let mut filetype_controls = row![].spacing(20).align_y(iced::Alignment::Center);
                
                for ext in &filetypes {
                    let ext_upper = ext.to_uppercase();
                    let ctrl = row![
                        text(ext_upper).font(bold_font()).size(14),
                        iced::widget::Space::new().width(8),
                        button(text("SELECT").font(bold_font()))
                            .on_press(Message::SelectByFiletype(ext.clone()))
                            .padding(iced::Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 8.0,
                                right: 8.0,
                            })
                            .style(brutalist_light_button_style),
                        iced::widget::Space::new().width(6),
                        button(text("DESELECT").font(bold_font()))
                            .on_press(Message::DeselectByFiletype(ext.clone()))
                            .padding(iced::Padding {
                                top: 4.0,
                                bottom: 4.0,
                                left: 8.0,
                                right: 8.0,
                            })
                            .style(brutalist_button_style),
                    ].align_y(iced::Alignment::Center);
                    
                    filetype_controls = filetype_controls.push(ctrl);
                }
                
                let dropdown_box = container(
                    column![
                        text("BULK ACTIONS BY FILETYPE").font(bold_font()).size(12),
                        iced::widget::Space::new().height(2),
                        filetype_controls
                    ].spacing(2)
                )
                .padding(8)
                .width(Length::Fill)
                .style(move |theme: &iced::Theme| {
                    let is_dark = theme.palette().background.r < 0.5;
                    let text_color = if is_dark { crate::ui::theme::CHARCOAL_DEEP } else { crate::ui::theme::PAPER_WHITE };
                    let border_color = if is_dark { crate::ui::theme::CHARCOAL_DEEP } else { crate::ui::theme::PAPER_WHITE };
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                        text_color: Some(text_color),
                        border: iced::Border {
                            color: border_color,
                            width: 2.0,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }
                });
                
                dropdown_panel = Some(dropdown_box);
            }

            let list_header = row![
                text(format!("SCANNED IMAGES ({} SELECTED)", selected_count))
                    .font(bold_font())
                    .size(16),
                iced::widget::Space::new().width(Length::Fill),
                button(text(if self.is_filetype_dropdown_open { "FILETYPE ▴" } else { "FILETYPE ▾" }).font(bold_font()))
                    .on_press(Message::ToggleFiletypeDropdown)
                    .padding(6)
                    .style(brutalist_button_style),
                iced::widget::Space::new().width(10),
                button(text("SELECT ALL").font(bold_font()))
                    .on_press(Message::SelectAll)
                    .padding(6)
                    .style(brutalist_button_style),
                iced::widget::Space::new().width(10),
                button(text("DESELECT ALL").font(bold_font()))
                    .on_press(Message::DeselectAll)
                    .padding(6)
                    .style(brutalist_button_style),
            ].align_y(iced::Alignment::Center);

            let scrollable_list = scrollable(list_col)
                .height(Length::Fixed(350.0))
                .width(Length::Fill);

            let mut list_card_contents = column![
                list_header,
            ].spacing(8);
            
            if let Some(panel) = dropdown_panel {
                list_card_contents = list_card_contents.push(panel);
            } else {
                list_card_contents = list_card_contents.push(iced::widget::Space::new().height(4));
            }
            
            list_card_contents = list_card_contents.push(scrollable_list);

            let inner_list_card = container(list_card_contents)
                .padding(20)
                .width(Length::Fill)
                .style(brutalist_card_style);

            let list_card = container(inner_list_card)
                .width(Length::Fill)
                .padding(iced::Padding {
                    top: 0.0,
                    left: 0.0,
                    bottom: 8.0,
                    right: 8.0,
                })
                .style(brutalist_card_shadow_style);

            main_col = main_col.push(list_card);
        }

        main_col.padding(20).into()
    }
}

fn checked_style(theme: &iced::Theme, _status: iced::widget::button::Status) -> iced::widget::button::Style {
    let is_dark = theme.palette().background.r < 0.5;
    let bg = crate::ui::theme::HOT_PINK;
    let text_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
    let border_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
    
    iced::widget::button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: iced::Border {
            color: border_color,
            width: 2.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

fn unchecked_style(theme: &iced::Theme, _status: iced::widget::button::Status) -> iced::widget::button::Style {
    let is_dark = theme.palette().background.r < 0.5;
    let bg = if is_dark { crate::ui::theme::CHARCOAL_DEEP } else { crate::ui::theme::PAPER_WHITE };
    let text_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
    let border_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
    
    iced::widget::button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: iced::Border {
            color: border_color,
            width: 2.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}
