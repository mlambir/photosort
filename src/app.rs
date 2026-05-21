use iced::widget::container;
use iced::{Element, Length, Task};

use crate::core::config::Config;
use crate::ui::{import, settings, sort, theme};

pub fn run() -> iced::Result {
    iced::application(
        PhotoSortApp::new_boot,
        PhotoSortApp::update,
        PhotoSortApp::view
    )
    .title("PhotoSort")
    .theme(PhotoSortApp::theme)
    .subscription(PhotoSortApp::subscription)
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
    pub fn theme(&self) -> iced::Theme {
        theme::get_theme(self.config.theme)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        if self.active_tab == Tab::Sort {
            self.sort_state.subscription().map(Message::SortMessage)
        } else {
            iced::Subscription::none()
        }
    }

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

    pub fn view(&self) -> Element<'_, Message> {
        let tabs = iced_aw::Tabs::new(Message::TabSelected)
            .push(
                Tab::Import,
                iced_aw::TabLabel::Text("IMPORT".to_string()),
                self.import_state.view().map(Message::ImportMessage),
            )
            .push(
                Tab::Sort,
                iced_aw::TabLabel::Text("SORT".to_string()),
                self.sort_state.view().map(Message::SortMessage),
            )
            .push(
                Tab::Settings,
                iced_aw::TabLabel::Text("SETTINGS".to_string()),
                self.settings_state.view().map(Message::SettingsMessage),
            )
            .set_active_tab(&self.active_tab)
            .text_font(theme::bold_font())
            .text_size(16.0)
            .tab_bar_style(theme::brutalist_tab_bar_style);

        container(tabs)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .into()
    }
}
