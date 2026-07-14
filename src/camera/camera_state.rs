use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, Buffer, BufferUsages,
    Device, ShaderStages,
};

use crate::camera::camera_uniform::{self, CameraUniform};

pub struct CameraState {
    pub camera_uniform: CameraUniform,
    pub camera_bind_group: BindGroup,
    pub camera_bind_group_layout: BindGroupLayout,
    pub camera_buffer: Buffer,
}

impl CameraState {
    pub fn get_camera_init_state(device: &Device) -> CameraState {
        let camera_uniform = camera_uniform::CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            label: Some("camera_bind_group"),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self {
            camera_uniform: camera_uniform,
            camera_bind_group: camera_bind_group,
            camera_bind_group_layout: camera_bind_group_layout,
            camera_buffer: camera_buffer,
        }
    }

    pub fn get_camera_buffer(&self) -> &Buffer {
        &self.camera_buffer
    }
}
