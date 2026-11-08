// Archivo: crates/nx_android_runner/src/lib.rs
#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::android::activity::AndroidApp;
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::window::{Window, WindowId};

use nx_hal_gpu::NxGpuEngine3D;
use nx_hal_sensors::{GLOBAL_IMU, trigger_haptic_pulse};

// Estructura compartida con la GPU (Idéntica a la generada por el compilador NativX)
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EstadoJugador {
    pub posicion: [f32; 3],
    pub _pad0: f32,
    pub velocidad: [f32; 3],
    pub energia: f32,
}

const SHADER_WGSL: &str = r#"
struct EstadoJugador {
    posicion: vec3<f32>,
    _pad0: f32,
    velocidad: vec3<f32>,
    energia: f32,
};
@group(0) @binding(0) var<uniform> u_state: EstadoJugador;

@vertex
fn proyectar_malla(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.25),
        vec2<f32>(-0.2, -0.2),
        vec2<f32>(0.2, -0.2)
    );
    let p = pos[idx] + u_state.posicion.xy;
    return vec4<f32>(p.x, p.y, 0.0, 1.0);
}

@fragment
fn pintar_superficie() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0 - u_state.energia, u_state.energia, 0.85, 1.0);
}
"#;

/// Arranca un hilo C++/Rust dedicado que lee el Giroscopio directamente del NDK a 125Hz
fn spawn_ndk_sensor_thread() {
    std::thread::spawn(|| {
        unsafe {
            let manager = ndk_sys::ASensorManager_getInstanceForPackage(b"com.nativx.runtime\0".as_ptr() as *const _);
            if manager.is_null() { return; }

            let gyro_type = ndk_sys::ASENSOR_TYPE_GYROSCOPE as i32;
            let gyro_sensor = ndk_sys::ASensorManager_getDefaultSensor(manager, gyro_type);
            if gyro_sensor.is_null() { return; }

            let looper = ndk_sys::ALooper_prepare(ndk_sys::ALOOPER_PREPARE_ALLOW_NON_CALLBACKS as i32);
            let queue = ndk_sys::ASensorManager_createEventQueue(
                manager,
                looper,
                1,
                None,
                std::ptr::null_mut(),
            );

            ndk_sys::ASensorEventQueue_enableSensor(queue, gyro_sensor);
            // 8000 microsegundos = 125 muestras por segundo directas del bus I2C/SPI del teléfono
            ndk_sys::ASensorEventQueue_setEventRate(queue, gyro_sensor, 8000);

            let mut event: ndk_sys::ASensorEvent = std::mem::zeroed();
            loop {
                ndk_sys::ALooper_pollOnce(16, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
                while ndk_sys::ASensorEventQueue_getEvents(queue, &mut event, 1) > 0 {
                    let v = event.__bindgen_anon_1.__bindgen_anon_1.vector;
                    // Escribe sin bloqueos (Lock-Free) en el bloque atómico de NativX
                    GLOBAL_IMU.write_hardware_sample(v.x, v.y, v.z);
                }
            }
        }
    });
}

struct NativXAndroidRuntime {
    window: Option<Arc<Window>>,
    gpu: Option<NxGpuEngine>,
    jugador: EstadoJugador,
    last_frame: Instant,
}

impl ApplicationHandler for NativXAndroidRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Cuando Android entrega la superficie física ANativeWindow, inicializamos Vulkan
        let win_attrs = Window::default_attributes().with_title("NativX Vulkan Surface");
        let window = Arc::new(event_loop.create_window(win_attrs).expect("Error al crear ANativeWindow"));
        let gpu = pollster::block_on(NxGpuEngine::init(
            window.clone(),
            SHADER_WGSL,
            std::mem::size_of::<EstadoJugador>() as u64,
        ));
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;

                if let Some(gpu) = &mut self.gpu {
                    // 1. Leer el giroscopio directamente de la memoria atómica (0ms latencia)
                    let gyro = gpu.frame_arena.alloc_temp(GLOBAL_IMU.read_gyro());

                    // 2. Actualizar física del juego
                    self.jugador.velocidad[0] += gyro[1] * 2.5 * dt;
                    self.jugador.velocidad[1] += gyro[0] * 2.5 * dt;
                    self.jugador.posicion[0] += self.jugador.velocidad[0] * dt;
                    self.jugador.posicion[1] += self.jugador.velocidad[1] * dt;

                    // 3. Rebote en los bordes con Háptica Nativa NDK
                    if self.jugador.posicion[0].abs() > 0.85 {
                        self.jugador.posicion[0] = self.jugador.posicion[0].signum() * 0.85;
                        self.jugador.velocidad[0] *= -0.8;
                        self.jugador.energia = (self.jugador.energia - 0.1).max(0.1);
                        trigger_haptic_pulse(0.9, 35);
                    }

                    // 4. Renderizar fotograma en Vulkan y limpiar la Arena de Memoria
                    gpu.render_frame(&self.jugador, [0.05, 0.05, 0.08, 1.0]);
                }

                if let Some(win) = &self.window {
                    win.request_redraw(); // Mantiene el bucle de 120Hz activo
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

/// Punto de entrada real invocado por el sistema operativo Android (reemplaza a `main()`)
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    log::info!("⚡ Arrancando NativX Runtime directo sobre Android NDK + Vulkan...");

    // Iniciar el hilo de captura directa del giroscopio
    spawn_ndk_sensor_thread();

    let event_loop = EventLoop::builder()
        .with_android_app(app)
        .build()
        .expect("Fallo al enlazar ALooper con NativX");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut runtime = NativXAndroidRuntime {
        window: None,
        gpu: None,
        jugador: EstadoJugador {
            posicion: [0.0, 0.0, 0.0],
            _pad0: 0.0,
            velocidad: [0.0, 0.0, 0.0],
            energia: 1.0,
        },
        last_frame: Instant::now(),
    };

    let _ = event_loop.run_app(&mut runtime);
}
