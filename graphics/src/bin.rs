
#[macro_use]
extern crate tracing;

use graphics::{
    Renderer,
    Canvas2d,
};
use std::{
    sync::Arc,
    time::{
        SystemTime,
        Instant,
        Duration,
    },
    thread::sleep,
};
use anyhow::*;
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    reexports::event::{
        Event,
        WindowEvent,
    },
};
use tracing_subscriber::{
    FmtSubscriber,
    EnvFilter,
};


struct Graphics {
    start_time: SystemTime,
}

impl Graphics {
    fn draw_frame(&mut self, mut canvas: Canvas2d) {
        let elapsed = self.start_time.elapsed().unwrap().as_millis() as f32 / 1000.0;
        let scale = elapsed.sin() * 0.4 + 0.6;
        trace!(%scale);
        let translate = (1.0 - scale) / 2.0;
        canvas
            .with_clip_min_x(0.1)
            .with_clip_min_y(0.2)
            .with_clip_max_x(0.7)
            .with_clip_max_y(0.6)
            .with_scale([scale, scale])
            .with_translate([translate, translate])
            .draw_solid();
    }
}

async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let window = event_loop.create_window(Default::default()).await?;
    let window = Arc::new(window);
    let mut renderer = Renderer::new(Arc::clone(&window)).await?;
    let mut graphics = Graphics {
        start_time: SystemTime::now(),
    };

    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    loop {
        let event = events.recv().await;
        trace!(?event, "received event");
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                WindowEvent::Resized(size) => {
                    trace!("resizing window");
                    renderer.resize(size);
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                let before_frame = Instant::now();

                // draw frame
                trace!("drawing frame");
                let result = renderer.draw_frame(|canvas| {
                    graphics.draw_frame(canvas);
                });
                if let Err(e) = result {
                    error!(error=%e, "draw_frame error");
                }

                // wait
                let after_frame = Instant::now();
                let delay = (before_frame + frame_delay)
                    .checked_duration_since(after_frame);
                if let Some(delay) = delay {
                    sleep(delay);
                }

                // request redraw
                window.request_redraw();
            },
            _ => (),
        };
    }

    Ok(())
}

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    winit_main::run(|event_loop, events| async move {
        let result = window_main(event_loop, events).await;
        if let Err(e) = result {
            error!(error=%e, "exit with error");
        }
    });
}
