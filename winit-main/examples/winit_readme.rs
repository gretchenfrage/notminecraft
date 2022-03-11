//! The example in `winit`'s readme, adapted to use `winit-main`.

use winit_main::reexports::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};

fn main() {
    winit_main::run(|event_loop, events| {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        for event in events.iter() {
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
