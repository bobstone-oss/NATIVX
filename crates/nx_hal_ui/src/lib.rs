// Archivo: crates/nx_hal_ui/src/lib.rs
use bytemuck::{Pod, Zeroable};

/// Cada elemento de UI (Botón, Barra de Energía, Joystick Táctil) ocupa solo 48 bytes en la GPU
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxUiPrimitive {
    pub rect: [f32; 4],         // [x, y, ancho, alto] en coordenadas normalizadas (0.0 a 1.0)
    pub color: [f32; 4],        // [R, G, B, A]
    pub border_color: [f32; 4], // Color del borde brillante
    pub params: [f32; 4],       // [radio_esquina, grosor_borde, suavizado_sdf, progreso_barra]
}

/// Contexto de UI de Modo Inmediato (Se construye en la FrameArena en cada fotograma)
pub struct NxUiContext {
    pub primitives: Vec<NxUiPrimitive>,
    pub pointer_pos: [f32; 2], // Coordenada actual del dedo o ratón (0.0 a 1.0)
    pub pointer_down: bool,
}

impl NxUiContext {
    pub fn new(max_elements: usize) -> Self {
        Self {
            primitives: Vec::with_capacity(max_elements),
            pointer_pos: [0.0, 0.0],
            pointer_down: false,
        }
    }

    #[inline(always)]
    pub fn begin_frame(&mut self, touch_x: f32, touch_y: f32, is_pressed: bool) {
        self.primitives.clear();
        self.pointer_pos = [touch_x, touch_y];
        self.pointer_down = is_pressed;
    }

    /// Dibuja una barra de energía o salud con bordes redondeados SDF en 1 línea
    pub fn progress_bar(&mut self, x: f32, y: f32, w: f32, h: f32, value_0_to_1: f32, color: [f32; 4]) {
        self.primitives.push(NxUiPrimitive {
            rect: [x, y, w, h],
            color,
            border_color: [1.0, 1.0, 1.0, 0.35],
            params: [h * 0.5, 0.003, 0.002, value_0_to_1.clamp(0.0, 1.0)],
        });
    }

    /// Dibuja un botón interactivo y devuelve `true` en el instante en que el jugador lo toca
    pub fn button(&mut self, x: f32, y: f32, w: f32, h: f32, base_color: [f32; 4]) -> bool {
        let px = self.pointer_pos[0];
        let py = self.pointer_pos[1];

        let hovered = px >= x && px <= (x + w) && py >= y && py <= (y + h);
        let clicked = hovered && self.pointer_down;

        // Ilumina el botón automáticamente al pasar el dedo o hacer clic
        let final_color = if clicked {
            [1.0, 1.0, 1.0, 1.0]
        } else if hovered {
            [base_color[0] * 1.25, base_color[1] * 1.25, base_color[2] * 1.25, base_color[3]]
        } else {
            base_color
        };

        self.primitives.push(NxUiPrimitive {
            rect: [x, y, w, h],
            color: final_color,
            border_color: [0.0, 0.94, 1.0, 0.9], // Borde cian NativX
            params: [0.02, 0.004, 0.002, 1.0],
        });

        clicked
    }

    /// Devuelve el slice binario exacto para subirlo a la GPU con `queue.write_buffer`
    #[inline(always)]
    pub fn as_gpu_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.primitives)
    }
}
