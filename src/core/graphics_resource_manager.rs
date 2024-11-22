use std::collections::HashMap;
use wgpu::BindGroupEntry;
use crate::object::Vertex;
use crate::renderer::renderer::{InstanceRaw, RenderContext};
use crate::texture;
use crate::texture::Texture;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct BindGroupHandle(u32);
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct BindGroupLayoutHandle(u32);
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct PipelineHandle(u32);
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct PipelineLayoutHandle(u32);


#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompareFunction {
    Undefined = 0,
    Never = 1,
    Less = 2,
    Equal = 3,
    LessEqual = 4,
    Greater = 5,
    NotEqual = 6,
    GreaterEqual = 7,
    Always = 8,
}

pub struct GraphicsResourceManager{
    bind_groups: HashMap<BindGroupHandle, wgpu::BindGroup>,
    bind_group_layouts: HashMap<BindGroupLayoutHandle, wgpu::BindGroupLayout>,

    pipelines: HashMap<PipelineHandle, wgpu::RenderPipeline>,
    pipeline_layouts: HashMap<PipelineLayoutHandle, wgpu::PipelineLayout>,
    /*uniforms:*/

    next_pipeline_id: u32,
    next_pipeline_layout_id: u32,
    next_bind_group_id: u32,
    next_bind_group_layout_id: u32,
}


impl GraphicsResourceManager{
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
            pipeline_layouts: HashMap::new(),
            bind_groups: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            
            next_pipeline_id: 0,
            next_pipeline_layout_id: 0,
            next_bind_group_id: 0,
            next_bind_group_layout_id: 0,
        }
    }

    pub fn create_render_context(&self, pipeline: &PipelineHandle, bind_groups: &[&BindGroupHandle]) -> RenderContext {
        RenderContext{
            pipeline: self.pipelines.get(pipeline).expect("The pipeline is missing"),
            bind_groups: bind_groups
                .iter()
                .map(|handle| self.bind_groups.get(handle).expect("The bind group layout is missing"))
                .collect(),
        }
    }


    pub fn create_pipeline_layout(
        &mut self,
        device: &wgpu::Device,
        bind_group_layout_handles: &[&BindGroupLayoutHandle],
    ) -> PipelineLayoutHandle {

        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = bind_group_layout_handles
            .iter()
            .map(|handle| self.bind_group_layouts.get(handle).expect("BindGroupLayout handle not found in bind_group_layouts"))
            .collect();

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main pipeline layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });


        let handle = PipelineLayoutHandle(self.next_pipeline_layout_id);
        self.next_pipeline_layout_id += 1;
        self.pipeline_layouts.insert(handle, layout);
        handle
    }

    pub fn create_pipeline(
        &mut self,
        device: &wgpu::Device,
        layout_handle: PipelineLayoutHandle,
        shader: &wgpu::ShaderModule, // change it into handle,
        surface_config: &wgpu::SurfaceConfiguration,
        depth_format: &Texture,
        is_instanced: bool,

    ) -> PipelineHandle {

        let layout = self.pipeline_layouts.get(&layout_handle).expect("PipelineLayoutHandle not found in pipeline_layouts");

        let mut buffers = vec![Vertex::desc()];
        let buffer = &[Vertex::desc()];

        let msaa_samples = 4;

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            
            label: Some("Render Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: Default::default(),
            },
            
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });



        let handle = PipelineHandle(self.next_pipeline_id);
        self.next_pipeline_id += 1;
        self.pipelines.insert(handle, pipeline);
        handle
    }

    pub fn create_bind_group_layout(
        &mut self,
        device: &wgpu::Device,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) -> BindGroupLayoutHandle {

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries,
            label: Some(&format!("bind_group_layout {}", self.next_bind_group_layout_id)),
        });

        let handle = BindGroupLayoutHandle(self.next_bind_group_layout_id);
        self.next_bind_group_layout_id += 1;
        self.bind_group_layouts.insert(handle, layout);
        handle
    }

    pub fn create_bind_group(
        &mut self,
        layout_handle: BindGroupLayoutHandle,
        device: &wgpu::Device,
        entries: &[BindGroupEntry],
    ) -> BindGroupHandle {

        let layout = self.bind_group_layouts.get(&layout_handle).expect("BindGroupHandle not found in bind_group_layouts");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &entries,
            label: Some(&format!("bind_group {}", self.next_bind_group_id)),
        });
        
        let handle = BindGroupHandle(self.next_bind_group_id);
        self.next_bind_group_id += 1;
        self.bind_groups.insert(handle, bind_group);
        handle
    }
}