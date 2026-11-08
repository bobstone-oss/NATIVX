pub mod skinning_compute;
// Archivo: crates/nx_hal_gpu/src/lib.rs
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;
use nx_hal_fs::MappedMeshView;

/// Extensión del Motor Gráfico NativX con Soporte 3D Real, Z-Buffer y MSDF Text
pub struct NxGpuEngine3D {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    // Pipelines gráficos separados para 3D (Mallas) y 2D/UI (SDF + Texto MSDF)
    pub pipeline_3d: wgpu::RenderPipeline,
    pub pipeline_ui: wgpu::RenderPipeline,

    // Textura de profundidad para oclusión geométrica 3D (Z-Buffer 24 bits)
    pub depth_texture_view: wgpu::TextureView,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

impl NxGpuEngine3D {
    pub async fn init(window: Arc<Window>, uniform_size: u64) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("GPU no compatible encontrada");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    label: Some("NativX 3D Device"),
                },
                None,
            )
            .await
            .expect("Error al abrir dispositivo gráfico 3D");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // 1. Creación del Depth Buffer (Z-Buffer de 32 bits en coma flotante)
        let depth_texture_view = Self::create_depth_texture(&device, &config);

        // 2. Buffer Uniforme Compartido CPU <-> GPU
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform State Buffer"),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("NativX Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 3. Shaders y Pipeline 3D (Con Depth Testing activado)
        let shader_3d = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader 3D Real"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh_3d.wgsl").into()),
        });

        let pipeline_3d = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline 3D"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_3d,
                entry_point: "vs_main",
                // Mapeo directo del struct NxVertex3D definido en nx_hal_fs
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<nx_hal_fs::NxVertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }, // Posición
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 }, // Normal
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 }, // UV
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_3d,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back), // Oculta triángulos traseros para optimizar rendimiento
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // Oclusión 3D estándar
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pipeline UI / MSDF Text (Se omite Depth Test para dibujar siempre sobre la escena)
        let shader_ui = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader UI & MSDF Text"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_msdf.wgsl").into()),
        });

        let pipeline_ui = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline UI MSDF"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_ui,
                entry_point: "ui_vertex_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_ui,
                entry_point: "ui_fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Transparencias suaves para texto
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            pipeline_3d,
            pipeline_ui,
            depth_texture_view,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        device.create_texture(&desc).create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Sube una malla `.nxmesh` leída por cero copias (`nx_hal_fs`) directamente a la VRAM de la GPU
    pub fn upload_mesh_buffers(&self, mesh: &MappedMeshView) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, mesh.indices.len() as u32)
    }
}
