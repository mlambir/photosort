use crate::core::config::AppTheme;
use iced::Color;
use iced::theme::Palette;

pub const PAPER_WHITE: Color = Color {
    r: 0xF4 as f32 / 255.0,
    g: 0xF4 as f32 / 255.0,
    b: 0xF0 as f32 / 255.0,
    a: 1.0,
};
pub const CHARCOAL_DEEP: Color = Color {
    r: 0x11 as f32 / 255.0,
    g: 0x11 as f32 / 255.0,
    b: 0x11 as f32 / 255.0,
    a: 1.0,
};
pub const HOT_PINK: Color = Color {
    r: 0xFE as f32 / 255.0,
    g: 0x2C as f32 / 255.0,
    b: 0x55 as f32 / 255.0,
    a: 1.0,
};
pub fn get_theme(app_theme: AppTheme) -> iced::Theme {
    match app_theme {
        AppTheme::Light => iced::Theme::custom(
            "Brutalist Light".to_string(),
            Palette {
                background: PAPER_WHITE,
                text: CHARCOAL_DEEP,
                primary: HOT_PINK, // Hot Pink
                success: HOT_PINK,
                danger: HOT_PINK,
                warning: HOT_PINK,
            },
        ),
        AppTheme::Dark => iced::Theme::custom(
            "Brutalist Dark".to_string(),
            Palette {
                background: CHARCOAL_DEEP, // Dark Black
                text: PAPER_WHITE,         // Paper White
                primary: HOT_PINK,         // Hot Pink Accent
                success: HOT_PINK,
                danger: HOT_PINK,
                warning: HOT_PINK,
            },
        ),
    }
}

pub fn brutalist_button_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let (bg, text_color, border_color) = match status {
        iced::widget::button::Status::Hovered => (
            HOT_PINK, // Hot Pink
            PAPER_WHITE,
            PAPER_WHITE,
        ),
        iced::widget::button::Status::Pressed => (
            HOT_PINK, // Darker Pink
            PAPER_WHITE,
            PAPER_WHITE,
        ),
        iced::widget::button::Status::Disabled => (PAPER_WHITE, CHARCOAL_DEEP, CHARCOAL_DEEP),
        _ => (
            CHARCOAL_DEEP, // Dark background
            PAPER_WHITE,   // Off-white text
            PAPER_WHITE,   // Off-white border
        ),
    };

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

pub fn brutalist_light_button_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let (bg, text_color, border_color) = match status {
        iced::widget::button::Status::Hovered => (
            HOT_PINK, // Hot Pink
            PAPER_WHITE,
            CHARCOAL_DEEP,
        ),
        iced::widget::button::Status::Pressed => (HOT_PINK, PAPER_WHITE, CHARCOAL_DEEP),
        _ => (
            PAPER_WHITE, // Light background
            CHARCOAL_DEEP,
            CHARCOAL_DEEP,
        ),
    };

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

pub fn bold_font() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..Default::default()
    }
}

pub fn icon_font() -> iced::Font {
    iced::Font {
        family: iced::font::Family::Name("Material Icons"),
        weight: iced::font::Weight::Normal,
        ..Default::default()
    }
}

pub const ICON_REFRESH: &str = "\u{e5d5}";
pub const ICON_ZOOM_IN: &str = "\u{e8ff}";
pub const ICON_ZOOM_OUT: &str = "\u{e900}";
pub const ICON_RESET: &str = "\u{e3e4}"; // fit_screen
pub const ICON_KEEP: &str = "\u{e876}"; // check
pub const ICON_DISCARD: &str = "\u{e872}"; // delete
pub const ICON_APPLY: &str = "\u{e86c}"; // check_circle / done_all
pub const ICON_INFO: &str = "\u{e88e}"; // info
pub const ICON_CHEVRON_LEFT: &str = "\u{e5cb}";
pub const ICON_CHEVRON_RIGHT: &str = "\u{e5cc}";

pub fn brutalist_radio_style(
    theme: &iced::Theme,
    _status: iced::widget::radio::Status,
) -> iced::widget::radio::Style {
    let is_dark = theme.palette().background.r < 0.5;

    let text_color = if is_dark { CHARCOAL_DEEP } else { PAPER_WHITE };

    let bg_color = if is_dark { PAPER_WHITE } else { CHARCOAL_DEEP };

    iced::widget::radio::Style {
        background: iced::Background::Color(bg_color),
        dot_color: HOT_PINK,
        border_width: 2.0,
        border_color: HOT_PINK,
        text_color: Some(text_color),
    }
}

pub fn brutalist_card_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let is_dark = theme.palette().background.r < 0.5;

    if is_dark {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(PAPER_WHITE)),
            text_color: Some(CHARCOAL_DEEP),
            border: iced::Border {
                color: CHARCOAL_DEEP,
                width: 3.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    } else {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(CHARCOAL_DEEP)),
            text_color: Some(PAPER_WHITE),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn brutalist_card_shadow_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let is_dark = theme.palette().background.r < 0.5;

    if is_dark {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(HOT_PINK)),
            border: iced::Border {
                color: CHARCOAL_DEEP,
                width: 3.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    } else {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(HOT_PINK)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn brutalist_tab_bar_style(
    theme: &iced::Theme,
    status: iced_aw::style::Status,
) -> iced_aw::style::tab_bar::Style {
    let is_dark = theme.palette().background.r < 0.5;
    let border_color = if is_dark { PAPER_WHITE } else { CHARCOAL_DEEP };
    let _bg_color = if is_dark { CHARCOAL_DEEP } else { PAPER_WHITE };
    let fg_color = if is_dark { PAPER_WHITE } else { CHARCOAL_DEEP };

    let (tab_bg, tab_text, tab_border) = match status {
        iced_aw::style::Status::Selected => (
            HOT_PINK, // Hot Pink for selected tab
            fg_color, // Paper White text
            border_color,
        ),
        iced_aw::style::Status::Hovered => (HOT_PINK, PAPER_WHITE, border_color),
        iced_aw::style::Status::Disabled => (
            if is_dark { CHARCOAL_DEEP } else { PAPER_WHITE },
            HOT_PINK,
            border_color,
        ),
        _ => (
            if is_dark { CHARCOAL_DEEP } else { PAPER_WHITE }, // inactive tab background
            if is_dark { PAPER_WHITE } else { CHARCOAL_DEEP }, // text matches parent text
            border_color,
        ),
    };

    let bar_bg = if is_dark { CHARCOAL_DEEP } else { PAPER_WHITE };

    iced_aw::style::tab_bar::Style {
        background: Some(iced::Background::Color(bar_bg)),
        border_color: Some(border_color),
        border_width: 3.0,
        tab_border_radius: 0.0.into(),
        tab_label_background: iced::Background::Color(tab_bg),
        tab_label_border_color: tab_border,
        tab_label_border_width: 3.0,
        icon_color: tab_text,
        icon_background: Some(iced::Background::Color(Color::TRANSPARENT)),
        icon_border_radius: 0.0.into(),
        text_color: tab_text,
    }
}
