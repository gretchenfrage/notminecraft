
use crate::{
    std140::{
        Std140,
        pad,
    },
    pipelines::{
        clip::ClipPipeline,
        image::ImagePipeline,
    },
    ModifierUniformData,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,
    },
};
use tracing::*;


#[derive(Debug)]
pub struct UniformBuffer {
    uniform_offset_align: usize,
    state: Option<UniformBufferState>
}

#[derive(Debug)]
struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    modifier_uniform_bind_group: BindGroup,
    clip_edit_uniform_bind_group: BindGroup,
    image_uniform_bind_group: BindGroup,
}

#[derive(Debug, Clone)]
pub struct UniformDataPacker {
    uniform_offset_align: usize,
    data: Vec<u8>,
}


impl UniformBuffer {
    pub fn new(device: &Device) -> Self {
        let uniform_offset_align = device
            .limits()
            .min_uniform_buffer_offset_alignment as usize;
        UniformBuffer {
            uniform_offset_align,
            state: None,
        }
    }

    pub fn create_packer(&self) -> UniformDataPacker {
        UniformDataPacker {
            uniform_offset_align: self.uniform_offset_align,
            data: Vec::new()
        }
    }

    pub fn upload(
        &mut self,
        data: &UniformDataPacker,
        device: &Device,
        queue: &Queue,
        modifier_uniform_bind_group_layout: &BindGroupLayout, // TODO move in?;
        clip_pipeline: &ClipPipeline,
        image_pipeline: &ImagePipeline,
    ) {
        if data.data.is_empty() {
            return;
        }

        let dst = self
            .state
            .as_ref()
            .filter(|state| state.uniform_buffer_len >= data.data.len());
        
        if let Some(dst) = dst {
            // buffer already exists and is big enough to hold data
            trace!("re-using uniform buffer");
            queue.write_buffer(&dst.uniform_buffer, 0, &data.data);
        } else {
            // buffer doesn't exist or isn't big enough
            trace!("creating new uniform buffer");
            let uniform_buffer = device
                .create_buffer_init(&BufferInitDescriptor {
                    label: Some("uniform buffer"),
                    contents: &data.data,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

            // re-create uniform buffer bind groups
            trace!("creating new uniform buffer bind groups");
            let modifier_uniform_bind_group = device
                .create_bind_group(&BindGroupDescriptor {
                    // TODO factor out
                    label: Some("modifier uniform bind group"),
                    layout: &modifier_uniform_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::Buffer(BufferBinding {
                                buffer: &uniform_buffer,
                                offset: 0,
                                size: Some(
                                    (ModifierUniformData::SIZE as u64).try_into().unwrap()
                                ),
                            }),
                        },
                    ],
                });

            let clip_edit_uniform_bind_group = clip_pipeline
                .create_clip_edit_uniform_bind_group(
                    device,
                    &uniform_buffer,
                );
            
            let image_uniform_bind_group = image_pipeline
                .create_image_uniform_bind_group(
                    device,
                    &uniform_buffer,
                );
            
            self.state = Some(UniformBufferState {
                uniform_buffer,
                uniform_buffer_len: data.data.len(),
                modifier_uniform_bind_group,
                clip_edit_uniform_bind_group,
                image_uniform_bind_group,
            });
        }
    }

    pub fn unwrap_modifier_uniform_bind_group(&self) -> &BindGroup {
        &self.state.as_ref().unwrap().modifier_uniform_bind_group
    }

    pub fn unwrap_clip_edit_uniform_bind_group(&self) -> &BindGroup {
        &self.state.as_ref().unwrap().clip_edit_uniform_bind_group
    }

    pub fn unwrap_image_uniform_bind_group(&self) -> &BindGroup {
        &self.state.as_ref().unwrap().image_uniform_bind_group
    }
}

impl UniformDataPacker {
    pub fn pack<T: Std140>(&mut self, data: &T) -> u32 {
        pad(&mut self.data, self.uniform_offset_align);
        data.pad_write(&mut self.data) as u32 // TODO less naivety
    }
}