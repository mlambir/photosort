use iced::widget::{button, column, text, row};
use iced::{Element, Task};
use std::path::PathBuf;
use crate::core::config::Config;

pub struct State {
    to_sort_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectToSortDir,
    SelectLibraryDir,
    DirSelected(Option<PathBuf>, DirType),
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
                    };
                    return (Task::none(), Some(new_config));
                }
                (Task::none(), None)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Settings").size(24),
            row![
                text(format!("To Sort Directory: {:?}", self.to_sort_dir)),
                button("Change").on_press(Message::SelectToSortDir),
            ].spacing(10),
            row![
                text(format!("Library Directory: {:?}", self.library_dir)),
                button("Change").on_press(Message::SelectLibraryDir),
            ].spacing(10)
        ]
        .spacing(20)
        .into()
    }
}
