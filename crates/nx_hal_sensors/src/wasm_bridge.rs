// Archivo: crates/nx_hal_sensors/src/wasm_bridge.rs
// Expone un bloque fijo de 16 floats (64 bytes) en la RAM lineal del binario .wasm

#[repr(C, align(64))]
pub struct WasmZeroCopyBlock {
    pub data: [f32; 16],
}

pub static mut ZERO_COPY_SENSORS: WasmZeroCopyBlock = WasmZeroCopyBlock {
    data: [0.0; 16],
};

extern "C" {
    /// Importado directamente desde `nx_loader.js`
    pub fn nx_haptic_pulse(intensity: f32, duration_ms: u32);
    pub fn nx_fs_write_fast(name_ptr: *const u8, name_len: usize, data_ptr: *const u8, data_len: usize);
}

#[no_mangle]
pub extern "C" fn nx_get_sensor_buffer_ptr() -> *mut f32 {
    unsafe { ZERO_COPY_SENSORS.data.as_mut_ptr() }
}

#[no_mangle]
pub extern "C" fn nx_init(width: u32, height: u32) {
    // Inicializa la superficie WebGPU con las dimensiones físicas del Canvas
    let _ = (width, height);
}

#[no_mangle]
pub extern "C" fn nx_frame(_dt: f32) {
    // En cada fotograma, NativX lee directamente los sensores desde ZERO_COPY_SENSORS
    // sin ninguna llamada de cruce JS -> Wasm:
    unsafe {
        let gyro_x = ZERO_COPY_SENSORS.data[0];
        let stick_x = ZERO_COPY_SENSORS.data[6];

        // Si el jugador choca en el borde, disparamos el pulso háptico nativo
        if (gyro_x + stick_x).abs() > 0.95 {
            nx_haptic_pulse(0.85, 35);
        }
    }
}
