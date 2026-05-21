use iced::widget::{button, column, row, text, image, container, scrollable, canvas};
use iced::{Element, Task, Length, Alignment, Subscription};
use crate::core::config::Config;
use crate::ui::viewer::{self, ViewerState, PreviewCanvas};
use crate::ui::theme::{brutalist_button_style, brutalist_light_button_style, bold_font, brutalist_card_style, brutalist_card_shadow_style};
use std::path::PathBuf;
use std::fs;
use exif::{In, Tag, Reader};
use tokio::sync::mpsc::UnboundedReceiver;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

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
    pub thumbnail: Option<iced::widget::image::Handle>,
    pub action: SortAction,
}

struct ThumbnailRequest {
    idx: usize,
    path: PathBuf,
    filename: String,
    cache_dir: PathBuf,
}

pub struct State {
    status: String,
    items: Vec<SortItem>,
    selected_index: Option<usize>,
    to_sort_dir: Option<PathBuf>,
    library_dir: Option<PathBuf>,
    is_loading: bool,
    preview_viewer: ViewerState,
    preview_is_fit: bool,
    spinner_tick: usize,
    preview_loading: bool,
    preview_handle: Option<iced::widget::image::Handle>,
    thumbnail_sender: std::sync::mpsc::Sender<ThumbnailRequest>,
    result_rx: Arc<TokioMutex<UnboundedReceiver<Message>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refreshed(Vec<(PathBuf, String, u64)>),
    Select(usize),
    SetAction(SortAction),
    ApplyChanges,
    ApplyComplete(()),
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ViewerMessage(viewer::Message),
    RefreshThumbnails,
    ThumbnailLoaded { idx: usize, path: PathBuf, dimensions: (u32, u32), handle: Option<iced::widget::image::Handle> },
    ThumbnailsLoaded(Vec<(usize, PathBuf, (u32, u32), Option<iced::widget::image::Handle>)>),
    PreviewLoaded { idx: usize, handle: Option<iced::widget::image::Handle>, dimensions: (u32, u32) },
    Tick,
    PrevImage,
    NextImage,
    ToggleKeep,
    ToggleDiscard,
}

fn make_placeholder_thumbnail() -> iced::widget::image::Handle {
    let width = 256u32;
    let height = 256u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = 100;     // R
        pixels[i + 1] = 110; // G
        pixels[i + 2] = 120; // B
        pixels[i + 3] = 255; // A
    }
    iced::widget::image::Handle::from_rgba(width, height, pixels)
}

fn get_exif_orientation(path: &std::path::Path) -> u32 {
    if let Ok(file) = std::fs::File::open(path) {
        let mut bufreader = std::io::BufReader::new(file);
        let exifreader = Reader::new();
        if let Ok(exif) = exifreader.read_from_container(&mut bufreader) {
            if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                return field.value.get_uint(0).unwrap_or(1);
            }
        }
    }
    1
}

fn apply_exif_orientation(img: ::image::DynamicImage, orientation: u32) -> ::image::DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.fliph().rotate270(),
        6 => img.rotate90(),
        7 => img.fliph().rotate90(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn generate_with_sips(src_path: &std::path::Path, dest_path: &std::path::Path, max_dimension: Option<u32>) -> bool {
    let mut cmd = std::process::Command::new("sips");
    cmd.arg("-s")
       .arg("format")
       .arg("jpeg");
       
    if let Some(dim) = max_dimension {
        cmd.arg("-Z")
           .arg(dim.to_string());
    }
    
    let output = cmd.arg(src_path)
        .arg("--out")
        .arg(dest_path)
        .output();
        
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}


fn ensure_raw_thumbnail(path: &std::path::Path, thumb_path: &std::path::Path) -> ((u32, u32), Option<iced::widget::image::Handle>) {
    let mut dims = (0, 0);
    
    if let Ok(file) = std::fs::File::open(path) {
        let mut bufreader = std::io::BufReader::new(file);
        let exifreader = Reader::new();
        if let Ok(exif) = exifreader.read_from_container(&mut bufreader) {
            // 1. Get raw dimensions
            let width = exif.get_field(Tag::PixelXDimension, In::PRIMARY)
                .or_else(|| exif.get_field(Tag::ImageWidth, In::PRIMARY))
                .and_then(|f| f.value.get_uint(0));
                
            let height = exif.get_field(Tag::PixelYDimension, In::PRIMARY)
                .or_else(|| exif.get_field(Tag::ImageLength, In::PRIMARY))
                .and_then(|f| f.value.get_uint(0));
                
            let orientation = exif.get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .unwrap_or(1);
                
            let is_portrait = orientation == 5 || orientation == 6 || orientation == 7 || orientation == 8;
                
            if let (Some(w), Some(h)) = (width, height) {
                if is_portrait {
                    dims = (h, w);
                } else {
                    dims = (w, h);
                }
            }
        }
    }
    
    // Always generate thumbnail using sips if not present
    if !thumb_path.exists() {
        if let Some(parent) = thumb_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = generate_with_sips(path, thumb_path, Some(256));
    }
    
    let mut handle = None;
    let mut decoded_dims = (0, 0);
    
    if thumb_path.exists() {
        if let Ok(img) = ::image::open(thumb_path) {
            let orientation = get_exif_orientation(thumb_path);
            let img = apply_exif_orientation(img, orientation);
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            decoded_dims = (w, h);
            handle = Some(iced::widget::image::Handle::from_rgba(w, h, rgba.into_raw()));
        } else {
            handle = Some(iced::widget::image::Handle::from_path(thumb_path));
        }
    }
    
    let final_dims = if dims == (0, 0) {
        if decoded_dims != (0, 0) {
            decoded_dims
        } else if thumb_path.exists() {
            ::image::image_dimensions(thumb_path).unwrap_or((0, 0))
        } else {
            (0, 0)
        }
    } else {
        dims
    };
    
    (final_dims, handle)
}

fn ensure_raw_preview(path: &std::path::Path, preview_path: &std::path::Path) -> bool {
    if preview_path.exists() {
        return true;
    }
    if let Some(parent) = preview_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    generate_with_sips(path, preview_path, None)
}

fn get_spinner_char(tick: usize) -> &'static str {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    frames[tick % frames.len()]
}


#[derive(Clone)]
struct HashableArc<T>(Arc<T>);

impl<T> std::hash::Hash for HashableArc<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}
impl<T> PartialEq for HashableArc<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T> Eq for HashableArc<T> {}

fn make_stream(rx: &HashableArc<TokioMutex<UnboundedReceiver<Message>>>) -> futures::stream::BoxStream<'static, Message> {
    use futures::StreamExt;
    let rx_clone = rx.0.clone();
    futures::stream::unfold(rx_clone, |rx| async move {
        let mut rx_guard = rx.lock().await;
        let first_msg = rx_guard.recv().await;
        
        if let Some(msg) = first_msg {
            match msg {
                Message::ThumbnailLoaded { idx, path, dimensions, handle } => {
                    // Sleep for 30ms to allow more thumbnails to accumulate in the channel buffer
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    
                    let mut batch = vec![(idx, path, dimensions, handle)];
                    while let Ok(next_msg) = rx_guard.try_recv() {
                        match next_msg {
                            Message::ThumbnailLoaded { idx, path, dimensions, handle } => {
                                batch.push((idx, path, dimensions, handle));
                            }
                            _ => {}
                        }
                    }
                    drop(rx_guard);
                    Some((Message::ThumbnailsLoaded(batch), rx))
                }
                other => {
                    drop(rx_guard);
                    Some((other, rx))
                }
            }
        } else {
            drop(rx_guard);
            None
        }
    }).boxed()
}


impl State {
    pub fn new(config: &Config) -> Self {

        let (tx, rx) = std::sync::mpsc::channel::<ThumbnailRequest>();
        let rx = std::sync::Arc::new(std::sync::Mutex::new(rx));
        
        let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
        let result_rx = std::sync::Arc::new(tokio::sync::Mutex::new(result_rx));
        
        for i in 0..2 {
            let rx_clone = rx.clone();
            let result_tx_clone = result_tx.clone();
            std::thread::spawn(move || {
                loop {
                    let req = {
                        let lock = match rx_clone.lock() {
                            Ok(guard) => guard,
                            Err(_) => break,
                        };
                        match lock.recv() {
                            Ok(req) => req,
                            Err(_) => break,
                        }
                    };
                    
                    println!("[Worker {}] Started processing item {} ({})", i, req.idx, req.filename);
                    
                    let ext = req.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    let is_raw = ["raw", "cr2", "nef", "arw"].contains(&ext.as_str());
                    
                    let thumb_path = req.cache_dir.join(format!("{}.jpg", req.filename));
                    
                    let (dimensions, mut handle) = if is_raw {
                        ensure_raw_thumbnail(&req.path, &thumb_path)
                    } else {
                        if !thumb_path.exists() {
                            let sips_success = generate_with_sips(&req.path, &thumb_path, Some(256));
                            if !sips_success {
                                if let Ok(img) = ::image::open(&req.path) {
                                    let orientation = get_exif_orientation(&req.path);
                                    let img = apply_exif_orientation(img, orientation);
                                    let thumb = img.thumbnail(256, 256);
                                    let _ = thumb.save(&thumb_path);
                                }
                            }
                        }
                        let mut final_handle = None;
                        let mut final_dims = (0, 0);
                        if thumb_path.exists() {
                            if let Ok(img) = ::image::open(&thumb_path) {
                                let orientation = get_exif_orientation(&thumb_path);
                                let img = apply_exif_orientation(img, orientation);
                                let rgba = img.to_rgba8();
                                let (w, h) = rgba.dimensions();
                                final_dims = (w, h);
                                final_handle = Some(iced::widget::image::Handle::from_rgba(w, h, rgba.into_raw()));
                            } else {
                                final_handle = Some(iced::widget::image::Handle::from_path(&thumb_path));
                            }
                        }
                        (final_dims, final_handle)
                    };
                    
                    if handle.is_none() {
                        handle = Some(make_placeholder_thumbnail());
                    }
                    
                    println!("[Worker {}] Finished processing item {} ({})", i, req.idx, req.filename);
                    let _ = result_tx_clone.send(Message::ThumbnailLoaded {
                        idx: req.idx,
                        path: req.path,
                        dimensions,
                        handle,
                    });
                    
                    // Yield a little bit to prevent CPU starvation of the main thread
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            });
        }

        Self {
            status: "No images to sort".to_string(),
            items: Vec::new(),
            selected_index: None,
            to_sort_dir: config.to_sort_dir.clone(),
            library_dir: config.library_dir.clone(),
            is_loading: false,
            preview_viewer: ViewerState::default(),
            preview_is_fit: true,
            spinner_tick: 0,
            preview_loading: false,
            preview_handle: None,
            thumbnail_sender: tx,
            result_rx,
        }
    }

    pub fn update_config(&mut self, config: &Config) {
        self.to_sort_dir = config.to_sort_dir.clone();
        self.library_dir = config.library_dir.clone();
    }

    fn trigger_preview_load(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.items.len() {
            self.preview_loading = false;
            self.preview_handle = None;
            return Task::none();
        }
        
        let item = &self.items[idx];
        let ext = item.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let is_raw = ["raw", "cr2", "nef", "arw"].contains(&ext.as_str());
        
        self.preview_loading = true;
        self.preview_handle = None;
        
        let path = item.path.clone();
        let to_sort_dir = self.to_sort_dir.clone();
        let filename = item.filename.clone();
        
        Task::perform(
            async move {
                let (tx, rx) = futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let mut loaded_handle = None;
                    let mut loaded_dims = (0, 0);
                    
                    if is_raw {
                        if let Some(to_sort) = to_sort_dir {
                            let preview_path = to_sort.join(".thumbnail_cache").join(format!("{}_preview.jpg", filename));
                            let thumb_path = to_sort.join(".thumbnail_cache").join(format!("{}.jpg", filename));
                            
                            let success = ensure_raw_preview(&path, &preview_path);
                            let target_path = if success && preview_path.exists() {
                                Some(preview_path)
                            } else if thumb_path.exists() {
                                Some(thumb_path)
                            } else {
                                None
                            };
                            
                            if let Some(t_path) = target_path {
                                if let Ok(img) = ::image::open(&t_path) {
                                    let orientation = get_exif_orientation(&t_path);
                                    let img = apply_exif_orientation(img, orientation);
                                    let img = img.to_rgba8();
                                    loaded_dims = img.dimensions();
                                    loaded_handle = Some(iced::widget::image::Handle::from_rgba(
                                        loaded_dims.0,
                                        loaded_dims.1,
                                        img.into_raw(),
                                    ));
                                }
                            }
                        }
                    } else {
                        if let Ok(img) = ::image::open(&path) {
                            let orientation = get_exif_orientation(&path);
                            let img = apply_exif_orientation(img, orientation);
                            let img = img.to_rgba8();
                            loaded_dims = img.dimensions();
                            loaded_handle = Some(iced::widget::image::Handle::from_rgba(
                                loaded_dims.0,
                                loaded_dims.1,
                                img.into_raw(),
                            ));
                        }
                    }
                    let _ = tx.send((idx, loaded_handle, loaded_dims));
                });
                rx.await.unwrap_or_else(|_| (idx, None, (0, 0)))
            },
            |(idx, handle, dimensions)| Message::PreviewLoaded { idx, handle, dimensions }
        )
    }

    pub fn refresh(&mut self, config: &Config) -> Task<Message> {
        self.update_config(config);
        
        if let Some(to_sort) = &self.to_sort_dir {
            let dir = to_sort.clone();
            self.status = "Scanning photos...".to_string();
            
            Task::perform(
                async move {
                    let mut found = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        let mut paths = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.components().any(|c| c.as_os_str() == ".thumbnail_cache") {
                                continue;
                            }
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                    let ext = ext.to_lowercase();
                                    if ["jpg", "jpeg", "png", "bmp", "tiff", "webp", "raw", "cr2", "nef", "arw"].contains(&ext.as_str()) {
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
                            found.push((path, filename, file_size_bytes));
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
            Message::RefreshThumbnails => {
                if let Some(to_sort) = &self.to_sort_dir {
                    let dir = to_sort.clone();
                    self.status = "Regenerating thumbnail cache...".to_string();
                    
                    // Clear all thumbnails immediately in UI to show spinners
                    for item in &mut self.items {
                        item.thumbnail = None;
                    }
                    
                    // Grab current items' metadata to pass to Refreshed once cache is cleared
                    let current_metadata: Vec<(PathBuf, String, u64)> = self.items.iter()
                        .map(|item| (item.path.clone(), item.filename.clone(), item.file_size_bytes))
                        .collect();
                    
                    Task::perform(
                        async move {
                            let cache_dir = dir.join(".thumbnail_cache");
                            let _ = std::fs::remove_dir_all(&cache_dir);
                            let _ = std::fs::create_dir_all(&cache_dir);
                            current_metadata
                        },
                        Message::Refreshed
                    )
                } else {
                    self.status = "To Sort directory not configured.".to_string();
                    Task::none()
                }
            }
            Message::Refreshed(items_meta) => {
                println!("[Main] Refreshed received with {} items", items_meta.len());
                self.items = items_meta.into_iter().map(|(path, filename, file_size_bytes)| {
                    SortItem {
                        path,
                        filename,
                        file_size_bytes,
                        dimensions: (0, 0),
                        thumbnail: None,
                        action: SortAction::Unsorted,
                    }
                }).collect();
                self.selected_index = if self.items.is_empty() { None } else { Some(0) };
                self.is_loading = false;
                self.update_status();
                
                if let Some(to_sort) = &self.to_sort_dir {
                    let cache_dir = to_sort.join(".thumbnail_cache");
                    let _ = std::fs::create_dir_all(&cache_dir);
                    
                    println!("[Main] Queuing {} thumbnail generation requests", self.items.len());
                    for (idx, item) in self.items.iter().enumerate() {
                        let req = ThumbnailRequest {
                            idx,
                            path: item.path.clone(),
                            filename: item.filename.clone(),
                            cache_dir: cache_dir.clone(),
                        };
                        let _ = self.thumbnail_sender.send(req);
                    }
                }
                
                if let Some(idx) = self.selected_index {
                    self.trigger_preview_load(idx)
                } else {
                    Task::none()
                }
            }
            Message::ThumbnailLoaded { idx, path, dimensions, handle } => {
                println!("[Main] ThumbnailLoaded received for item {} ({:?})", idx, path.file_name());
                if idx < self.items.len() && self.items[idx].path == path {
                    self.items[idx].dimensions = dimensions;
                    self.items[idx].thumbnail = handle;
                }
                Task::none()
            }
            Message::ThumbnailsLoaded(batch) => {
                println!("[Main] ThumbnailsLoaded received batch of {} items", batch.len());
                for (idx, path, dimensions, handle) in batch {
                    if idx < self.items.len() && self.items[idx].path == path {
                        self.items[idx].dimensions = dimensions;
                        self.items[idx].thumbnail = handle;
                    }
                }
                Task::none()
            }
            Message::PreviewLoaded { idx, handle, dimensions } => {
                if let Some(current_idx) = self.selected_index {
                    if current_idx == idx {
                        self.preview_handle = handle;
                        self.preview_loading = false;
                        if idx < self.items.len() && dimensions != (0, 0) {
                            self.items[idx].dimensions = dimensions;
                        }
                    }
                }
                Task::none()
            }
            Message::Select(index) => {
                self.selected_index = Some(index);
                self.preview_is_fit = true;
                self.preview_viewer = ViewerState::default();
                self.preview_handle = None;
                self.trigger_preview_load(index)
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
                    self.update_status();
                }
                Task::none()
            }
            Message::PrevImage => {
                if let Some(idx) = self.selected_index {
                    if idx > 0 {
                        self.selected_index = Some(idx - 1);
                        self.preview_is_fit = true;
                        self.preview_viewer = ViewerState::default();
                        self.preview_handle = None;
                        return self.trigger_preview_load(idx - 1);
                    }
                }
                Task::none()
            }
            Message::NextImage => {
                if let Some(idx) = self.selected_index {
                    if idx + 1 < self.items.len() {
                        self.selected_index = Some(idx + 1);
                        self.preview_is_fit = true;
                        self.preview_viewer = ViewerState::default();
                        self.preview_handle = None;
                        return self.trigger_preview_load(idx + 1);
                    }
                }
                Task::none()
            }
            Message::ToggleKeep => {
                if let Some(idx) = self.selected_index {
                    if self.items[idx].action == SortAction::Keep {
                        self.items[idx].action = SortAction::Unsorted;
                    } else {
                        self.items[idx].action = SortAction::Keep;
                    }
                    self.update_status();
                }
                Task::none()
            }
            Message::ToggleDiscard => {
                if let Some(idx) = self.selected_index {
                    if self.items[idx].action == SortAction::Discard {
                        self.items[idx].action = SortAction::Unsorted;
                    } else {
                        self.items[idx].action = SortAction::Discard;
                    }
                    self.update_status();
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
                                let mut has_moved_or_deleted = false;
                                match item.action {
                                    SortAction::Keep => {
                                        let target = library.join(item.path.file_name().unwrap());
                                        let _ = std::fs::create_dir_all(target.parent().unwrap());
                                        if fs::rename(&item.path, &target).is_err() {
                                            if fs::copy(&item.path, &target).is_ok() {
                                                let _ = fs::remove_file(&item.path);
                                                has_moved_or_deleted = true;
                                            }
                                        } else {
                                            has_moved_or_deleted = true;
                                        }
                                    }
                                    SortAction::Discard => {
                                        if fs::remove_file(&item.path).is_ok() {
                                            has_moved_or_deleted = true;
                                        }
                                    }
                                    SortAction::Unsorted => {}
                                }
                                if has_moved_or_deleted {
                                    if let Some(parent) = item.path.parent() {
                                        let thumb_path = parent.join(".thumbnail_cache").join(format!("{}.jpg", item.filename));
                                        let _ = fs::remove_file(thumb_path);
                                    }
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
            Message::Tick => {
                self.spinner_tick = self.spinner_tick.wrapping_add(1);
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
        let drag_sub = if self.preview_viewer.is_dragging {
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
        };

        let keyboard_sub = iced::event::listen_with(|event, _status, _window| {
            match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
                    match key {
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                            Some(Message::PrevImage)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                            Some(Message::NextImage)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            Some(Message::ToggleKeep)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            Some(Message::ToggleDiscard)
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        });

        let rx_hashable = HashableArc(self.result_rx.clone());
        let result_sub = Subscription::run_with(
            rx_hashable,
            make_stream,
        );

        let needs_tick = self.preview_loading || self.items.iter().any(|item| item.thumbnail.is_none());
        
        let tick_sub = if needs_tick {
            iced::time::every(std::time::Duration::from_millis(150))
                .map(|_| Message::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch(vec![drag_sub, keyboard_sub, result_sub, tick_sub])
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
        let mut header = row![
            text("SORT PHOTOS").font(bold_font()).size(28),
        ].spacing(10).align_y(Alignment::Center);
        
        if !self.is_loading {
            header = header.push(iced::widget::Space::new().width(Length::Fill));
            header = header.push(
                button(text("REFRESH THUMBNAILS").font(bold_font()))
                    .on_press(Message::RefreshThumbnails)
                    .padding(8)
                    .style(brutalist_button_style)
            );
        }

        let mut main_col = column![
            header,
            text(self.status.to_uppercase()).font(bold_font()).size(14),
        ].spacing(15);
        
        if self.is_loading {
            let spinner = get_spinner_char(self.spinner_tick);
            let loader = container(
                column![
                    text(spinner).size(48).font(bold_font()),
                    text("PROCESSING... PLEASE WAIT.").font(bold_font()).size(14),
                ]
                .spacing(15)
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);
            
            main_col = main_col.push(loader);
        } else {
            // Left Side: Preview Area
            let preview_area: Element<'_, Message> = if let Some(idx) = self.selected_index {
                if idx < self.items.len() {
                    let item = &self.items[idx];
                    if self.preview_loading {
                        let ext = item.path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        let is_raw = ["raw", "cr2", "nef", "arw"].contains(&ext.as_str());
                        let needs_dev = is_raw && self.to_sort_dir.as_ref().map(|to_sort| {
                            !to_sort.join(".thumbnail_cache").join(format!("{}_preview.jpg", item.filename)).exists()
                        }).unwrap_or(true);
                        
                        let spinner_text = if needs_dev {
                            "DEVELOPING RAW IMAGE..."
                        } else {
                            "LOADING PREVIEW..."
                        };
                        
                        let spinner = get_spinner_char(self.spinner_tick);
                        container(
                            column![
                                text(spinner).size(48).font(bold_font()),
                                text(spinner_text).size(14).font(bold_font()),
                            ]
                            .spacing(15)
                            .align_x(Alignment::Center)
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(brutalist_card_style)
                        .into()
                    } else {
                        let preview_handle = self.preview_handle.clone().unwrap_or_else(make_placeholder_thumbnail);
                        let preview_dimensions = if item.dimensions == (0, 0) {
                            (256, 256)
                        } else {
                            item.dimensions
                        };
                        
                        let canvas_widget = canvas(PreviewCanvas {
                            handle: preview_handle,
                            dimensions: preview_dimensions,
                            state: &self.preview_viewer,
                            is_fit: self.preview_is_fit,
                        })
                        .width(Length::Fill)
                        .height(Length::Fill);
                        
                        Element::from(canvas_widget).map(Message::ViewerMessage)
                    }
                } else {
                    container(text("No image selected").size(16).font(bold_font()))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(brutalist_card_style)
                        .into()
                }
            } else {
                container(text("No images to sort").size(16).font(bold_font()))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(brutalist_card_style)
                    .into()
            };

            // Right Side: Control Panel
            let control_panel: Element<'_, Message> = if let Some(idx) = self.selected_index {
                if idx < self.items.len() {
                    let item = &self.items[idx];
                    
                    let inner_details = container(
                        column![
                            text(format!("FILE: {}", item.filename.to_uppercase()))
                                .size(14)
                                .font(bold_font()),
                            text(format!("SIZE: {:.2} MB", item.file_size_bytes as f64 / 1_048_576.0))
                                .size(12)
                                .font(bold_font()),
                            text(format!("DIMENSIONS: {} X {}", item.dimensions.0, item.dimensions.1))
                                .size(12)
                                .font(bold_font()),
                        ]
                        .spacing(6)
                        .align_x(Alignment::Start)
                    )
                    .padding(12)
                    .width(Length::Fill)
                    .style(brutalist_card_style);
                    
                    let details_card = container(inner_details)
                        .width(Length::Fill)
                        .padding(iced::Padding {
                            top: 0.0,
                            left: 0.0,
                            bottom: 6.0,
                            right: 6.0,
                        })
                        .style(brutalist_card_shadow_style);
                        
                    let zoom_controls = row![
                        button(text("ZOOM OUT (-)").font(bold_font()))
                            .padding(8)
                            .style(brutalist_button_style)
                            .on_press(Message::ZoomOut),
                        button(text("RESET").font(bold_font()))
                            .padding(8)
                            .style(brutalist_button_style)
                            .on_press(Message::ResetZoom),
                        button(text("ZOOM IN (+)").font(bold_font()))
                            .padding(8)
                            .style(brutalist_button_style)
                            .on_press(Message::ZoomIn),
                    ].spacing(10);
                    
                    let actions = row![
                        button(text("DISCARD").font(bold_font()))
                            .padding(10)
                            .style(brutalist_button_style)
                            .on_press(Message::SetAction(SortAction::Discard)),
                        button(text("UNSORTED").font(bold_font()))
                            .padding(10)
                            .style(brutalist_button_style)
                            .on_press(Message::SetAction(SortAction::Unsorted)),
                        button(text("KEEP").font(bold_font()))
                            .padding(10)
                            .style(brutalist_light_button_style)
                            .on_press(Message::SetAction(SortAction::Keep)),
                    ].spacing(20);
                    
                    let panel_col = column![
                        details_card,
                        text("ZOOM PREVIEW").font(bold_font()).size(12),
                        zoom_controls,
                        iced::widget::Space::new().height(10.0),
                        text("SORT ACTION").font(bold_font()).size(12),
                        actions,
                        iced::widget::Space::new().height(Length::Fill),
                        button(text("APPLY CHANGES").font(bold_font()))
                            .on_press(Message::ApplyChanges)
                            .padding(iced::Padding {
                                top: 12.0,
                                bottom: 12.0,
                                left: 24.0,
                                right: 24.0,
                            })
                            .width(Length::Fill)
                            .style(brutalist_light_button_style)
                    ]
                    .spacing(15)
                    .align_x(Alignment::Center);
                    
                    container(panel_col)
                        .width(Length::Fixed(350.0))
                        .height(Length::Fill)
                        .into()
                } else {
                    container(iced::widget::Space::new())
                        .width(Length::Fixed(350.0))
                        .height(Length::Fill)
                        .into()
                }
            } else {
                container(iced::widget::Space::new())
                    .width(Length::Fixed(350.0))
                    .height(Length::Fill)
                    .into()
            };

            let workspace = row![
                preview_area,
                iced::widget::Space::new().width(15.0),
                control_panel,
            ]
            .width(Length::Fill)
            .height(Length::Fill);

            main_col = main_col.push(workspace);

            // Bottom filmstrip of thumbnails
            if !self.items.is_empty() {
                let divider = container(iced::widget::Space::new().height(3.0))
                    .width(Length::Fill)
                    .style(move |theme: &iced::Theme| {
                        let is_dark = theme.palette().background.r < 0.5;
                        let line_color = if is_dark { crate::ui::theme::PAPER_WHITE } else { crate::ui::theme::CHARCOAL_DEEP };
                        iced::widget::container::Style {
                            background: Some(iced::Background::Color(line_color)),
                            ..iced::widget::container::Style::default()
                        }
                    });
                
                main_col = main_col.push(iced::widget::Space::new().height(10.0));
                main_col = main_col.push(divider);
                main_col = main_col.push(iced::widget::Space::new().height(10.0));

                let mut filmstrip = row![].spacing(10);
                
                for (idx, item) in self.items.iter().enumerate() {
                    let is_selected = self.selected_index == Some(idx);
                    
                    let content_stack = if let Some(handle) = &item.thumbnail {
                        let img = image(handle.clone())
                            .width(Length::Fixed(120.0))
                            .height(Length::Fixed(120.0));
                            
                        let (circle_color, text_val) = match item.action {
                            SortAction::Keep => (Some(iced::Color::from_rgb(0.12, 0.8, 0.43)), "K"),
                            SortAction::Discard => (Some(iced::Color::from_rgb(0.9, 0.2, 0.25)), "D"),
                            SortAction::Unsorted => (None, ""),
                        };
                        
                        let mut stack_children = vec![img.into()];
                        
                        if let Some(color) = circle_color {
                            let indicator = container(
                                text(text_val)
                                    .size(9)
                                    .font(bold_font())
                                    .color(iced::Color::BLACK)
                            )
                            .padding(iced::Padding {
                                top: 2.0,
                                bottom: 2.0,
                                left: 6.0,
                                right: 6.0,
                            })
                            .style(move |_theme: &iced::Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(color)),
                                border: iced::Border {
                                    color: iced::Color::BLACK,
                                    width: 1.5,
                                    radius: 0.0.into(),
                                },
                                ..iced::widget::container::Style::default()
                            });
                                
                            let overlay = container(indicator)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(iced::alignment::Horizontal::Right)
                                .align_y(iced::alignment::Vertical::Bottom)
                                .padding(6);
                                
                            stack_children.push(overlay.into());
                        }
                        
                        iced::widget::stack(stack_children)
                            .width(Length::Fixed(120.0))
                            .height(Length::Fixed(120.0))
                    } else {
                        let spinner = get_spinner_char(self.spinner_tick);
                        let placeholder = container(
                            column![
                                text(spinner).size(20).font(bold_font()),
                                text("LOADING...").size(8).font(bold_font()),
                            ]
                            .spacing(6)
                            .align_x(Alignment::Center)
                        )
                        .width(Length::Fixed(120.0))
                        .height(Length::Fixed(120.0))
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center)
                        .style(move |theme: &iced::Theme| {
                            let bg = theme.palette().background;
                            iced::widget::container::Style {
                                background: Some(iced::Background::Color(bg)),
                                border: iced::Border {
                                    color: iced::Color::from_rgb(0.5, 0.5, 0.5),
                                    width: 1.0,
                                    radius: 0.0.into(),
                                },
                                ..iced::widget::container::Style::default()
                            }
                        });
                        
                        iced::widget::stack(vec![placeholder.into()])
                            .width(Length::Fixed(120.0))
                            .height(Length::Fixed(120.0))
                    };
                    
                    let styled_container = container(content_stack)
                        .padding(3)
                        .style(move |theme: &iced::Theme| {
                            let bg = theme.palette().background;
                            let is_dark = theme.palette().background.r < 0.5;
                            
                            let border_color = if is_selected {
                                crate::ui::theme::HOT_PINK
                            } else {
                                if is_dark {
                                    crate::ui::theme::PAPER_WHITE
                                } else {
                                    crate::ui::theme::CHARCOAL_DEEP
                                }
                            };
                            
                            iced::widget::container::Style {
                                background: Some(iced::Background::Color(bg)),
                                border: iced::Border {
                                    color: border_color,
                                    width: 4.0,
                                    radius: 0.0.into(),
                                },
                                ..iced::widget::container::Style::default()
                            }
                        });
                        
                    let content = button(styled_container)
                        .padding(0)
                        .style(iced::widget::button::text)
                        .on_press(Message::Select(idx));
                        
                    filmstrip = filmstrip.push(content);
                }
                
                let filmstrip_scrollable = scrollable(container(filmstrip).padding(5))
                    .direction(scrollable::Direction::Horizontal(Default::default()))
                    .width(Length::Fill)
                    .height(Length::Fixed(150.0));
                
                main_col = main_col.push(filmstrip_scrollable);
            }
        }

        main_col.padding(20).into()
    }
}
