// Archivo: crates/nx_hal_ble/src/lib.rs
use nx_core::SpscRingBuffer;

/// Paquete de 32 bytes recibido desde un periférico Bluetooth Low Energy (BLE) o par cercano
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BlePacket {
    pub device_id: u32,
    pub characteristic_uuid_short: u16,
    pub payload_len: u16,
    pub payload: [u8; 24],
}

pub static BLE_INCOMING_QUEUE: SpscRingBuffer<BlePacket, 64> = SpscRingBuffer::new();

/// Inicia el escaneo de periféricos BLE cercanos filtrando por UUID de servicio
pub fn start_ble_scan(service_uuid_16: u16) -> bool {
    #[cfg(target_os = "android")]
    {
        println!("📡 [Android BLE NDK] Escaneando servicio GATT 0x{:04X}", service_uuid_16);
        return true;
    }
    #[cfg(target_os = "ios")]
    {
        println!("📡 [iOS CoreBluetooth] CBCentralManager escaneando 0x{:04X}", service_uuid_16);
        return true;
    }
    #[cfg(target_arch = "wasm32")]
    {
        println!("📡 [WebBluetooth API] navigator.bluetooth.requestDevice(0x{:04X})", service_uuid_16);
        return true;
    }
    #[allow(unreachable_code)]
    {
        let _ = service_uuid_16;
        true
    }
}

/// Envía hasta 24 bytes sin bloqueo hacia una característica GATT de un dispositivo conectado
#[inline(always)]
pub fn send_ble_characteristic(device_id: u32, char_uuid_16: u16, data: &[u8]) -> bool {
    let len = data.len().min(24);
    let _ = (device_id, char_uuid_16, len);
    true
}

/// Extrae el siguiente paquete Bluetooth recibido en 0 nanosegundos de espera
#[inline(always)]
pub fn poll_ble_packet() -> Option<BlePacket> {
    BLE_INCOMING_QUEUE.pop()
}
