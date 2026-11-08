use bytemuck::{Pod, Zeroable};
use nx_core::{Camera3D, FrameArena, Mat4, Vec3};
use nx_ecs::{extract_render_batch, EntityId, GpuRenderInstance, World};
use nx_hal_fs::NxVertex3D;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod nx_compiled_script {
    #![allow(dead_code, unused_variables, unused_imports, unused_mut)]
    include!(concat!(env!("OUT_DIR"), "/generated_nx_game.rs"));
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraGlobals {
    view_proj: Mat4,
    light_dir: Vec3,
    _pad: f32,
}

const SHADER_WGSL: &str = r#"
struct CameraGlobals { view_proj: mat4x4<f32>, light_dir: vec3<f32>, _pad: f32 };
struct GpuRenderInstance { world_matrix: mat4x4<f32>, color: vec4<f32> };
@group(0) @binding(0) var<uniform> u_cam: CameraGlobals;
@group(0) @binding(1) var<storage, read> instances: array<GpuRenderInstance>;

struct VertexInput { @location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32> };
struct VertexOutput { @builtin(position) clip_pos: vec4<f32>, @location(0) norm: vec3<f32>, @location(1) color: vec4<f32> };

@vertex
fn vs_main(in: VertexInput, @builtin(instance_index) idx: u32) -> VertexOutput {
    let inst = instances[idx];
    let wp = inst.world_matrix * vec4<f32>(in.pos, 1.0);
    let wn = (inst.world_matrix * vec4<f32>(in.normal, 0.0)).xyz;
    var out: VertexOutput;
    out.clip_pos = u_cam.view_proj * wp;
    out.norm = normalize(wn);
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let diff = max(dot(in.norm, normalize(u_cam.light_dir)), 0.0);
    return vec4<f32>(in.color.rgb * (0.25 + diff * 0.75), 1.0);
}
"#;

fn cube_mesh(s: f32) -> (Vec<NxVertex3D>, Vec<u32>) {
    let v = vec![
        NxVertex3D { position: [-s, -s,  s], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
        NxVertex3D { position: [ s, -s,  s], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
        NxVertex3D { position: [ s,  s,  s], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
        NxVertex3D { position: [-s,  s,  s], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
        NxVertex3D { position: [ s, -s, -s], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
        NxVertex3D { position: [-s, -s, -s], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
        NxVertex3D { position: [-s,  s, -s], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
        NxVertex3D { position: [ s,  s, -s], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
    ];
    let idx = vec![
        0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7,
        3, 2, 7, 3, 7, 6, 5, 4, 1, 5, 1, 0,
        1, 4, 7, 1, 7, 2, 5, 0, 3, 5, 3, 6,
    ];
    (v, idx)
}

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    pipeline: Option<wgpu::RenderPipeline>,
    cam_buf: Option<wgpu::Buffer>,
    inst_buf: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    vbo: Option<wgpu::Buffer>,
    ibo: Option<wgpu::Buffer>,
    idx_count: u32,
    world: World,
    arena: FrameArena,
    camera: Camera3D,
    mothership: EntityId,
    input: Vec3,
    last: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let win = Arc::new(event_loop.create_window(
            Window::default_attributes().with_title("⚡ NativX 3D Runtime (Usa WASD / Flechas)")
        ).unwrap());
        let size = win.inner_size();
        let inst = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surf = inst.create_surface(win.clone()).unwrap();
        let adapter = pollster::block_on(inst.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surf), ..Default::default()
        })).unwrap();
        let (dev, q) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();
        let caps = surf.get_capabilities(&adapter);
        let cfg = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format: caps.formats[0],
            width: size.width.max(1), height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo, alpha_mode: caps.alpha_modes[0],
            view_formats: vec![], desired_maximum_frame_latency: 1,
        };
        surf.configure(&dev, &cfg);

        let cam_buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: None, size: std::mem::size_of::<CameraGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        let inst_buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: None, size: (std::mem::size_of::<GpuRenderInstance>() * 512) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None,
                },
            ],
        });
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None, layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: inst_buf.as_entire_binding() },
            ],
        });
        let shader = dev.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None, source: wgpu::ShaderSource::Wgsl(SHADER_WGSL.into()),
        });
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&bgl], push_constant_ranges: &[],
        });
        let pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<NxVertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { format: cfg.format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None,
            multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let (verts, idxs) = cube_mesh(0.65);
        let vbo = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(&verts), usage: wgpu::BufferUsages::VERTEX,
        });
        let ibo = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(&idxs), usage: wgpu::BufferUsages::INDEX,
        });

        self.window = Some(win); self.surface = Some(surf); self.device = Some(dev);
        self.queue = Some(q); self.config = Some(cfg); self.pipeline = Some(pipe);
        self.cam_buf = Some(cam_buf); self.inst_buf = Some(inst_buf); self.bind_group = Some(bg);
        self.vbo = Some(vbo); self.ibo = Some(ibo); self.idx_count = idxs.len() as u32;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let v = if event.state == ElementState::Pressed { 1.0 } else { 0.0 };
                if let PhysicalKey::Code(c) = event.physical_key {
                    match c {
                        KeyCode::KeyW | KeyCode::ArrowUp => self.input.y = v,
                        KeyCode::KeyS | KeyCode::ArrowDown => self.input.y = -v,
                        KeyCode::KeyD | KeyCode::ArrowRight => self.input.x = v,
                        KeyCode::KeyA | KeyCode::ArrowLeft => self.input.x = -v,
                        KeyCode::Escape => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().clamp(0.001, 0.05);
                self.last = now;

                nx_compiled_script::actualizar_universo_ecs(
                    &mut self.arena, &mut self.world, self.mothership, self.input, dt
                );
                let batch = extract_render_batch(&self.world, &mut self.arena);

                if let (Some(surf), Some(dev), Some(q), Some(pipe), Some(bg), Some(cb), Some(ib), Some(vbo), Some(ibo)) = (
                    &self.surface, &self.device, &self.queue, &self.pipeline, &self.bind_group,
                    &self.cam_buf, &self.inst_buf, &self.vbo, &self.ibo,
                ) {
                    let cg = CameraGlobals {
                        view_proj: self.camera.view_projection_matrix(),
                        light_dir: Vec3::new(0.4, 0.8, 0.6).normalize_or_zero(), _pad: 0.0,
                    };
                    q.write_buffer(cb, 0, bytemuck::bytes_of(&cg));
                    q.write_buffer(ib, 0, bytemuck::cast_slice(batch));

                    if let Ok(frame) = surf.get_current_texture() {
                        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let mut enc = dev.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                        {
                            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: None,
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view, resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.03, g: 0.03, b: 0.06, a: 1.0 }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None, occlusion_query_set: None, timestamp_writes: None,
                            });
                            pass.set_pipeline(pipe);
                            pass.set_bind_group(0, bg, &[]);
                            pass.set_vertex_buffer(0, vbo.slice(..));
                            pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint32);
                            pass.draw_indexed(0..self.idx_count, 0, 0..(batch.len() as u32));
                        }
                        q.submit(std::iter::once(enc.finish()));
                        frame.present();
                    }
                }
                self.arena.reset();
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

fn main() {
    let mut world = World::with_capacity(512);
    let mut arena = FrameArena::with_capacity(1024 * 1024);
    let mothership = nx_compiled_script::poblar_universo_3d(&mut arena, &mut world);
    arena.reset();

    let el = EventLoop::new().unwrap();
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None, surface: None, device: None, queue: None, config: None,
        pipeline: None, cam_buf: None, inst_buf: None, bind_group: None,
        vbo: None, ibo: None, idx_count: 0, world, arena,
        camera: Camera3D { eye: Vec3::new(0.0, 2.0, 16.0), ..Default::default() },
        mothership, input: Vec3::ZERO, last: Instant::now(),
    };
    let _ = el.run_app(&mut app);
}
