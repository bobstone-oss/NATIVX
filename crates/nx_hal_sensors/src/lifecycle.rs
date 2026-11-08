// Archivo: crates/nx_hal_sensors/src/lifecycle.rs
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLifecycleState {
    Active = 0,          // En primer plano corriendo a 120Hz
    Paused = 1,          // Ventana sin foco o notificación desplegada
    Suspended = 2,       // Minimizado en segundo plano (GPU y Audio detenidos)
    SurfaceLost = 3,     // Rotación de pantalla o teléfono plegable cambiando resolución
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalStatus {
    Nominal = 0,         // Temperatura fría/normal -> 120 FPS permitidos
    Fair = 1,            // Tibio -> 120 FPS permitidos
    Serious = 2,         // Caliente -> Bajar automáticamente a 60 FPS y reducir partículas
    Critical = 3,        // Muy caliente -> Modo ahorro de emergencia (30 FPS)
}

pub struct DeviceTelemetry {
    lifecycle_state: AtomicU8,
    thermal_status: AtomicU8,
    battery_percent: AtomicU8,
    battery_charging: AtomicU8,
    target_fps: AtomicU32,
}

pub static DEVICE_TELEMETRY: DeviceTelemetry = DeviceTelemetry {
    lifecycle_state: AtomicU8::new(AppLifecycleState::Active as u8),
    thermal_status: AtomicU8::new(ThermalStatus::Nominal as u8),
    battery_percent: AtomicU8::new(100),
    battery_charging: AtomicU8::new(1),
    target_fps: AtomicU32::new(120),
};

impl DeviceTelemetry {
    /// Llamado por el Sistema Operativo (Android NDK `onPause`, iOS `willResignActive`, Web `visibilitychange`)
    #[inline(always)]
    pub fn notify_lifecycle_change(&self, new_state: AppLifecycleState) {
        self.lifecycle_state.store(new_state as u8, Ordering::SeqCst);
    }

    /// Llamado por el monitor térmico del SO (`ADeviceThermalManager` en Android / `NSProcessInfo.thermalState` en iOS)
    #[inline(always)]
    pub fn notify_thermal_change(&self, status: ThermalStatus) {
        self.thermal_status.store(status as u8, Ordering::SeqCst);
        // Ajuste adaptativo automático de fotogramas por segundo para proteger la batería y el silicio
        let recommended_fps = match status {
            ThermalStatus::Nominal | ThermalStatus::Fair => 120,
            ThermalStatus::Serious => 60,
            ThermalStatus::Critical => 30,
        };
        self.target_fps.store(recommended_fps, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn notify_battery_status(&self, percent: u8, is_charging: bool) {
        self.battery_percent.store(percent.min(100), Ordering::Relaxed);
        self.battery_charging.store(u8::from(is_charging), Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn current_lifecycle(&self) -> AppLifecycleState {
        match self.lifecycle_state.load(Ordering::Acquire) {
            0 => AppLifecycleState::Active,
            1 => AppLifecycleState::Paused,
            2 => AppLifecycleState::Suspended,
            _ => AppLifecycleState::SurfaceLost,
        }
    }

    #[inline(always)]
    pub fn recommended_fps(&self) -> u32 {
        self.target_fps.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn battery_level(&self) -> u8 {
        self.battery_percent.load(Ordering::Relaxed)
    }
}

/// Abre o cierra el teclado virtual nativo del teléfono (Android `ANativeActivity_showSoftInput` / iOS `UITextInput`)
pub fn set_virtual_keyboard_visible(visible: bool) {
    #[cfg(target_os = "android")]
    {
        println!("⌨️ [Android NDK SoftInput] Teclado virtual: {}", visible);
    }
    #[cfg(target_os = "ios")]
    {
        println!("⌨️ [iOS UIKeyInput] Teclado virtual: {}", visible);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = visible;
    }
}
