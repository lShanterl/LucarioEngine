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

    pub fn create_texture_atlas(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures_bytes: &[&[u8]],
        label: &str,
    ) -> Result<(Self, Vec<TextureCoordinates>)> {
        let textures = textures_bytes
            .iter()
            .map(|b| image::load_from_memory(b).map_err(|e| anyhow!(e)))
            .collect::<Result<Vec<_>>>()?;

        let tile_size: u32 = 16;
        let gutter: u32   = 16; // must equal tile_size for mip halving to work
        let n = textures.len() as u32;
        let mip_count = (tile_size as f32).log2() as u32 + 1; // log2(16)+1 = 5

        let base_width  = n * (tile_size + gutter);
        let base_height = tile_size;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: base_width, height: base_height, depth_or_array_layers: 1 },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for m in 0..mip_count {
            let t = (tile_size >> m).max(1);   // tile pixels at this mip
            let g = (gutter   >> m).max(1);   // gutter pixels at this mip
            let mip_w = n * (t + g);
            let mip_h = t;

            let mut atlas = image::RgbaImage::new(mip_w, mip_h);

            for (i, img) in textures.iter().enumerate() {
                // resize tile to this mip level using nearest (preserves pixel-art look)
                let resized = img.resize_exact(t, t, image::imageops::FilterType::Nearest);
                let x_off = i as u32 * (t + g);

                // copy tile pixels
                for py in 0..t {
                    for px in 0..t {
                        atlas.put_pixel(x_off + px, py, resized.get_pixel(px, py));
                    }
                }
                // extrude right edge into right gutter
                for gi in 0..g {
                    for py in 0..t {
                        atlas.put_pixel(x_off + t + gi, py, resized.get_pixel(t - 1, py));
                    }
                }
                // extrude left edge into left gutter of previous slot (safe because
                // the previous tile's right gutter is our left neighbor)
                if x_off > 0 {
                    for gi in 0..g {
                        for py in 0..t {
                            atlas.put_pixel(x_off - 1 - gi, py, resized.get_pixel(0, py));
                        }
                    }
                }
            }

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: m,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                atlas.as_raw(),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * mip_w),
                    rows_per_image: Some(mip_h),
                },
                wgpu::Extent3d { width: mip_w, height: mip_h, depth_or_array_layers: 1 },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,  // crisp up-close
            min_filter: wgpu::FilterMode::Nearest,  // crisp at distance
            mipmap_filter: wgpu::FilterMode::Linear, // smooth mip transitions
            ..Default::default()
        });

        let coords = textures
            .iter()
            .enumerate()
            .map(|(i, _)| TextureCoordinates {
                x0: i as u32 * (tile_size + gutter),
                x1: i as u32 * (tile_size + gutter) + tile_size,
                y0: 0,
                y1: tile_size,
            })
            .collect();

        Ok((Self { texture, view, sampler }, coords))
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
        let mip_level_count = (dimensions.0.max(dimensions.1) as f32).log2().floor() as u32 + 1;

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }
        );

        for m in 0..mip_level_count {
            let mip_size = wgpu::Extent3d {
                width: (dimensions.0 >> m).max(1),
                height: (dimensions.1 >> m).max(1),
                depth_or_array_layers: 1,
            };

            let resized = img.resize(mip_size.width, mip_size.height, image::imageops::FilterType::Gaussian);
            let rgba_mip = resized.to_rgba8();

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: m,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba_mip,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * mip_size.width),
                    rows_per_image: Some(mip_size.height),
                },
                mip_size,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                anisotropy_clamp: 1,
                ..Default::default()
            }
        );
        //Nearest: Return the texel value nearest to the texture coordinates. This creates an image that's crisper from far away but pixelated up close.
        

        Ok(Self { texture, view, sampler })
    }
}