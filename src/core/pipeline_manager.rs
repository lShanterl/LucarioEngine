use std::collections::HashMap;
use wgpu::{Device, RenderPipeline, PipelineLayout, ShaderModule};
use crate::object::Vertex;

pub struct PipelineManager {
    pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    next_id: u32,
}
impl PipelineManager {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            next_id: 0,
        }
    }
    fn generate_handle(&mut self) -> PipelineHandle {
        let handle = PipelineHandle(self.next_id);
        self.next_id += 1;
        handle
    }
    pub fn create_pipeline(
        &mut self,
        device: &Device,
        layout: &PipelineLayout,
        shader: &ShaderModule,
        //render_target_format: wgpu::TextureFormat,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> PipelineHandle {
        let handle = self.generate_handle();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        self.pipelines.insert(handle, render_pipeline);
        handle
    }

    pub fn get_pipeline(&self, pipeline_handle: &PipelineHandle) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(pipeline_handle)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct PipelineHandle(u32);


