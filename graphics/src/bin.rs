
#[macro_use]
extern crate tracing;

use graphics::Renderer;
use std::{
    panic,
    sync::Arc,
    time::{
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
    },
};
use tracing_subscriber::{
    FmtSubscriber,
    EnvFilter,
};
use backtrace::Backtrace;



mod game_behavior {
    use graphics::{
        Renderer,
        frame_content::{
            FrameContent,
            GpuImage,
            TextBlock,
            TextSpan,
            HorizontalAlign,
            VerticalAlign,
            LayedOutTextBlock,
            FontId,
            GpuImageArray,
            Mesh,
            Vertex,
            Triangle,
        },
        view_proj::ViewProj,
    };
    use winit_main::reexports::dpi::PhysicalSize;
    use vek::*;
    use anyhow::*;

    #[allow(dead_code)]
    pub struct GameBehavior {
        renderer: Renderer,
        image: GpuImage,
        font: FontId,
        text: LayedOutTextBlock,
        image_array: GpuImageArray,
        mesh: Mesh,
    }

    impl GameBehavior {
        pub async fn new(mut renderer: Renderer) -> Result<Self> {
            debug!("loading");
            let image = renderer.load_image_file("src/assets/sheep.jpg").await?;
            let font = renderer.load_font_file("src/assets/LiberationSerif-Regular.ttf").await?;
            let text = renderer
                .lay_out_text(&TextBlock {
                    spans: &[
                        TextSpan {
                            text: "h",
                            font_id: font,
                            font_size: 24.0,
                            color: Rgba::black(),
                        },
                        TextSpan {
                            text: "e",
                            font_id: font,
                            font_size: 24.0,
                            color: Rgba::black(),
                        },
                        TextSpan {
                            text: "e",
                            font_id: font,
                            font_size: 24.0,
                            color: Rgba::black(),
                        },
                        TextSpan {
                            text: "w",
                            font_id: font,
                            font_size: 280.0,
                            color: Rgba::red(),
                        },
                        TextSpan {
                            text: "w",
                            font_id: font,
                            font_size: 280.0,
                            color: Rgba::green(),
                        },
                        TextSpan {
                            text: "w",
                            font_id: font,
                            font_size: 280.0,
                            color: Rgba::blue(),
                        }
                    ],
                    horizontal_align: HorizontalAlign::Left {
                        width: None,
                    },
                    vertical_align: VerticalAlign::Top,
                });
            let image_array = renderer
                .load_image_array_files(
                    None,
                    &[
                        "src/assets/sheep.jpg",
                    ]
                ).await?;
            let mesh = Mesh {
                vertices: renderer
                    .create_gpu_vec_init(&[
                        Vertex {
                            pos: [-0.25, 0.25, 1.0].into(),
                            tex: [0.0, 0.0].into(),
                            color: Rgba::white(),
                            tex_index: 0,
                        },
                        Vertex {
                            pos: [0.25, 0.25, 1.0].into(),
                            tex: [1.0, 0.0].into(),
                            color: Rgba::white(),
                            tex_index: 0,
                        },
                        Vertex {
                            pos: [0.25, -0.25, 1.0].into(),
                            tex: [1.0, 1.0].into(),
                            color: Rgba::white(),
                            tex_index: 0,
                        },
                        Vertex {
                            pos: [-0.25, -0.25, 1.0].into(),
                            tex: [0.0, 1.0].into(),
                            color: Rgba::white(),
                            tex_index: 0,
                        },
                    ]),
                triangles: renderer
                    .create_gpu_vec_init(&[
                        Triangle([0, 2, 1]),
                        Triangle([0, 3, 2]),
                    ]),
            };
            debug!("loaded");
            Ok(GameBehavior {
                renderer,
                image,
                font,
                text,
                image_array,
                mesh,
            })
        }

        pub async fn draw<'a>(&mut self) -> Result<()> {
            debug!("drawing");
            let mut frame = FrameContent::new();
            /*frame.canvas()
                .draw_image(&self.image, [300.0, 300.0])
                .min_x(150.0)
                .translate([0.0, 200.0])
                .rotate(0.5)
                .draw_text(&self.text)
                ;*/
            frame.canvas()
                .scale(self.renderer.size().map(|n| n as f32))
                .begin_3d(ViewProj::perspective(
                    [0.0, 0.0, 0.0].into(),
                    Quaternion::rotation_y(f32::to_radians(15.0)),
                    f32::to_radians(75.0),
                    1.0,
                ))
                .draw_mesh(&self.mesh, &self.image_array)
                ;
            debug!("{:#?}", frame);
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
