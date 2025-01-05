
@group(1)@binding(0)
var<uniform> view: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) position: vec3<f32>,
}

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
    out.tex_coords = model.tex_coords;

    out.normal = normalize(model_matrix * vec4<f32>(model.normal, 0.0)).xyz;
    out.clip_position = view * model_matrix * vec4<f32>(model.position, 1.0);

    var vertPos4 = model_matrix * vec4<f32>(model.position, 1.0);
    out.position = vertPos4.xyz / vertPos4.w;
    //out.position = out.clip_position.xyz;

    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(1)
var s_diffuse: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let tex = textureLoad(t_diffuse, vec2<i32>(in.tex_coords), 0);
    let v = f32(tex.x) / 255.0;
    //return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
    return tex;
    //return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.9, 0.1, 0.9, 1.0);
}
