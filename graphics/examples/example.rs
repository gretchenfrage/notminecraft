
#[macro_use]
extern crate tracing;

use graphics::{
    Renderer,
    Canvas2d,
};
use std::sync::Arc;
use anyhow::Result;
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    reexports::event::{
        Event,
        WindowEvent,
    },
};


fn draw_frame(mut canvas: Canvas2d) {
    canvas.draw_solid();
}

fn window_main(event_loop: EventLoopHandle, events: EventReceiver) -> Result<()> {
    let window = event_loop.create_window(Default::default())?;
    let window = Arc::new(window);
    let mut renderer = Renderer::new(Arc::clone(&window))?;

    for event in events.iter() {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                renderer.draw_frame(|canvas| {
                    draw_frame(canvas);
                });
                window.request_redraw();
            },
            _ => (),
        };
    }

    Ok(())
}

fn main() {
    winit_main::run(|event_loop, events| {
        let result = window_main(event_loop, events);
        if let Err(e) = result {
            error!("{}", e);
        }
    });
}