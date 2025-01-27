// Camera Uniform Struct
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Vertex Input Struct
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

// Instance Input Struct
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) u_min: f32, 
    @location(10) v_min: f32,
    @location(11) u_max: f32,
    @location(12) v_max: f32,
};

// Vertex Output Struct
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

// Vertex Shader
@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;


    out.tex_coords = model.tex_coords * vec2<f32>(instance.u_max - instance.u_min, instance.v_max - instance.v_min) + vec2<f32>(instance.u_min, instance.v_min);


    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 0.25);
    return out;
}

// Texture and Sampler Bindings
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// Fragment Shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //return textureSample(t_diffuse, s_diffuse, in.tex_coords);

    return vec4<f32>(in.tex_coords, 0.0, 1.0);
}
