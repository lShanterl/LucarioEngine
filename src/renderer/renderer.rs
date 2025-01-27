use std::ops::Range;
use std::sync::Arc;
use cgmath::{InnerSpace, Rotation3, Vector3, Zero};
use tokio::sync::Mutex;
use tokio::task;
use wgpu::util::DeviceExt;
use winit::dpi::Position;
use crate::core::graphics_resource_manager::{BindGroupHandle, GraphicsResourceManager, PipelineHandle};
use crate::core::scene_manager::SceneManager;
use crate::core::wgpu_context::{self, WgpuContext};
use crate::object::Mesh;
use crate::texture::Texture;
use rayon::prelude::*;
use wgpu::hal::gles::TextureInner;

const NUM_INSTANCES_PER_ROW: u32 = 164;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(0.0, 0.0, 0.0); // Adjusted to make them touch

pub(crate) const BASE_LEVEL: f32 = 0f32;

pub struct Renderer{

}

#[derive(Clone)]
pub struct RenderContext {
    pub pipeline: PipelineHandle,
    pub bind_groups: Vec<BindGroupHandle>,
}
pub struct Scene<'a> {
    pub(crate) meshes: &'a[&'a Mesh],
}

#[derive(Clone, Copy, Debug)]
pub struct Instance{
    pub(crate) position: cgmath::Vector3<f32>,
    pub(crate) rotation: cgmath::Quaternion<f32>,
    pub(crate) texture_index: u32,
    pub(crate) u_min: f32,
    pub(crate) v_min: f32,
    pub(crate) u_max: f32,
    pub(crate) v_max: f32,
}

impl Instance {
    pub(crate) fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>, texture_index: u32) -> Self {

        let cell_width = 1.0 / 6.0; 
        let cell_height = 1.0;

        let u_min = texture_index as f32 * cell_width; 
        let v_min = 0.0;                               
        let u_max = u_min + cell_width;
        let v_max = v_min + cell_height;
        Self {
            position,
            rotation,
            texture_index,
            u_min,
            v_min,
            u_max,
            v_max,
        }
    }

    pub(crate) fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
            texture_index: self.texture_index,
            min_u: self.u_min,  // Assume these values exist in the original struct
            min_v: self.v_min,  // Same for these
            max_u: self.u_max,  // Same for these
            max_v: self.v_max,  // Same for these
        }
    }
}

pub struct State {
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
}

impl State {
    pub(crate) fn map_to_closest_multiple_of(value: u32, multiple: u32) -> u32 {
        if multiple == 0 {
            return 0;
        }
        
        let remainder = value % multiple;
        if remainder == 0 {
            value
        } else {
            value + multiple - remainder
        }
    }
    pub(crate) async fn new_single_mesh(device: &wgpu::Device, position: Vector3<f32>) -> State {
        //create a state with a single instance
        let instance = Instance {
            position,
            rotation: cgmath::Quaternion::zero(),
            texture_index: 0,
            u_min: 0.0,
            v_min: 0.0,
            u_max: 1.0,
            v_max: 1.0,

        };

        let instance_data = vec![instance.to_raw()];
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        State {
            instances: vec![instance],
            instance_buffer,
        }
    }

    pub(crate) fn new(device: &wgpu::Device) -> State {

        // let chunk_size = NUM_INSTANCES_PER_ROW;
        // let num_chunks = chunk_size * chunk_size;
        // let chunk_batch_size = 16;
        // 
        // let instances: Vec<_> = (0..num_chunks)
        //     .collect::<Vec<_>>()
        //     .into_par_iter()
        //     .chunks(chunk_batch_size)
        //     .map(|chunk_batch| {
        //         let mut batch_instances = Vec::new();
        //         for i in chunk_batch {
        //             let x = i % chunk_size;
        //             let z = i / chunk_size;
        // 
        //             let mut instances = Vec::new();
        //             let noise_value = perlin(x as f32 * 0.09, z as f32 * 0.09);
        //             let height_value = (noise_value * 250.0).clamp(0.0, 250.0) as u8;
        //             let height_value = Self::map_to_closest_multiple_of(height_value as u32, 8) as u8;
        // 
        //             for y in 0..=height_value {
        //                 let position = cgmath::Vector3 {
        //                     x: x as f32 * 8.0,
        //                     y: BASE_LEVEL + Self::map_to_closest_multiple_of(y as u32, 8) as f32,
        //                     z: z as f32 * 8.0,
        //                 };
        // 
        //                 let rotation = cgmath::Quaternion::from_axis_angle(
        //                     cgmath::Vector3::unit_x(),
        //                     cgmath::Deg(180.0),
        //                 );
        // 
        //                 let texture_index = if height_value > 200 {
        //                     0
        //                 } else if height_value > 100 {
        //                     1
        //                 } else {
        //                     2
        //                 };
        // 
        //                 instances.push(Instance { position, rotation, texture_index });
        //             }
        //             batch_instances.extend(instances);
        //         }
        //         batch_instances
        //     })
        //     .collect::<Vec<_>>();
        // 
        // let instance_data = instances.iter().flat_map(|chunk| chunk.iter().map(Instance::to_raw)).collect::<Vec<_>>();
        // 
        // let instance_buffer = device.create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: Some("Instance Buffer"),
        //         contents: bytemuck::cast_slice(&instance_data),
        //         usage: wgpu::BufferUsages::VERTEX,
        //     }
        // );
        // 
        // State {
        //     instances: instances.into_iter().flatten().collect(),
        //     instance_buffer,
        // }
        State {
            instances: vec![],
            instance_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: 0,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }),
        }
    }

}


#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    pub texture_index: u32,
    pub min_u: f32,
    pub min_v: f32,
    pub max_u: f32,
    pub max_v: f32,

}

impl InstanceRaw {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Model matrix 0 (vec4)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix 1 (vec4)
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix 2 (vec4)
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Model matrix 3 (vec4)
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Minimum U coordinate (f32)
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32,
                },
                // Minimum V coordinate (f32)
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[[f32; 4]; 4]>() + mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32,
                },
                // Maximum U coordinate (f32)
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[[f32; 4]; 4]>() + 2 * mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32,
                },
                // Maximum V coordinate (f32)
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[[f32; 4]; 4]>() + 3 * mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}


impl Renderer{
    pub fn new() -> Renderer{

        Renderer{}
    }
    pub fn render_light_mesh_instanced(
        &self,
        wgpu_context: &WgpuContext,
        light_render_ctx: &RenderContext,
        graphics_resource_manager: &GraphicsResourceManager,
        scene: &SceneManager,
        state: &State,
        mesh: &Mesh,
        depth_texture: &Texture, // Pass the depth texture
    ) -> Result<(), wgpu::SurfaceError> {
        // Obtain the current frame's texture from the swap chain
        let output_frame = wgpu_context.surface.get_current_texture()?;
        let color_view = output_frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder for the render commands
        let mut encoder = wgpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Begin the render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Light Mesh Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load the existing content
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Load the existing depth
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set the render pipeline for the light rendering
            render_pass.set_pipeline(graphics_resource_manager.get_pipeline(light_render_ctx.pipeline));

            // Bind the resource groups (e.g., uniforms, textures)
            for (i, &bind_group) in light_render_ctx.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, graphics_resource_manager.get_bind_group(bind_group), &[]);
            }

            // Set vertex and index buffers
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Draw indexed geometry for each instance
            render_pass.draw_indexed(0..mesh.num_indices, 0, 0..state.instances.len() as _);
        }

        // Submit the command buffer for execution
        wgpu_context.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        output_frame.present();

        Ok(())
    }

    pub fn render_instanced(
        &self,
        wgpu_context: &WgpuContext,
        render_ctx: &RenderContext,
        graphics_resource_manager: &GraphicsResourceManager,
        state: &State,
        mesh: &Mesh,
        depth_texture: &Texture,
    ) -> Result<(), wgpu::SurfaceError> {
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(graphics_resource_manager.get_pipeline(render_ctx.pipeline));

            for (i, &bind_group) in render_ctx.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, graphics_resource_manager.get_bind_group(bind_group), &[]);
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
        graphics_resource_manager: GraphicsResourceManager,
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

            render_pass.set_pipeline(graphics_resource_manager.get_pipeline(render_ctx.pipeline));

            for (i, &bind_group) in render_ctx.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, graphics_resource_manager.get_bind_group(bind_group), &[]);
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

    pub fn render_chunks(
        &self,
        wgpu_context: &WgpuContext,
        render_ctx: &RenderContext,
        graphics_resource_manager: &GraphicsResourceManager,
        mesh: &Mesh,
        depth_texture: &Texture,
        scene_manager: &SceneManager
    ) -> Result<(), wgpu::SurfaceError> {
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
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(graphics_resource_manager.get_pipeline(render_ctx.pipeline));

            for (i, &bind_group) in render_ctx.bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, graphics_resource_manager.get_bind_group(bind_group), &[]);
            }

            // Render each chunk
            for (_,chunk) in scene_manager.get_chunk_array() {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, chunk.instance_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..chunk.instances.len() as _);
            }
        }

        wgpu_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}