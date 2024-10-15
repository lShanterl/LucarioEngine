use crate::core::wgpu_context::{self, WgpuContext};
use crate::object::Mesh;
use crate::texture::Texture;

pub struct Renderer{

}

#[derive(Debug)]
pub struct RenderContext<'a> {
    pub(crate) pipeline: &'a wgpu::RenderPipeline,
    pub(crate) bind_groups: Vec<&'a wgpu::BindGroup>,
}
pub struct Scene<'a> {
    pub(crate) meshes: &'a[&'a Mesh],
}

impl Renderer{
    pub fn new() -> Renderer{

        Renderer{}
    }

    pub fn render(
        &self,
        wgpu_context: &WgpuContext,
        render_ctx: &RenderContext,
        scene: &Scene,
        depth_texture: &Texture
    ) -> Result<(), wgpu::SurfaceError>{

        

        let output = wgpu_context.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = wgpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(render_ctx.pipeline);

            for (i, &bind_group) in render_ctx.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, bind_group, &[]);
            }

            for mesh in scene.meshes {

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

        } // encoder.begin_render_pass(...) borrows encoder mutably. I can't call encoder.finish() until I release that mutable borrow. That's why here's an additional scope
        // I could also achieve similar effect by using drop(_render_pass)

        wgpu_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}