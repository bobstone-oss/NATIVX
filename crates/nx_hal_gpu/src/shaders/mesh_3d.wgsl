// Archivo: crates/nx_hal_gpu/src/shaders/mesh_3d.wgsl

struct SceneUniforms {
    view_proj: mat4x4<f32>,   // Matriz de Cámara 3D (64 bytes)
    model_pos: vec3<f32>,     // Posición del objeto en el mundo
    energia: f32,             // Estado dinámico del jugador
    light_dir: vec3<f32>,     // Dirección de la luz solar/estelar
    _pad: f32,
};

@group(0) @binding(0) var<uniform> u_scene: SceneUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let world_pos = in.position + u_scene.model_pos;
    var out: VertexOutput;
    out.clip_pos = u_scene.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_normal = normalize(in.normal);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(u_scene.light_dir);
    // Iluminación difusa Lambert + Luz ambiental hemisférica
    let ndotl = max(dot(in.world_normal, light), 0.0);
    let ambient = 0.18;
    let lighting = ambient + ndotl * 0.82;

    // Color base reactivo a la energía del jugador
    let base_color = vec3<f32>(1.0 - u_scene.energia, u_scene.energia * 0.9, 1.0);
    return vec4<f32>(base_color * lighting, 1.0);
}
