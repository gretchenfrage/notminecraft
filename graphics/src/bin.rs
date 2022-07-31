
#[macro_use]
extern crate tracing;

use graphics::{*, frame_content::*};
use std::{
    panic,
    sync::Arc,
    time::{
        SystemTime,
        Instant,
        Duration,
    },
    thread::sleep,
    env,
};
use anyhow::*;
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    reexports::event::{
        Event,
        WindowEvent,
        KeyboardInput,
        VirtualKeyCode,
        ElementState,
    },
};
use tracing_subscriber::{
    FmtSubscriber,
    EnvFilter,
};
use backtrace::Backtrace;
use vek::*;



mod game_behavior {
    use graphics::{
        Renderer,
        frame_content::{
            FrameContent,
            Canvas2,
            GpuImage,
        },
    };
    use winit_main::reexports::dpi::PhysicalSize;
    use anyhow::*;

    pub struct GameBehavior {
        renderer: Renderer,
        image: GpuImage,
    }

    impl GameBehavior {
        pub async fn new(renderer: Renderer) -> Result<Self> {
            let image = renderer.load_image_file("src/assets/sheep.jpg").await?;
            Ok(GameBehavior {
                renderer,
                image,
            })
        }

        pub async fn draw<'a>(&mut self) -> Result<()> {
            let mut frame = FrameContent::new();
            frame.canvas()
                //.min_x(0.5)
                //.max_y(0.5)
                //.translate([0.25, 0.25])
                //.scale([0.5, 0.5])
                //.scale([-1.0, 1.0])
                //.translate([-0.5, -0.5])
                //.scale([2.0, -2.0])
                //.color([1.0, 0.0, 0.0, 1.0])
                //.draw_solid()
                .draw_image(&self.image, [0.0, 0.0], [1.0, 1.0]);
            self.renderer.draw_frame(&frame)
        }

        pub async fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
            self.renderer.resize(size);
            Ok(())
        }
    }
}



async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let window = event_loop.create_window(Default::default()).await?;
    let window = Arc::new(window);
    let renderer = Renderer::new(Arc::clone(&window)).await?;

    let mut game = game_behavior::GameBehavior::new(renderer).await?;

    //let image = renderer.load_image_file("src/assets/sheep.jpg").await?;

    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    loop {
        let event = events.recv().await;
        trace!(?event, "received event");
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        trace!("not resizing window because of 0 size"); // TODO factor in
                    } else {
                        trace!("resizing window");
                        game.resize(size).await?;
                    }
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                let before_frame = Instant::now();

                // draw frame
                trace!("drawing frame");
                //let mut frame = FrameContent::new();
                let result = game.draw().await;
                
                //let result = renderer.draw_frame(&frame);
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
        //.with_ansi(false).with_writer(std::fs::File::create("log.txt").unwrap())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    info!("starting program");

    panic::set_hook(Box::new(|info| {
        error!("{}", info);
        if env::var("RUST_BACKTRACE").map(|val| val == "1").unwrap_or(false) {
            error!("{:?}", Backtrace::new());
        }
    }));
    trace!("installed custom panic hook");

    winit_main::run(|event_loop, events| async move {
        trace!("successfully bootstrapped winit + tokio");
        let result = window_main(event_loop, events).await;
        if let Err(e) = result {
            error!(error=%e, "exit with error");
        }
    });
}
