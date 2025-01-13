use wgpu::util::DeviceExt;
use crate::texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords:   [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords:  [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords:   [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords:    [0.9414737, 0.2652641], }, // E
];

pub const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

pub const CUBE_VERTICES: &[Vertex] = &[
    // Front face
    Vertex { position: [-1.0, -1.0,  1.0], tex_coords: [0.0, 0.0], }, // 0
    Vertex { position: [ 1.0, -1.0,  1.0], tex_coords: [1.0, 0.0], }, // 1
    Vertex { position: [ 1.0,  1.0,  1.0], tex_coords: [1.0, 1.0], }, // 2
    Vertex { position: [-1.0,  1.0,  1.0], tex_coords: [0.0, 1.0], }, // 3
    // Back face
    Vertex { position: [-1.0, -1.0, -1.0], tex_coords: [1.0, 0.0], }, // 4
    Vertex { position: [ 1.0, -1.0, -1.0], tex_coords: [0.0, 0.0], }, // 5
    Vertex { position: [ 1.0,  1.0, -1.0], tex_coords: [0.0, 1.0], }, // 6
    Vertex { position: [-1.0,  1.0, -1.0], tex_coords: [1.0, 1.0], }, // 7

    // Top face (unique UVs)
    Vertex { position: [-1.0,  1.0, -1.0], tex_coords: [0.0, 0.0] }, // 8
    Vertex { position: [ 1.0,  1.0, -1.0], tex_coords: [1.0, 0.0] }, // 9
    Vertex { position: [ 1.0,  1.0,  1.0], tex_coords: [1.0, 1.0] }, // 10
    Vertex { position: [-1.0,  1.0,  1.0], tex_coords: [0.0, 1.0] }, // 11

    // Bottom face (unique UVs)
    Vertex { position: [-1.0, -1.0, -1.0], tex_coords: [0.0, 0.0] }, // 12
    Vertex { position: [ 1.0, -1.0, -1.0], tex_coords: [1.0, 0.0] }, // 13
    Vertex { position: [ 1.0, -1.0,  1.0], tex_coords: [1.0, 1.0] }, // 14
    Vertex { position: [-1.0, -1.0,  1.0], tex_coords: [0.0, 1.0] }, // 15
];


pub const CUBE_INDICES: &[u16] = &[
    // Front face
    0, 1, 2, 2, 3, 0,

    // Top face (updated indices)
    11, 10, 9, 9, 8, 11,

    // Back face
    7, 6, 5, 5, 4, 7,

    // Left face
    4, 0, 3, 3, 7, 4,

    // Bottom face (updated indices)
    12, 13, 14, 14, 15, 12,

    // Right face
    1, 5, 6, 6, 2, 1
];

pub const CONE_VERTICES: &[Vertex] = &[
    // Base vertices
    Vertex { position: [0.0, -0.5, -1.0], tex_coords: [0.25, 0.49] }, // 0
    Vertex { position: [1.0, -0.5, 0.0], tex_coords: [0.25, 0.25] }, // 1
    Vertex { position: [0.0, -0.5, 1.0], tex_coords: [0.49, 0.25] }, // 2
    Vertex { position: [-1.0, -0.5, 0.0], tex_coords: [0.25, 0.01] }, // 3
    // Apex of the cone
    Vertex { position: [0.0, 0.5, 0.0], tex_coords: [0.75, 0.49] }, // 4
];


pub const CONE_INDICES: &[u16] = &[
    4, 1, 0,
    4, 2, 1,
    4, 3, 2,
    4, 0, 3,
    2, 1, 0,
    0, 3, 2,
];
#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

// create an enum of materials
// create a hashmap of materials

pub enum BlockTypes{
    Grass,
    Dirt,
    Stone,
    Wood,
    Leaves,
    Water,
}

#[derive(Debug)]
pub(crate) struct Mesh{
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) num_vertices: u32,
    pub(crate) num_indices: u32,

    //pub(crate) material: Material,
}


impl Mesh{
    pub fn new(device: &wgpu::Device, vertex_buffer_ar: &[Vertex], index_buffer_ar: &[u16]) -> Self{

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertex_buffer_ar),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(index_buffer_ar),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_vertices = vertex_buffer_ar.len() as u32;
        let num_indices = index_buffer_ar.len() as u32;


        Self{
            vertex_buffer,
            index_buffer,
            num_vertices,
            num_indices,
        }
    }
    
    pub fn new_cube_at(device: &wgpu::Device, position: [f32; 3], color: [f32; 3]) -> Self{
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(CUBE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(CUBE_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_vertices = CUBE_VERTICES.len() as u32;
        let num_indices = CUBE_INDICES.len() as u32;

        Self{
            vertex_buffer,
            index_buffer,
            num_vertices,
            num_indices,
        }
    }
}

pub(crate) struct CubeMesh{
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
    
}
