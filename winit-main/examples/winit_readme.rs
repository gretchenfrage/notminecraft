//! The example in `winit`'s readme, adapted to use `winit-main`.

use winit_main::reexports::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};

fn main() {
    winit_main::run(|event_loop, mut events| async move {
        let window = event_loop
            .create_window(WindowAttributes::default()).await
            .unwrap();

        loop {
            let event = events.recv().await;
            if matches!(
                event,
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id()
            ) {
                break;
            }
        }
    });
}
