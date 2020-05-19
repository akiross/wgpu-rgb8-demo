use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn main() {
    let evl = EventLoop::new();
    let win = Window::new(&evl).unwrap();
    let mut rend = wgpu_rgb8_demo::Renderer::new(&win, 2, 2);

    let image = wgpu_rgb8_demo::bgr2bgra(&[127, 0, 0, 255, 255, 255, 0, 0, 255, 0, 255, 0]);

    evl.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            // Redraw window as soon as events are cleared
            Event::MainEventsCleared => win.request_redraw(),
            // Resize events must rebuild the swap chain
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                rend.resize(size.width, size.height);
            }
            Event::RedrawRequested(_) => {
                rend.render(image.as_slice());
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
