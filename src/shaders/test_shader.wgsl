struct CameraUniform {
    view_proj:     mat4x4<f32>,
    view_position: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct FogUniform {
    color: vec4<f32>,
    start: f32,
    end:   f32,
    _pad:  vec2<f32>,
};

@group(2) @binding(0)
var<uniform> fog: FogUniform;

struct VertexInput {
    @location(0) position:   vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) uv_range:       vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos:  vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;

    out.tex_coords = vec2<f32>(
        instance.uv_range.x + model.tex_coords.x * (instance.uv_range.z - instance.uv_range.x),
        instance.uv_range.y + model.tex_coords.y * (instance.uv_range.w - instance.uv_range.y),
    );

    let world_position = model_matrix * vec4<f32>(model.position, .250);
    out.world_pos     = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;

    return out;
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(2.2));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    // converting the uniform color to linear space to match the clear color
    let corrected_fog_color = srgb_to_linear(fog.color.rgb); // otherwise the fog doesnt match void color

    let dist = distance(camera.view_position.xyz, in.world_pos * 4.0);
    let factor = clamp((dist - fog.start) / (fog.end - fog.start), 0.0, 1.0);

    let final_rgb = mix(texture_color.rgb, corrected_fog_color, factor);
    return vec4<f32>(final_rgb, texture_color.a);
}