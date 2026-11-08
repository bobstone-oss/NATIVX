// Archivo: crates/nx_core/src/stdlib.rs
use crate::vec2::Vec2;
use crate::vec3::Vec3;
use bytemuck::{Pod, Zeroable};

/// Generador Pseudoaleatorio Determinista XorShift128+ (Idéntico bit a bit en ARM64, x86_64 y Wasm32)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct NxRng {
    state0: u64,
    state1: u64,
}

impl Default for NxRng {
    fn default() -> Self {
        Self::from_seed(0x853c49e6748fea9b)
    }
}

impl NxRng {
    #[inline(always)]
    pub const fn from_seed(seed: u64) -> Self {
        let s0 = if seed == 0 { 0xdeadbeefcafebabe } else { seed };
        let s1 = s0.wrapping_mul(0x9e3779b97f4a7c15);
        Self { state0: s0, state1: s1 }
    }

    /// Genera el siguiente entero de 64 bits en 3 ciclos de reloj
    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        let mut s1 = self.state0;
        let s0 = self.state1;
        let result = s0.wrapping_add(s1);
        self.state0 = s0;
        s1 ^= s1 << 23;
        self.state1 = s1 ^ s0 ^ (s1 >> 18) ^ (s0 >> 5);
        result
    }

    #[inline(always)]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Devuelve un flotante uniforme en el rango `[0.0, 1.0)`
    #[inline(always)]
    pub fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u32() >> 8) | 0x3f800000;
        f32::from_bits(bits) - 1.0
    }

    /// Devuelve un flotante en el rango `[min, max)`
    #[inline(always)]
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    /// Devuelve un entero en el rango `[min, max]` inclusivo
    #[inline(always)]
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as u32;
        min + (self.next_u32() % span) as i32
    }

    /// Genera un vector 2D aleatorio dentro del círculo unitario (para dispersión de disparos/partículas)
    #[inline(always)]
    pub fn inside_unit_circle(&mut self) -> Vec2 {
        let angle = self.range_f32(0.0, core::f32::consts::TAU);
        let radius = self.next_f32().sqrt();
        let (s, c) = angle.sin_cos();
        Vec2::new(c * radius, s * radius)
    }

    /// Genera una dirección 3D unitaria uniformemente distribuida en la esfera
    #[inline(always)]
    pub fn unit_sphere_dir(&mut self) -> Vec3 {
        let z = self.range_f32(-1.0, 1.0);
        let theta = self.range_f32(0.0, core::f32::consts::TAU);
        let r = (1.0 - z * z).max(0.0).sqrt();
        let (s, c) = theta.sin_cos();
        Vec3::new(r * c, r * s, z)
    }
}

/// Temporizador sin asignaciones de memoria para enfriamientos (Cooldowns) y eventos repetitivos
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct NxTimer {
    pub elapsed: f32,
    pub duration: f32,
    pub repeating: u32,
    pub finished: u32,
}

impl NxTimer {
    #[inline(always)]
    pub const fn new(duration_secs: f32, repeating: bool) -> Self {
        Self {
            elapsed: 0.0,
            duration: duration_secs,
            repeating: if repeating { 1 } else { 0 },
            finished: 0,
        }
    }

    /// Avanza el temporizador con `dt` y devuelve `true` en el fotograma exacto en que se dispara
    #[inline(always)]
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.finished != 0 && self.repeating == 0 {
            return false;
        }
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            if self.repeating != 0 {
                self.elapsed %= self.duration.max(1e-6);
            } else {
                self.elapsed = self.duration;
                self.finished = 1;
            }
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn progress_0_to_1(&self) -> f32 {
        (self.elapsed / self.duration.max(1e-6)).clamp(0.0, 1.0)
    }
}

/// Curvas matemáticas de animación (Easings) para UI y cámaras
pub mod easing {
    #[inline(always)]
    pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    #[inline(always)]
    pub fn ease_out_cubic(t: f32) -> f32 {
        let inv = 1.0 - t.clamp(0.0, 1.0);
        1.0 - inv * inv * inv
    }

    #[inline(always)]
    pub fn ease_in_out_quad(t: f32) -> f32 {
        let x = t.clamp(0.0, 1.0);
        if x < 0.5 {
            2.0 * x * x
        } else {
            let v = -2.0 * x + 2.0;
            1.0 - (v * v) * 0.5
        }
    }
}
