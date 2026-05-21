use iced::widget::{button, column, text, row, radio, container};
use iced::{Element, Task, Length};
use std::path::PathBuf;
use crate::core::config::{Config, AppTheme};
use crate::ui::theme::{brutalist_button_style, bold_font, brutalist_radio_style, brutalist_card_style, brutalist_card_shadow_style};

pub struct State {
    to_sort_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
    theme: AppTheme,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectToSortDir,
    SelectLibraryDir,
    DirSelected(Option<PathBuf>, DirType),
    ThemeToggled(AppTheme),
}

#[derive(Debug, Clone, Copy)]
pub enum DirType {
    ToSort,
    Library,
}

impl State {
    pub fn new(config: &Config) -> Self {
        Self {
            to_sort_dir: config.to_sort_dir.clone(),
            library_dir: config.library_dir.clone(),
            theme: config.theme,
        }
    }

    pub fn update(&mut self, message: Message, _config: &Config) -> (Task<Message>, Option<Config>) {
        match message {
            Message::SelectToSortDir => {
                let task = Task::perform(
                    async {
                        rfd::AsyncFileDialog::new().pick_folder().await.map(|f| f.path().to_path_buf())
                    },
                    |p| Message::DirSelected(p, DirType::ToSort)
                );
                (task, None)
            }
            Message::SelectLibraryDir => {
                let task = Task::perform(
                    async {
                        rfd::AsyncFileDialog::new().pick_folder().await.map(|f| f.path().to_path_buf())
                    },
                    |p| Message::DirSelected(p, DirType::Library)
                );
                (task, None)
            }
            Message::DirSelected(path, dir_type) => {
                if let Some(p) = path {
                    match dir_type {
                        DirType::ToSort => self.to_sort_dir = Some(p),
                        DirType::Library => self.library_dir = Some(p),
                    }
                    let new_config = Config {
                        to_sort_dir: self.to_sort_dir.clone(),
                        library_dir: self.library_dir.clone(),
                        theme: self.theme,
                    };
                    return (Task::none(), Some(new_config));
                }
                (Task::none(), None)
            }
            Message::ThemeToggled(theme) => {
                self.theme = theme;
                let new_config = Config {
                    to_sort_dir: self.to_sort_dir.clone(),
                    library_dir: self.library_dir.clone(),
                    theme: self.theme,
                };
                (Task::none(), Some(new_config))
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let settings_info = column![
            row![
                text(format!("TO SORT: {:?}", self.to_sort_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "NOT SET".to_string())))
                    .font(bold_font())
                    .size(14),
                iced::widget::Space::new().width(Length::Fill),
                button(text("CHANGE").font(bold_font()))
                    .on_press(Message::SelectToSortDir)
                    .padding(8)
                    .style(brutalist_button_style),
            ].align_y(iced::Alignment::Center),
            
            row![
                text(format!("LIBRARY: {:?}", self.library_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "NOT SET".to_string())))
                    .font(bold_font())
                    .size(14),
                iced::widget::Space::new().width(Length::Fill),
                button(text("CHANGE").font(bold_font()))
                    .on_press(Message::SelectLibraryDir)
                    .padding(8)
                    .style(brutalist_button_style),
            ].align_y(iced::Alignment::Center),
            
            iced::widget::Space::new().height(10),
            text("THEME").font(bold_font()).size(16),
            row![
                radio("Light", AppTheme::Light, Some(self.theme), Message::ThemeToggled)
                    .font(bold_font())
                    .style(brutalist_radio_style),
                radio("Dark", AppTheme::Dark, Some(self.theme), Message::ThemeToggled)
                    .font(bold_font())
                    .style(brutalist_radio_style),
            ].spacing(20)
        ].spacing(20);

        let inner_card = container(settings_info)
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
            text("SETTINGS").font(bold_font()).size(28),
            card
        ]
        .spacing(20)
        .padding(20)
        .into()
    }
}
