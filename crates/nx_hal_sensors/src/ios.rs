// Archivo: crates/nx_hal_sensors/src/ios.rs
#[cfg(target_os = "ios")]
pub mod ios_bridge {
    use crate::GLOBAL_IMU;
    use std::ffi::c_void;

    // Enlaces dinámicos a las APIs del SDK de iOS (CoreMotion y CoreHaptics)
    extern "C" {
        pub fn NativX_StartCoreMotion(callback: extern "C" fn(f32, f32, f32));
        pub fn NativX_TriggerHaptic(intensity: f32);
    }

    /// Callback exportado hacia Objective-C para recibir muestras de giro a 120Hz
    #[no_mangle]
    pub extern "C" fn __nativx_core_motion_callback(x: f32, y: f32, z: f32) {
        GLOBAL_IMU.write_hardware_sample(x, y, z);
    }

    pub fn init_ios_sensors() {
        unsafe {
            NativX_StartCoreMotion(__nativx_core_motion_callback);
        }
    }

    pub fn trigger_ios_haptic(intensity: f32) {
        unsafe {
            NativX_TriggerHaptic(intensity);
        }
    }
}
