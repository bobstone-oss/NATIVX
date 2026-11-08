use std::sync::atomic::{AtomicU32, Ordering};

pub struct ImuSharedRing {
    gx: AtomicU32,
    gy: AtomicU32,
    gz: AtomicU32,
}

pub static GLOBAL_IMU: ImuSharedRing = ImuSharedRing {
    gx: AtomicU32::new(0),
    gy: AtomicU32::new(0),
    gz: AtomicU32::new(0),
};

impl ImuSharedRing {
    #[inline(always)]
    pub fn write_hardware_sample(&self, x: f32, y: f32, z: f32) {
        self.gx.store(x.to_bits(), Ordering::Relaxed);
        self.gy.store(y.to_bits(), Ordering::Relaxed);
        self.gz.store(z.to_bits(), Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn read_gyro(&self) -> [f32; 3] {
        [
            f32::from_bits(self.gx.load(Ordering::Relaxed)),
            f32::from_bits(self.gy.load(Ordering::Relaxed)),
            f32::from_bits(self.gz.load(Ordering::Relaxed)),
        ]
    }
}

pub fn trigger_haptic_pulse(_intensity: f32, _duration_ms: u32) {}

pub mod lifecycle;
pub mod wasm_bridge;
#[cfg(target_os = "ios")]
pub mod ios;
