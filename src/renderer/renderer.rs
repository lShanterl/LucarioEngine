use std::ops::Range;
use cgmath::{InnerSpace, Rotation3, Zero};
use wgpu::util::DeviceExt;
use winit::dpi::Position;
use crate::core::scene_manager::SceneManager;
use crate::core::wgpu_context::{self, WgpuContext};
use crate::object::Mesh;
use crate::texture::Texture;
use crate::utils::perlin_noise::perlin_noise::perlin;

const NUM_INSTANCES_PER_ROW: u32 = 255;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 0.0, 0.0); // Adjusted to make them touch



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

pub struct Instance{
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

pub struct State {
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
}

impl State {
    pub(crate) fn new(device: &wgpu::Device) -> State {

        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let noise_value = perlin(x as f32 * 0.09, z as f32 * 0.09);
                let height_value = (noise_value * 250.0).clamp(0.0, 250.0) as u8;
                
                let position = cgmath::Vector3 { x: x as f32 * 8f32, y: height_value as f32 - 100f32, z: z as f32 * 8f32};

                let rotation = cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Deg(180.0));

                Instance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        State {
            instances,
            instance_buffer,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}


impl Renderer{
    pub fn new() -> Renderer{

        Renderer{}
    }


    pub fn render_instanced(
        &self,
        wgpu_context: &WgpuContext,
        render_ctx: &RenderContext,
        state: &State,
        mesh: &Mesh,
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

            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            
            render_pass.draw_indexed(0..mesh.num_indices, 0, 0..state.instances.len() as _);

        }

        wgpu_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    pub fn render(
        &self,
        wgpu_context: &WgpuContext,
        render_ctx: &RenderContext,
        scene: &SceneManager,
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

            for (mesh_id, mesh) in scene.iter() {
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