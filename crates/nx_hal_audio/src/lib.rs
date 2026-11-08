// Archivo: crates/nx_hal_audio/src/lib.rs
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const SAMPLE_RATE: usize = 48_000;
pub const MAX_VOICES: usize = 32;

/// Estado atómico del oyente (Cámara / Jugador) actualizado a 120Hz desde el juego
pub struct AtomicListener {
    pub pos_x: AtomicU32,
    pub pos_y: AtomicU32,
    pub pos_z: AtomicU32,
    pub right_x: AtomicU32, // Vector derecho de la cabeza para paneo binaural
    pub right_y: AtomicU32,
    pub right_z: AtomicU32,
}

pub static LISTENER_3D: AtomicListener = AtomicListener {
    pos_x: AtomicU32::new(0),
    pos_y: AtomicU32::new(0),
    pos_z: AtomicU32::new(0),
    right_x: AtomicU32::new(0x3f800000), // 1.0f32 en IEEE 754
    right_y: AtomicU32::new(0),
    right_z: AtomicU32::new(0),
};

impl AtomicListener {
    #[inline(always)]
    pub fn update_pose(&self, pos: [f32; 3], right_vec: [f32; 3]) {
        self.pos_x.store(pos[0].to_bits(), Ordering::Relaxed);
        self.pos_y.store(pos[1].to_bits(), Ordering::Relaxed);
        self.pos_z.store(pos[2].to_bits(), Ordering::Relaxed);
        self.right_x.store(right_vec[0].to_bits(), Ordering::Relaxed);
        self.right_y.store(right_vec[1].to_bits(), Ordering::Relaxed);
        self.right_z.store(right_vec[2].to_bits(), Ordering::Relaxed);
    }
}

/// Emisor de sonido 3D individual en el mezclador de hardware
pub struct SpatialVoice {
    pub active: AtomicBool,
    pub position: [f32; 3],
    pub samples: &'static [f32],
    pub cursor: usize,
    pub loop_sound: bool,
}

/// Calcula las ganancias Izquierda/Derecha (HRTF simplificado + Ley Inversa del Cuadrado)
#[inline(always)]
pub fn compute_spatial_gains(source_pos: [f32; 3]) -> (f32, f32) {
    let lx = f32::from_bits(LISTENER_3D.pos_x.load(Ordering::Relaxed));
    let ly = f32::from_bits(LISTENER_3D.pos_y.load(Ordering::Relaxed));
    let lz = f32::from_bits(LISTENER_3D.pos_z.load(Ordering::Relaxed));

    let rx = f32::from_bits(LISTENER_3D.right_x.load(Ordering::Relaxed));
    let ry = f32::from_bits(LISTENER_3D.right_y.load(Ordering::Relaxed));
    let rz = f32::from_bits(LISTENER_3D.right_z.load(Ordering::Relaxed));

    let dx = source_pos[0] - lx;
    let dy = source_pos[1] - ly;
    let dz = source_pos[2] - lz;

    let dist_sq = dx * dx + dy * dy + dz * dz;
    let dist = dist_sq.sqrt().max(0.001);

    // Atenuación física suave por distancia
    let attenuation = (1.0 / (1.0 + 0.15 * dist_sq)).clamp(0.0, 1.0);

    // Producto punto con el vector derecho del oído (-1.0 = Izquierda total, +1.0 = Derecha total)
    let pan = ((dx * rx + dy * ry + dz * rz) / dist).clamp(-1.0, 1.0);

    // Paneo de potencia constante (Constant Power Panning)
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    let gain_left = angle.cos() * attenuation;
    let gain_right = angle.sin() * attenuation;

    (gain_left, gain_right)
}

/// Rellena el buffer estéreo intercalado (L, R, L, R...) directamente para el chip de audio
#[no_mangle]
pub extern "C" fn nx_audio_mix_stereo_block(
    out_ptr: *mut f32,
    frames: usize,
    source_x: f32,
    source_y: f32,
    source_z: f32,
    phase: &mut f32,
) {
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_ptr, frames * 2) };
    let (gain_l, gain_r) = compute_spatial_gains([source_x, source_y, source_z]);

    // Ejemplo de sintetizador/motor espacial en tiempo real (cero asignaciones de memoria)
    let freq = 220.0;
    let step = (freq * 2.0 * std::f32::consts::PI) / (SAMPLE_RATE as f32);

    for frame in 0..frames {
        let sample = (*phase).sin() * 0.3;
        *phase = (*phase + step) % (2.0 * std::f32::consts::PI);

        out_slice[frame * 2] = sample * gain_l;     // Canal Izquierdo
        out_slice[frame * 2 + 1] = sample * gain_r; // Canal Derecho
    }
}
