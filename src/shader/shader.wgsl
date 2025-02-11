
// Vertex shader

//struct Camera {
//    view_proj: mat4x4<f32>,
//}
//@group(1) @binding(0)
//var<uniform> camera: Camera;

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
    var light = vec3<f32>(1000., 1000., 2000.);

    var light_dir = normalize(light - in.position);

    var material = vec3<f32>(0.9, 0.4, 0.3);
    var diffuse = clamp(dot(in.normal, light_dir), 0., 1.);

    var view_dir = normalize(-in.position);
    var half_dir = normalize(light_dir + view_dir);

    var specAngle = max(dot(half_dir, in.normal), 0.0);
    var shininess = 4.0;
    var specular = pow(specAngle, shininess);

    var ambient = 0.4 * vec3<f32>(0.9, 0.9, 0.8);

    var color = (diffuse * material) + (ambient * material) + specular * vec3<f32>(1.0, 1.0, 0.9) * .1;

    return vec4<f32>(color, 1.0);
}
