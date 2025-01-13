use std::cmp::Ordering;
use image::{DynamicImage, GenericImage, GenericImageView};
use anyhow::*;
use cgmath::num_traits::real::Real;

#[derive(Debug)]
pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

#[derive(Debug)]
pub struct TextureCoordinates {
    #[allow(unused)]
    pub x0: u32,
    pub x1: u32,

    pub y0: u32,
    pub y1: u32,
}


impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    
    fn next_power_of_two(value: u32) -> u32{
        2u32.pow((value as f32).log2().ceil() as u32)
    }
    
    pub fn create_texture_atlas(device: &wgpu::Device, queue: &wgpu::Queue, textures: &[&[u8]], label: &str) -> Result<(Self,Vec<TextureCoordinates> )> {

        let mut textures = textures.iter().map(|path| {
            let img = image::load_from_memory(path)?;
            Ok(img)
        }).collect::<Result<Vec<_>>>()?;

        textures.sort_by(|img1,img2|{
            return img2.height().cmp(&img1.height());
        });

        let sizes = textures.iter().map(|path| {(path.height(), path.width())}).collect::<Vec<_>>();

        let max_height = sizes[0].0;
        let max_width = sizes.iter().map(|size| size.1).sum();
        
        //let final_height = Texture::next_power_of_two(max_height);
        //let final_width = Texture::next_power_of_two(max_width);
        

        
        let mut atlas = image::DynamicImage::ImageRgba8(image::RgbaImage::new(max_width, max_height));
        
        let mut x = 0;
        let mut y = 0;
        let mut texture_coordinates = Vec::new();
        
        for img in textures.iter(){
            let size = img.dimensions();
            let x1 = x + size.0;
            let y1 = y + size.1;
            let coordinates = TextureCoordinates{x0: x, x1, y0: y, y1};
            texture_coordinates.push(coordinates);
            atlas.copy_from(img, x, y).expect("failed to copy image");
            x = x1;
        }

        let texture = Self::from_image(device, queue, &atlas, Some(label))?;
        
        //save the texture to disk
        atlas.save("atlas.png")?;

        Ok((texture, texture_coordinates))
    }
    pub fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { texture, view, sampler }
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        //Nearest: Return the texel value nearest to the texture coordinates. This creates an image that's crisper from far away but pixelated up close.
        

        Ok(Self { texture, view, sampler })
    }
}