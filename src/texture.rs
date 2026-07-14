use anyhow::{Ok, Result};
use image::DynamicImage;
use image::GenericImageView;
use wgpu::{Device, Queue, Sampler, TextureView};

pub struct Texture {
    pub texture: wgpu::Texture,
    pub sampler: Sampler,
    pub view: TextureView,
}

impl Texture {
    pub fn from_bytes(device: &Device, queue: &Queue, bytes: &[u8], label: &str) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, label)
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        image: &DynamicImage,
        label: &str,
    ) -> Result<Self> {
        let diffuse_rgba = image.to_rgba8();
        let dimmentions = image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimmentions.0,
            height: dimmentions.1,
            // all textures are stored as 3d, we represent our 2D texture by setting depth to 1
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // texture binding tells that we want to use this texture in shaders
            // copy_dst means tha twe want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimmentions.0),
                rows_per_image: Some(dimmentions.1),
            },
            texture_size,
        );

        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // determines what to do if the sampler gets a texture coordinate thats outside the
            // textures itself. Possible option: ClampToEdge, Repear, MirrorRepeat
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // describe what to do when the sample footprint is smaller or larger than one textel
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        return Ok(Self {
            texture: diffuse_texture,
            sampler: diffuse_sampler,
            view: diffuse_texture_view,
        });
    }
}
