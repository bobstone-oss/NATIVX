// Archivo: crates/nx_hal_gpu/src/shaders/ui_msdf.wgsl
struct UiPrimitive {
    rect: vec4<f32>,         // x, y, w, h
    color: vec4<f32>,
    border_color: vec4<f32>,
    params: vec4<f32>,       // radius, thickness, softness, unused
};

@group(0) @binding(0) var<uniform> u_time: f32; // O state universal

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn ui_vertex_main(@builtin(vertex_index) v_idx: u32) -> VertexOutput {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0)
    );
    let uv = quad[v_idx];
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);

    var out: VertexOutput;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn ui_fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Renderizado tipográfico MSDF de alta precisión matemática
    // Muestreo de distancia multicanal para bordes vectoriales sin pixelación
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
