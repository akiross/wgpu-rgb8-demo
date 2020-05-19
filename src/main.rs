use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use std::time::{Duration, Instant};

struct Producer {
    height: usize,
    width: usize,
    frame: usize,
    fps_t: Instant,  // When we started the timer
    fps_n: usize,    // How many frames passed since last step
    fps_c: Duration, // How many seconds to wait before fps message
}

impl Producer {
    fn new_with_size(width: usize, height: usize) -> Self {
        Producer {
            height,
            width,
            frame: 0,
            fps_t: Instant::now(),
            fps_n: 0,
            fps_c: Duration::new(5, 0),
        }
    }
    fn next_frame(&mut self) -> Vec<u8> {
        let data = (0..self.height * self.width)
            .flat_map(|i| {
                let x = i % self.width;
                let y = i / self.width;
                let col = (255.0 * (y as f32 / self.height as f32)) as u8 * (self.frame % 2) as u8;
                if ((x / 10) % 2) ^ ((y / 10) % 2) == 0 {
                    vec![col as u8, 0x00 as u8, 0x00 as u8] // Blue
                } else {
                    vec![0x00 as u8, 0x00 as u8, col as u8]
                }
            })
            .collect();
        self.frame += 1;
        self.fps_n += 1;
        let now = Instant::now();
        if now >= self.fps_t + self.fps_c {
            let dt = (now - self.fps_t).as_secs_f32(); // NOPE! now now, last call
            let n = self.fps_n as f32;
            println!("{} FPS - {} frames in {} seconds", n / dt, n, dt);
            self.fps_n = 0;
            self.fps_t = now;
        }
        return data;
    }
}

fn main() {
    env_logger::init();
    const WIDTH: usize = 400;
    const HEIGHT: usize = 300;

    let mut producer = Producer::new_with_size(WIDTH, HEIGHT);

    let evl = EventLoop::new();
    let win = Window::new(&evl).unwrap();
    let mut rend = wgpu_rgb8_demo::Renderer::new(&win, WIDTH, HEIGHT);

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
                rend.render(wgpu_rgb8_demo::bgr2bgra(producer.next_frame().as_slice()).as_slice());
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
