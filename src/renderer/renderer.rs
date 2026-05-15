use crate::core::graphics_resource_manager::{BindGroupHandle, GraphicsResourceManager, PipelineHandle};
use crate::core::scene_manager::SceneManager;
use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;
use crate::texture::Texture;
use crate::core::chunk::CHUNK_SIZE;
use crate::core::chunk::BLOCK_WIDTH;

pub const BASE_LEVEL: f32 = 0.0;


#[derive(Clone)]
pub struct RenderContext {
    pub pipeline:    PipelineHandle,
    pub bind_groups: Vec<BindGroupHandle>,
}


#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub uv_range: [f32; 4],
}

impl Instance {
    pub fn new(
        position: cgmath::Vector3<f32>,
        rotation: cgmath::Quaternion<f32>,
        uv_range: [f32; 4],
    ) -> Self {
        Self { position, rotation, uv_range }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let model = (cgmath::Matrix4::from_translation(self.position)
            * cgmath::Matrix4::from(self.rotation))
            .into();
        InstanceRaw { model, uv_range: self.uv_range }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model:    [[f32; 4]; 4],
    pub uv_range: [f32; 4],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode:    wgpu::VertexStepMode::Instance,
            attributes: &[
                // Mat4 occupies locations 5–8
                wgpu::VertexAttribute { offset:  0, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 16, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 32, shader_location: 7, format: wgpu::VertexFormat::Float32x4 },
                wgpu::VertexAttribute { offset: 48, shader_location: 8, format: wgpu::VertexFormat::Float32x4 },
                // uv_range at location 9
                wgpu::VertexAttribute { offset: 64, shader_location: 9, format: wgpu::VertexFormat::Float32x4 },
            ],
        }
    }
}


/// clear color that matches the fog color so the horizon blends seamlessly.
const SKY_COLOUR: wgpu::Color = wgpu::Color { r: 0.53, g: 0.81, b: 0.98, a: 1.0 };


pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    /// render one instance-buffer per chunk, sharing a single cube mesh.
    pub fn render_chunks(
        &self,
        wgpu_context:              &WgpuContext,
        render_ctx:                &RenderContext,
        graphics_resource_manager: &GraphicsResourceManager,
        mesh:                      &Mesh,
        depth_texture:             &Texture,
        scene_manager:             &SceneManager,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = wgpu_context.surface.get_current_texture()?;
        let view   = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            wgpu_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Chunk Render Encoder"),
                });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Chunk Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(SKY_COLOUR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes:     None,
                occlusion_query_set:  None,
            });

            pass.set_pipeline(graphics_resource_manager.get_pipeline(render_ctx.pipeline));

            for (i, &bg) in render_ctx.bind_groups.iter().enumerate() {
                pass.set_bind_group(
                    i as u32,
                    graphics_resource_manager.get_bind_group(bg),
                    &[],
                );
            }

            for (_, chunk) in scene_manager.get_chunk_array() {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, chunk.instance_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..mesh.num_indices, 0, 0..chunk.instances.len() as _);
            }
        }

        wgpu_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}