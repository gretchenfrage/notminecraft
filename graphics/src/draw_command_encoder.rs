
use crate::{
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
};
use wgpu::*;
use std::{
    mem::replace,
    iter::once,
};


pub struct DrawCommandEncoder<'a> {
    pub device: &'a Device,
    pub color_texture: &'a TextureView,
    pub depth_texture: &'a TextureView,

    pub command: CommandEncoder,
    pub render: RenderBundleEncoder<'a>,
}

fn make_render_encoder(device: &Device) -> RenderBundleEncoder {
    device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
        label: Some("draw render bundle encoder"),
        color_formats: &[Some(SWAPCHAIN_FORMAT)],
        depth_stencil: Some(RenderBundleDepthStencil {
            format: DEPTH_FORMAT,
            depth_read_only: false,
            stencil_read_only: true
        }),
        sample_count: 1,
        multiview: None,
    })
}

impl<'a> DrawCommandEncoder<'a> {
    pub fn new(
        device: &'a Device,
        color_texture: &'a TextureView,
        depth_texture: &'a TextureView,
        command: CommandEncoder,
    ) -> Self {
        DrawCommandEncoder {
            device,
            color_texture,
            depth_texture,
            command,
            render: make_render_encoder(device),
        }
    }

    pub fn submit_render(&mut self) {
        let render_bundle_encoder = replace(
            &mut self.render,
            make_render_encoder(self.device),
        );
        let render_bundle = render_bundle_encoder
            .finish(&RenderBundleDescriptor {
                label: Some("draw render bundle"),
            });
        let mut render_pass = self.command
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("draw render pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: self.color_texture,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: self.depth_texture,
                    depth_ops: Some(Operations {
                        load: depth_load_op,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        render_pass.execute_bundles(once(&render_bundle));
        drop(render_pass)
    }
}
