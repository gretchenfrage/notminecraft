
#[macro_use]
extern crate tracing;

use graphics::{
    Renderer,
    Canvas2d,
};
use std::sync::Arc;
use anyhow::*;
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    reexports::event::{
        Event,
        WindowEvent,
    },
};
use tracing_subscriber::FmtSubscriber;


fn draw_frame(mut canvas: Canvas2d) {
    canvas
        .with_clip_min_x(0.5)
        .with_translate([0.25, 0.25])
        .draw_solid();
    /*
    canvas
        .with_translate([0.25, 0.5])
        .draw_solid();
    canvas
        .with_scale([0.2, 0.2])
        .with_translate([0.4, 0.4])
        .with_color([0xff, 0, 0xff, 0xff / 2])
        .draw_solid();*/
}

async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let window = event_loop.create_window(Default::default()).await?;
    let window = Arc::new(window);
    let mut renderer = Renderer::new(Arc::clone(&window)).await?;

    loop {
        match events.recv().await {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                WindowEvent::Resized(size) => {
                    renderer.resize(size);
                },
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let result = renderer.draw_frame(|canvas| {
                    draw_frame(canvas);
                });
                if let Err(e) = result {
                    error!(error=%e, "draw_frame error");
                }
                window.request_redraw();
            },
            _ => (),
        };
    }

    Ok(())
}

fn main() {
    let subscriber = FmtSubscriber::builder().finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    winit_main::run(|event_loop, events| async move {
        let result = window_main(event_loop, events).await;
        if let Err(e) = result {
            error!("{:?}", e);
        }
    });
}
