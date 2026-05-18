use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};

use crate::core::config::Config;
use crate::ui::{import, settings, sort};

pub fn run() -> iced::Result {
    iced::application(
        PhotoSortApp::new_boot,
        PhotoSortApp::update,
        PhotoSortApp::view
    )
    .title("PhotoSort")
    .run()
}

pub struct PhotoSortApp {
    config: Config,
    active_tab: Tab,
    import_state: import::State,
    sort_state: sort::State,
    settings_state: settings::State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tab {
    Import,
    Sort,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(Tab),
    ImportMessage(import::Message),
    SortMessage(sort::Message),
    SettingsMessage(settings::Message),
}

impl PhotoSortApp {
    pub fn new_boot() -> (Self, Task<Message>) {
        let config = Config::load().unwrap_or_default();
        (Self::new(config), Task::none())
    }

    pub fn new(config: Config) -> Self {
        Self {
            active_tab: Tab::Import,
            import_state: import::State::new(&config),
            sort_state: sort::State::new(&config),
            settings_state: settings::State::new(&config),
            config,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                
                if self.active_tab == Tab::Sort {
                    return self.sort_state.refresh(&self.config).map(Message::SortMessage);
                }
                
                Task::none()
            }
            Message::ImportMessage(msg) => {
                self.import_state.update(msg, &self.config).map(Message::ImportMessage)
            }
            Message::SortMessage(msg) => {
                self.sort_state.update(msg, &self.config).map(Message::SortMessage)
            }
            Message::SettingsMessage(msg) => {
                let (task, new_config) = self.settings_state.update(msg, &self.config);
                if let Some(c) = new_config {
                    self.config = c.clone();
                    self.config.save().ok();
                    self.import_state.update_config(&self.config);
                    self.sort_state.update_config(&self.config);
                }
                task.map(Message::SettingsMessage)
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let tabs = row![
            button(text("Import").size(16))
                .on_press(Message::TabSelected(Tab::Import)),
            button(text("Sort").size(16))
                .on_press(Message::TabSelected(Tab::Sort)),
            button(text("Settings").size(16))
                .on_press(Message::TabSelected(Tab::Settings)),
        ]
        .spacing(10)
        .padding(10);

        let content: Element<Message> = match self.active_tab {
            Tab::Import => self.import_state.view().map(Message::ImportMessage),
            Tab::Sort => self.sort_state.view().map(Message::SortMessage),
            Tab::Settings => self.settings_state.view().map(Message::SettingsMessage),
        };

        column![
            tabs,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20)
        ]
        .into()
    }
}
