struct CameraUniform {
    view_proj:     mat4x4<f32>,
    view_position: vec4<f32>,
};
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct FogUniform {
    color: vec4<f32>,
    start: f32,
    end:   f32,
    _pad:  vec2<f32>,
};
@group(2) @binding(0) var<uniform> fog: FogUniform;

struct VertexInput {
    @location(0) position:  vec3<f32>,
    @location(1) uv:        vec2<f32>,
    @location(2) uv_rect:   vec4<f32>,
    @location(3) tex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv:        vec2<f32>,   // raw UV [0..w, 0..h]
    @location(1) uv_rect:   vec4<f32>,   // the atlas slot [u0, v0, u1, v1]
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.uv = model.uv;
    out.uv_rect = model.uv_rect;

    let world_pos = vec4<f32>(model.position, 1.0);
    out.world_pos = world_pos.xyz;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}


@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(2.2));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let local_uv = fract(in.uv);

    // unterpolate inside the specific atlas rect atlass mapping
    let tex_coords = vec2<f32>(
        in.uv_rect.x + (local_uv.x * (in.uv_rect.z - in.uv_rect.x)),
        in.uv_rect.y + (local_uv.y * (in.uv_rect.w - in.uv_rect.y))
    );

    let texture_color = textureSample(t_diffuse, s_diffuse, tex_coords);
    if (texture_color.a < 0.1) {
        discard;
    }

    let corrected_fog_color = srgb_to_linear(fog.color.rgb);
    let dist = distance(camera.view_position.xyz, in.world_pos);
    let factor = clamp((dist - fog.start) / (fog.end - fog.start), 0.0, 1.0);
    let final_rgb = mix(texture_color.rgb, corrected_fog_color, factor);
    return vec4<f32>(final_rgb, texture_color.a);
}