
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
@group(0)@binding(2)
var normal_map: texture_2d<f32>;
@group(0)@binding(3)
var s_normal: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let z = textureSample(t_diffuse, s_diffuse, in.tex_coords).x;
    let n: vec4<f32> = textureSample(normal_map, s_normal, in.tex_coords);
    let tan_norm = vec3<f32>(2. * n.xyz - 1.0);


    var c = vec4<f32>(0.2, 0.2, 0.5, 1);
    c = mix(c, vec4<f32>(0.5, 0.5, 0.8, 1), smoothstep(0.0, 0.125, z));
    c = mix(c, vec4<f32>(0.3, 0.5, 0.3, 1), smoothstep(0.125, 0.25, z));
    c = mix(c, vec4<f32>(0.9, 0.8, 0.6, 1), smoothstep(0.500, 0.625, z));
    c = mix(c, vec4<f32>(0.9, 0.7, 0.4, 1), smoothstep(0.625, 0.750, z));
    c = mix(c, vec4<f32>(0.7, 0.6, 0.5, 1), smoothstep(0.750, 0.875, z));
    c = mix(c, vec4<f32>(1, 1, 1, 1), smoothstep(0.875, 1., z));

    let light_dir = normalize(vec3<f32>(0., 1000., 10000.));
    let diffuse = max(dot(tan_norm, light_dir), 0.);

    let ambient = vec4<f32>(1.0, 1.0, 1.0, 1.0) * 0.3;

    return c * diffuse + c * ambient;
    //return vec4<f32>((n.xy + 1.0) / 2.0, n.z, 1.0);
    //return tan_norm;
}

@fragment
fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.9, 0.1, 0.9, 1.0);
}
