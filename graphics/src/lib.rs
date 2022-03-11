
use std::sync::Arc;
use anyhow::Result;
use winit_main::{
    EventLoopHandle,
    reexports::window::Window,
};
use wgpu::*;


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    window: Arc<Window>,
}

impl Renderer {
    /// Create a new renderer on a given window.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&*window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            });

        Ok(Renderer {
            window,
        })
    }

    /// Draw a frame. The callback can draw onto the Canvas2d. Then it will be
    /// displayed on the window from <0,0> (top left corner) to <1,1> (bottom
    /// right corner).
    pub fn draw_frame(&mut self, f: impl FnOnce(Canvas2d)) {

    }
}

/// Target for drawing 2 dimensionally onto. Each successive draw call is
/// blended over the previously drawn data.
pub struct Canvas2d {

}

impl Canvas2d {
    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {

    }
}
