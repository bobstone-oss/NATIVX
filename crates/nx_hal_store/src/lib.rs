// Archivo: crates/nx_hal_store/src/lib.rs
use nx_core::SpscRingBuffer;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PurchaseStatus {
    #[default]
    Pending = 0,
    Succeeded = 1,
    CancelledByUser = 2,
    NetworkError = 3,
    AlreadyOwned = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PurchaseReceipt {
    pub sku_hash: u64,
    pub status: PurchaseStatus,
    pub timestamp_utc: u64,
    pub signature_token: [u8; 32], // Hash criptográfico del recibo para validar en `nx_server`
}

pub static STORE_RECEIPT_QUEUE: SpscRingBuffer<PurchaseReceipt, 16> = SpscRingBuffer::new();

/// Dispara el flujo nativo de compra (Google Play / Apple FaceID StoreKit 2 / Web Payment)
pub fn request_in_app_purchase(sku_id: &str, sku_hash: u64) {
    #[cfg(target_os = "android")]
    {
        println!("💳 [Google Play Billing] Iniciando flujo nativo para SKU '{}'", sku_id);
    }
    #[cfg(target_os = "ios")]
    {
        println!("💳 [Apple StoreKit 2] Solicitando autorización FaceID/TouchID para '{}'", sku_id);
    }
    #[cfg(target_arch = "wasm32")]
    {
        println!("💳 [Web Payment Request API] Abriendo pasarela para '{}'", sku_id);
    }

    // En entorno de desarrollo local, simula la confirmación firmada en la cola libre de bloqueos
    let _ = STORE_RECEIPT_QUEUE.push(PurchaseReceipt {
        sku_hash,
        status: PurchaseStatus::Succeeded,
        timestamp_utc: 1_762_500_000,
        signature_token: [0xAA; 32],
    });
}

/// Consulta si el sistema operativo ya confirmó una compra en este fotograma (0ms bloqueo)
#[inline(always)]
pub fn poll_purchase_receipt() -> Option<PurchaseReceipt> {
    STORE_RECEIPT_QUEUE.pop()
}
