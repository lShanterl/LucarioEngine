use std::collections::HashMap;

use wgpu::BindGroupEntry;

use crate::object::Vertex;
use crate::renderer::renderer::{InstanceRaw, RenderContext};
use crate::texture::{self, Texture};

// ── Typed handles ─────────────────────────────────────────────────────────────
// Newtype wrappers around u32 give us compile-time safety: you can't
// accidentally pass a PipelineHandle where a BindGroupHandle is expected.

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct BindGroupHandle(u32);

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct BindGroupLayoutHandle(u32);

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct PipelineHandle(u32);

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct PipelineLayoutHandle(u32);


/// Owns all wgpu render pipelines, pipeline layouts, bind groups, and bind
/// group layouts for the lifetime of the application. Resources are accessed
/// through cheap opaque handles rather than raw references, which keeps the
/// borrow checker happy across the rest of the codebase :)
pub struct GraphicsResourceManager {
    bind_groups:         HashMap<BindGroupHandle, wgpu::BindGroup>,
    bind_group_layouts:  HashMap<BindGroupLayoutHandle, wgpu::BindGroupLayout>,
    pipelines:           HashMap<PipelineHandle, wgpu::RenderPipeline>,
    pipeline_layouts:    HashMap<PipelineLayoutHandle, wgpu::PipelineLayout>,

    next_pipeline_id:          u32,
    next_pipeline_layout_id:   u32,
    next_bind_group_id:        u32,
    next_bind_group_layout_id: u32,
}

impl GraphicsResourceManager {
    pub fn new() -> Self {
        Self {
            bind_groups:        HashMap::new(),
            bind_group_layouts: HashMap::new(),
            pipelines:          HashMap::new(),
            pipeline_layouts:   HashMap::new(),

            next_pipeline_id:          0,
            next_pipeline_layout_id:   0,
            next_bind_group_id:        0,
            next_bind_group_layout_id: 0,
        }
    }


    pub fn get_bind_group(&self, handle: BindGroupHandle) -> &wgpu::BindGroup {
        self.bind_groups.get(&handle).expect("invalid BindGroupHandle")
    }

    pub fn get_bind_group_layout(
        &self,
        handle: BindGroupLayoutHandle,
    ) -> &wgpu::BindGroupLayout {
        self.bind_group_layouts
            .get(&handle)
            .expect("invalid BindGroupLayoutHandle")
    }

    pub fn get_pipeline(&self, handle: PipelineHandle) -> &wgpu::RenderPipeline {
        self.pipelines.get(&handle).expect("invalid PipelineHandle")
    }

    pub fn get_pipeline_layout(
        &self,
        handle: PipelineLayoutHandle,
    ) -> &wgpu::PipelineLayout {
        self.pipeline_layouts
            .get(&handle)
            .expect("invalid PipelineLayoutHandle")
    }


    pub fn create_bind_group_layout(
        &mut self,
        device:  &wgpu::Device,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) -> BindGroupLayoutHandle {
        let id = self.next_bind_group_layout_id;
        self.next_bind_group_layout_id += 1;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some(&format!("bind_group_layout_{id}")),
            entries,
        });

        let handle = BindGroupLayoutHandle(id);
        self.bind_group_layouts.insert(handle, layout);
        handle
    }

    pub fn create_bind_group(
        &mut self,
        layout_handle: BindGroupLayoutHandle,
        device:        &wgpu::Device,
        entries:       &[BindGroupEntry],
    ) -> BindGroupHandle {
        let id = self.next_bind_group_id;
        self.next_bind_group_id += 1;

        let layout = self
            .bind_group_layouts
            .get(&layout_handle)
            .expect("invalid BindGroupLayoutHandle in create_bind_group");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some(&format!("bind_group_{id}")),
            layout,
            entries,
        });

        let handle = BindGroupHandle(id);
        self.bind_groups.insert(handle, bind_group);
        handle
    }

    pub fn create_pipeline_layout(
        &mut self,
        device:                    &wgpu::Device,
        bind_group_layout_handles: &[&BindGroupLayoutHandle],
    ) -> PipelineLayoutHandle {
        let id = self.next_pipeline_layout_id;
        self.next_pipeline_layout_id += 1;

        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = bind_group_layout_handles
            .iter()
            .map(|h| {
                self.bind_group_layouts
                    .get(h)
                    .expect("invalid BindGroupLayoutHandle in create_pipeline_layout")
            })
            .collect();

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                Some(&format!("pipeline_layout_{id}")),
            bind_group_layouts:   &bind_group_layouts,
            push_constant_ranges: &[],
        });

        let handle = PipelineLayoutHandle(id);
        self.pipeline_layouts.insert(handle, layout);
        handle
    }

    pub fn create_pipeline(
        &mut self,
        device:         &wgpu::Device,
        layout_handle:  PipelineLayoutHandle,
        shader:         &wgpu::ShaderModule,
        surface_config: &wgpu::SurfaceConfiguration,
        depth_texture:  Option<&Texture>,
        _is_instanced:  bool, // reserved for future non-instanced variant
    ) -> PipelineHandle {
        let id = self.next_pipeline_id;
        self.next_pipeline_id += 1;

        let layout = self
            .pipeline_layouts
            .get(&layout_handle)
            .expect("invalid PipelineLayoutHandle in create_pipeline");

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some(&format!("render_pipeline_{id}")),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module:               shader,
                entry_point:          "vs_main",
                buffers:              &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options:  Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module:      shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend:  Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology:           wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face:         wgpu::FrontFace::Ccw,
                cull_mode:          Some(wgpu::Face::Back),
                polygon_mode:       wgpu::PolygonMode::Fill,
                unclipped_depth:    false,
                conservative:       false,
            },
            depth_stencil: depth_texture.map(|_| wgpu::DepthStencilState {
                format:               texture::Texture::DEPTH_FORMAT,
                depth_write_enabled:  true,
                depth_compare:        wgpu::CompareFunction::Less,
                stencil:              wgpu::StencilState::default(),
                bias:                 wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count:                     1,
                mask:                      !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache:     None,
        });

        let handle = PipelineHandle(id);
        self.pipelines.insert(handle, pipeline);
        handle
    }

    pub fn create_render_context(
        &self,
        pipeline:    &PipelineHandle,
        bind_groups: &[&BindGroupHandle],
    ) -> RenderContext {
        RenderContext {
            pipeline:    *pipeline,
            bind_groups: bind_groups.iter().copied().cloned().collect(),
        }
    }
}