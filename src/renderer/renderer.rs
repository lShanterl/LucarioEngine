use crate::core::wgpu_context::WgpuContext;
use crate::object::Mesh;

pub struct Renderer{

}

impl Renderer{
    pub fn new() -> Renderer{

        Renderer{}
    }

    pub fn render(
        &self,
        wgpu_context: &WgpuContext,
        pipeline: &wgpu::RenderPipeline,
        meshes: &[&Mesh],
        surface: &wgpu::Surface,
        texture_bind_group: &wgpu::BindGroup,
    ) -> Result<(), wgpu::SurfaceError>{

        let output = surface.get_current_texture()?;

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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &texture_bind_group, &[]);

            for mesh in meshes {

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