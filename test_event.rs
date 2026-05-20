fn main() {
    let _x = iced::Event::Window(winit::event::WindowEvent::PinchGesture {
        device_id: unsafe { std::mem::zeroed() },
        delta: 0.0,
        phase: winit::event::TouchPhase::Started,
    });
}
