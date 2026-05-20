use iced::widget::{button, column, row, text, image, container, pane_grid, scrollable, canvas};
use iced::{Element, Task, Length, Alignment, Subscription};
use iced::widget::pane_grid::PaneGrid;
use crate::core::config::Config;
use crate::ui::viewer::{self, ViewerState, PreviewCanvas};
use std::path::PathBuf;
use std::fs;
use ::image::GenericImageView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortAction {
    Unsorted,
    Keep,
    Discard,
}

#[derive(Debug, Clone)]
pub struct SortItem {
    pub path: PathBuf,
    pub filename: String,
    pub file_size_bytes: u64,
    pub dimensions: (u32, u32),
    pub thumbnail: iced::widget::image::Handle,
    pub action: SortAction,
}

pub struct State {
    status: String,
    items: Vec<SortItem>,
    selected_index: Option<usize>,
    to_sort_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
    panes: pane_grid::State<PaneState>,
    is_loading: bool,
    preview_viewer: ViewerState,
    preview_is_fit: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum PaneState {
    Grid,
    Preview,
}

#[derive(Debug, Clone)]
pub enum Message {
    Resized(pane_grid::ResizeEvent),
    Refreshed(Vec<SortItem>),
    Select(usize),
    SetAction(SortAction),
    ApplyChanges,
    ApplyComplete(()),
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ViewerMessage(viewer::Message),
}

impl State {
    pub fn new(config: &Config) -> Self {
        let (mut panes, grid_pane) = pane_grid::State::new(PaneState::Grid);
        let (_preview_pane, _) = panes.split(
            pane_grid::Axis::Vertical,
            grid_pane,
            PaneState::Preview,
        ).unwrap();

        Self {
            status: "No images to sort".to_string(),
            items: Vec::new(),
            selected_index: None,
            to_sort_dir: config.to_sort_dir.clone(),
            library_dir: config.library_dir.clone(),
            panes,
            is_loading: false,
            preview_viewer: ViewerState::default(),
            preview_is_fit: true,
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
            self.is_loading = true;
            self.status = "Generating thumbnails...".to_string();
            
            Task::perform(
                async move {
                    let mut found = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        let mut paths = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    let ext = ext.to_lowercase();
                                    if ["jpg", "jpeg", "png", "bmp", "tiff", "webp"].contains(&ext.as_str()) {
                                        paths.push(path);
                                    }
                                }
                            }
                        }
                        paths.sort();
                        
                        for path in paths {
                            let metadata = fs::metadata(&path).ok();
                            let file_size_bytes = metadata.map(|m| m.len()).unwrap_or(0);
                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            
                            if let Ok(img) = ::image::open(&path) {
                                let orig_dimensions = img.dimensions();
                                let thumb = img.thumbnail(256, 256);
                                let rgba = thumb.to_rgba8();
                                let dimensions = rgba.dimensions();
                                let handle = iced::widget::image::Handle::from_rgba(
                                    dimensions.0, 
                                    dimensions.1, 
                                    rgba.into_raw()
                                );
                                found.push(SortItem {
                                    path,
                                    filename,
                                    file_size_bytes,
                                    dimensions: orig_dimensions,
                                    thumbnail: handle,
                                    action: SortAction::Unsorted,
                                });
                            }
                        }
                    }
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
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                Task::none()
            }
            Message::Refreshed(items) => {
                self.items = items;
                self.selected_index = if self.items.is_empty() { None } else { Some(0) };
                self.is_loading = false;
                self.update_status();
                Task::none()
            }
            Message::Select(index) => {
                self.selected_index = Some(index);
                self.preview_is_fit = true;
                self.preview_viewer = ViewerState::default();
                Task::none()
            }
            Message::ZoomIn => {
                let bounds = self.preview_viewer.bounds.lock().unwrap().unwrap_or(iced::Size::new(100.0, 100.0));
                let center = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
                self.apply_zoom(1.2, center);
                Task::none()
            }
            Message::ZoomOut => {
                let bounds = self.preview_viewer.bounds.lock().unwrap().unwrap_or(iced::Size::new(100.0, 100.0));
                let center = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
                self.apply_zoom(0.8, center);
                Task::none()
            }
            Message::ResetZoom => {
                self.preview_is_fit = true;
                self.preview_viewer = ViewerState::default();
                Task::none()
            }
            Message::ViewerMessage(msg) => {
                match msg {
                    viewer::Message::Zoomed(multiplier, cursor) => {
                        self.apply_zoom(multiplier, cursor);
                    }
                    viewer::Message::Dragged(pos) => {
                        if let Some(last) = self.preview_viewer.last_cursor {
                            self.preview_viewer.offset.x += pos.x - last.x;
                            self.preview_viewer.offset.y += pos.y - last.y;
                        }
                        self.preview_viewer.last_cursor = Some(pos);
                    }
                    viewer::Message::DragStarted(pos) => {
                        if self.preview_is_fit {
                            self.preview_is_fit = false;
                            if let Ok(z) = self.preview_viewer.rendered_zoom.lock() { self.preview_viewer.zoom = *z; }
                            if let Ok(o) = self.preview_viewer.rendered_offset.lock() { self.preview_viewer.offset = *o; }
                        }
                        self.preview_viewer.is_dragging = true;
                        self.preview_viewer.last_cursor = Some(pos);
                    }
                    viewer::Message::DragEnded => {
                        self.preview_viewer.is_dragging = false;
                        self.preview_viewer.last_cursor = None;
                    }
                }
                Task::none()
            }
            Message::SetAction(action) => {
                if let Some(idx) = self.selected_index {
                    self.items[idx].action = action;
                }
                Task::none()
            }
            Message::ApplyChanges => {
                if let Some(library) = &self.library_dir {
                    self.is_loading = true;
                    self.status = "Applying changes...".to_string();
                    
                    let items_to_process = self.items.clone();
                    let library = library.clone();
                    
                    Task::perform(
                        async move {
                            for item in items_to_process {
                                match item.action {
                                    SortAction::Keep => {
                                        let target = library.join(item.path.file_name().unwrap());
                                        let _ = std::fs::create_dir_all(target.parent().unwrap());
                                        if fs::rename(&item.path, &target).is_err() {
                                            if fs::copy(&item.path, &target).is_ok() {
                                                let _ = fs::remove_file(&item.path);
                                            }
                                        }
                                    }
                                    SortAction::Discard => {
                                        let _ = fs::remove_file(&item.path);
                                    }
                                    SortAction::Unsorted => {}
                                }
                            }
                        },
                        |_| Message::ApplyComplete(())
                    )
                } else {
                    self.status = "Library directory not configured!".to_string();
                    Task::none()
                }
            }
            Message::ApplyComplete(()) => {
                self.items.retain(|item| item.action == SortAction::Unsorted);
                self.selected_index = if self.items.is_empty() { None } else { Some(0) };
                self.is_loading = false;
                self.update_status();
                Task::none()
            }
        }
    }

    fn apply_zoom(&mut self, multiplier: f32, center: iced::Point) {
        if self.preview_is_fit {
            self.preview_is_fit = false;
            if let Ok(z) = self.preview_viewer.rendered_zoom.lock() { self.preview_viewer.zoom = *z; }
            if let Ok(o) = self.preview_viewer.rendered_offset.lock() { self.preview_viewer.offset = *o; }
        }
        
        let old_zoom = self.preview_viewer.zoom;
        let new_zoom = old_zoom * multiplier;
        
        let cursor_in_image_x = (center.x - self.preview_viewer.offset.x) / old_zoom;
        let cursor_in_image_y = (center.y - self.preview_viewer.offset.y) / old_zoom;
        
        self.preview_viewer.offset.x = center.x - cursor_in_image_x * new_zoom;
        self.preview_viewer.offset.y = center.y - cursor_in_image_y * new_zoom;
        
        self.preview_viewer.zoom = new_zoom;
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.preview_viewer.is_dragging {
            iced::event::listen_with(|event, _status, _window| {
                match event {
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                        Some(Message::ViewerMessage(viewer::Message::Dragged(position)))
                    }
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                        Some(Message::ViewerMessage(viewer::Message::DragEnded))
                    }
                    _ => None,
                }
            })
        } else {
            Subscription::none()
        }
    }

    fn update_status(&mut self) {
        if self.items.is_empty() {
            self.status = "All caught up!".to_string();
        } else {
            let unsorted = self.items.iter().filter(|i| i.action == SortAction::Unsorted).count();
            let keep = self.items.iter().filter(|i| i.action == SortAction::Keep).count();
            let discard = self.items.iter().filter(|i| i.action == SortAction::Discard).count();
            self.status = format!("Remaining: {} | Keeping: {} | Discarding: {}", unsorted, keep, discard);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.panes, |_pane, state, _| {
            match state {
                PaneState::Grid => {
                    let mut wrap = iced_aw::Wrap::new()
                        .spacing(10.0)
                        .line_spacing(10.0);
                    
                    for (idx, item) in self.items.iter().enumerate() {
                        let is_selected = self.selected_index == Some(idx);
                        
                        let img = image(item.thumbnail.clone())
                            .width(Length::Fixed(150.0))
                            .height(Length::Fixed(150.0));
                            
                        let circle_color = match item.action {
                            SortAction::Keep => Some(iced::Color::from_rgb(0.0, 1.0, 0.0)),
                            SortAction::Discard => Some(iced::Color::from_rgb(1.0, 0.0, 0.0)),
                            SortAction::Unsorted => None,
                        };
                        
                        // We use a Column if stack is not available, but let's try iced::widget::stack
                        // Actually, Iced 0.14 added `Stack` widget. Let's try it.
                        let mut stack_children = vec![img.into()];
                        
                        if let Some(color) = circle_color {
                            let indicator = container(iced::widget::Space::new().width(20.0).height(20.0))
                                .style(move |_theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(color)),
                                    border: iced::Border {
                                        radius: 0.0.into(),
                                        ..iced::Border::default()
                                    },
                                    ..iced::widget::container::Style::default()
                                });
                                
                            let overlay = container(indicator)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(iced::alignment::Horizontal::Right)
                                .align_y(iced::alignment::Vertical::Bottom)
                                .padding(5);
                                
                            stack_children.push(overlay.into());
                        }
                        
                        // Fallback to manual layout if stack fails, but let's try `stack` macro or `Stack::with_children`
                        // Actually `stack!` might not be imported or available. Let's use a standard wrapper for now to test.
                        // I will use `iced::widget::stack` function if it exists.
                        // For safety, let's use `iced::widget::stack(stack_children)`
                        let content_stack = iced::widget::stack(stack_children)
                            .width(Length::Fixed(150.0))
                            .height(Length::Fixed(150.0));
                        
                        let border_color = if is_selected {
                            iced::Color::from_rgb(0.0, 0.5, 1.0)
                        } else {
                            iced::Color::TRANSPARENT
                        };
                        
                        let styled_container = container(content_stack)
                            .padding(2)
                            .style(move |_theme| iced::widget::container::Style {
                                border: iced::Border {
                                    color: border_color,
                                    width: if is_selected { 3.0 } else { 0.0 },
                                    radius: 0.0.into(),
                                },
                                ..iced::widget::container::Style::default()
                            });
                        
                        let content = button(styled_container)
                            .padding(0)
                            // Button styling in iced 0.14 for transparent background:
                            .style(iced::widget::button::text)
                            .on_press(Message::Select(idx));
                        
                        wrap = wrap.push(content);
                    }
                    
                    let grid_view = scrollable(container(wrap).padding(10))
                        .width(Length::Fill)
                        .height(Length::Fill);
                        
                    pane_grid::Content::new(grid_view)
                }
                PaneState::Preview => {
                    let mut preview_col = column![].spacing(10).align_x(Alignment::Center);
                    
                    if let Some(idx) = self.selected_index {
                        if idx < self.items.len() {
                            let item = &self.items[idx];
                            
                            // File Details Header
                            let details = column![
                                text(format!("File: {}", item.filename)).size(16),
                                text(format!("Size: {:.2} MB", item.file_size_bytes as f64 / 1_048_576.0)).size(14),
                                text(format!("Dimensions: {} x {}", item.dimensions.0, item.dimensions.1)).size(14),
                            ].spacing(5).align_x(Alignment::Center);
                            
                            preview_col = preview_col.push(details);
                            
                            let canvas_widget = canvas(PreviewCanvas {
                                handle: image::Handle::from_path(item.path.clone()),
                                dimensions: item.dimensions,
                                state: &self.preview_viewer,
                                is_fit: self.preview_is_fit,
                            })
                            .width(Length::Fill)
                            .height(Length::FillPortion(5));
                            
                            let mapped_canvas = Element::from(canvas_widget).map(Message::ViewerMessage);
                            preview_col = preview_col.push(mapped_canvas);
                            
                            // Controls
                            let zoom_controls = row![
                                button("Zoom Out (-)").on_press(Message::ZoomOut),
                                button("Reset").on_press(Message::ResetZoom),
                                button("Zoom In (+)").on_press(Message::ZoomIn),
                            ].spacing(10);
                            
                            let actions = row![
                                button("Discard").on_press(Message::SetAction(SortAction::Discard)),
                                button("Unsorted").on_press(Message::SetAction(SortAction::Unsorted)),
                                button("Keep").on_press(Message::SetAction(SortAction::Keep)),
                            ].spacing(20);
                            
                            preview_col = preview_col.push(zoom_controls);
                            preview_col = preview_col.push(actions);
                        }
                    } else {
                        preview_col = preview_col.push(text("No image selected"));
                    }
                    
                    pane_grid::Content::new(container(preview_col).padding(10).width(Length::Fill).height(Length::Fill))
                }
            }
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .on_resize(10.0, Message::Resized);

        let mut main_col = column![
            text("Sort Photos").size(24),
            text(&self.status),
        ].spacing(10);
        
        if self.is_loading {
            main_col = main_col.push(text("Processing... please wait."));
        } else {
            main_col = main_col.push(
                container(pane_grid)
                    .width(Length::Fill)
                    .height(Length::FillPortion(5))
            );
            
            if !self.items.is_empty() {
                main_col = main_col.push(
                    container(button("Apply Changes").on_press(Message::ApplyChanges))
                        .width(Length::Fill)
                        .padding(10)
                        .align_x(Alignment::Center)
                );
            }
        }

        main_col.into()
    }
}
