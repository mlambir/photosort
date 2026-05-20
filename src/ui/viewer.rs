use iced::widget::canvas::{Action, Event, Frame, Geometry, Program};
use iced::{mouse, Point, Rectangle, Size, Vector, Theme, Renderer};
use iced::widget::image;
use iced_core::image as core_image;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ViewerState {
    pub zoom: f32,
    pub offset: Vector,
    pub is_dragging: bool,
    pub last_cursor: Option<Point>,
    
    pub rendered_zoom: Arc<Mutex<f32>>,
    pub rendered_offset: Arc<Mutex<Vector>>,
    pub bounds: Arc<Mutex<Option<Size>>>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vector::new(0.0, 0.0),
            is_dragging: false,
            last_cursor: None,
            rendered_zoom: Arc::new(Mutex::new(1.0)),
            rendered_offset: Arc::new(Mutex::new(Vector::new(0.0, 0.0))),
            bounds: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Zoomed(f32, Point),
    Dragged(Point),
    DragStarted(Point),
    DragEnded,
}

pub struct PreviewCanvas<'a> {
    pub handle: image::Handle,
    pub dimensions: (u32, u32),
    pub state: &'a ViewerState,
    pub is_fit: bool,
}

impl<'a> Program<Message, Theme, Renderer> for PreviewCanvas<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let cursor_position_relative = cursor.position_in(_bounds)?;

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 10.0,
                };
                
                let multiplier = if y > 0.0 { 1.1 } else if y < 0.0 { 0.9 } else { 1.0 };
                if multiplier != 1.0 {
                    return Some(Action::publish(Message::Zoomed(multiplier, cursor_position_relative)));
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let absolute = cursor.position()?;
                return Some(Action::publish(Message::DragStarted(absolute)));
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        
        let (img_w, img_h) = (self.dimensions.0 as f32, self.dimensions.1 as f32);
        
        let mut zoom = self.state.zoom;
        let mut offset = self.state.offset;
        
        if self.is_fit {
            let scale_x = bounds.width / img_w;
            let scale_y = bounds.height / img_h;
            zoom = scale_x.min(scale_y);
            
            let scaled_w = img_w * zoom;
            let scaled_h = img_h * zoom;
            offset = Vector::new(
                (bounds.width - scaled_w) / 2.0,
                (bounds.height - scaled_h) / 2.0,
            );
        }
        
        if let Ok(mut r_z) = self.state.rendered_zoom.lock() { *r_z = zoom; }
        if let Ok(mut r_o) = self.state.rendered_offset.lock() { *r_o = offset; }
        if let Ok(mut b) = self.state.bounds.lock() { *b = Some(bounds.size()); }
        
        frame.translate(offset);
        frame.scale(zoom);
        
        frame.draw_image(
            Rectangle::new(Point::ORIGIN, Size::new(img_w, img_h)), 
            core_image::Image {
                handle: self.handle.clone(),
                filter_method: core_image::FilterMethod::Linear,
                rotation: iced_core::Radians(0.0),
                border_radius: iced_core::border::Radius::from(0.0),
                opacity: 1.0,
                snap: false,
            }
        );
        
        vec![frame.into_geometry()]
    }
}
