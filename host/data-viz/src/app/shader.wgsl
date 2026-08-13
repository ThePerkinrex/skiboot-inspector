// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    transform: mat4x4<f32>,
    normal: mat3x3<f32>,
};
@group(1) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) color: vec3<f32>,
    @location(10) normal_matrix_0: vec3<f32>,
    @location(11) normal_matrix_1: vec3<f32>,
    @location(12) normal_matrix_2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.color = instance.color;
    let world_position = instance_matrix * model_uniform.transform * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.normal = normal_matrix * model_uniform.normal * model.normal;
    return out;
}

// struct Light {
//     direction: vec3<f32>,
//     color: vec3<f32>,
// }

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_strength = 0.4;
    let light_color = vec3(1.0,1.0,1.0);
    let light_dir = normalize(vec3(0.1,0.8,0.0));

    let ambient_color = light_color * ambient_strength;
    let diffuse_strength = max(dot(in.normal, light_dir), 0.0);
    let diffuse_color = light_color * diffuse_strength;
    let result = (ambient_color + diffuse_color) * in.color; 

    return vec4<f32>(result, 1.0);
}