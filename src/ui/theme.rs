use iced::theme::Palette;
use iced::Color;
use crate::core::config::AppTheme;

pub fn get_theme(app_theme: AppTheme) -> iced::Theme {
    match app_theme {
        AppTheme::Light => iced::Theme::custom(
            "Brutalist Light".to_string(),
            Palette {
                background: Color::from_rgb8(0xF4, 0xF4, 0xF0),
                text: Color::from_rgb8(0x11, 0x11, 0x11),
                primary: Color::from_rgb8(0x33, 0x33, 0x33),
                success: Color::from_rgb8(0x00, 0xFF, 0x00),
                danger: Color::from_rgb8(0xFF, 0x00, 0x00),
                warning: Color::from_rgb8(0xFF, 0xA5, 0x00),
            },
        ),
        AppTheme::Dark => iced::Theme::custom(
            "Brutalist Dark".to_string(),
            Palette {
                background: Color::from_rgb8(0x11, 0x11, 0x11),
                text: Color::from_rgb8(0xF4, 0xF4, 0xF0),
                primary: Color::from_rgb8(0xCC, 0xCC, 0xCC),
                success: Color::from_rgb8(0x00, 0xFF, 0x00),
                danger: Color::from_rgb8(0xFF, 0x00, 0x00),
                warning: Color::from_rgb8(0xFF, 0xA5, 0x00),
            },
        ),
    }
}
