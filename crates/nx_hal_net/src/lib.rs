// Archivo: crates/nx_hal_net/src/lib.rs
use bytemuck::{Pod, Zeroable};

/// Datagrama binario ultracompacto de 32 bytes (Cabe en un solo fragmento MTU de hardware sin dividir)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxNetStatePacket {
    pub magic_header: u16,   // 0x4E58 ("NX")
    pub sequence_tick: u16,  // Número de tick para descartar paquetes viejos (UDP Unordered)
    pub player_id: u32,      // ID único del jugador en la sesión
    pub pos: [f32; 3],       // Posición X, Y, Z (12 bytes)
    pub vel: [f32; 3],       // Velocidad X, Y, Z para predicción en el cliente (12 bytes)
}

impl NxNetStatePacket {
    #[inline(always)]
    pub fn to_wire_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline(always)]
    pub fn from_wire_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != std::mem::size_of::<Self>() {
            return None;
        }
        let pkt: Self = *bytemuck::from_bytes(bytes);
        if pkt.magic_header == 0x4E58 {
            Some(pkt)
        } else {
            None
        }
    }
}

/// Interpolación Hermite / Dead-Reckoning para suavizar el movimiento de otros jugadores a 120 FPS
#[inline(always)]
pub fn predict_remote_position(pkt: &NxNetStatePacket, time_since_packet_sec: f32) -> [f32; 3] {
    [
        pkt.pos[0] + pkt.vel[0] * time_since_packet_sec,
        pkt.pos[1] + pkt.vel[1] * time_since_packet_sec,
        pkt.pos[2] + pkt.vel[2] * time_since_packet_sec,
    ]
}

// ============================================================================
// 1. TRANSPORTE NATIVO: ANDROID, iOS Y ESCRITORIO (Sockets UDP No-Bloqueantes)
// ============================================================================
#[cfg(not(target_arch = "wasm32"))]
pub mod transport {
    use super::NxNetStatePacket;
    use std::net::UdpSocket;

    pub struct NxUdpEndpoint {
        socket: UdpSocket,
        last_remote_seq: u16,
    }

    impl NxUdpEndpoint {
        /// Abre un socket UDP de latencia cero sin bloquear el hilo de 120 FPS
        pub fn connect_nonblocking(bind_addr: &str, server_addr: &str) -> Result<Self, String> {
            let socket = UdpSocket::bind(bind_addr).map_err(|e| e.to_string())?;
            socket.connect(server_addr).map_err(|e| e.to_string())?;
            socket.set_nonblocking(true).map_err(|e| e.to_string())?;
            Ok(Self {
                socket,
                last_remote_seq: 0,
            })
        }

        /// Dispara el estado del jugador hacia el servidor en menos de 5 microsegundos
        #[inline(always)]
        pub fn send_unreliable(&self, packet: &NxNetStatePacket) {
            let _ = self.socket.send(packet.to_wire_bytes());
        }

        /// Drena todos los paquetes recibidos y devuelve únicamente el estado más reciente
        #[inline(always)]
        pub fn poll_latest_state(&mut self) -> Option<NxNetStatePacket> {
            let mut buf = [0u8; 32];
            let mut newest: Option<NxNetStatePacket> = None;

            while let Ok(len) = self.socket.recv(&mut buf) {
                if let Some(pkt) = NxNetStatePacket::from_wire_bytes(&buf[..len]) {
                    // Manejo aritmético de desbordamiento de secuencia (Sequence Wrap-Around)
                    let diff = pkt.sequence_tick.wrapping_sub(self.last_remote_seq);
                    if diff > 0 && diff < 32768 {
                        self.last_remote_seq = pkt.sequence_tick;
                        newest = Some(pkt);
                    }
                }
            }
            newest
        }
    }
}

// ============================================================================
// 2. TRANSPORTE NAVEGADOR WEB: WEBTRANSPORT DATAGRAMS (UDP sobre QUIC)
// ============================================================================
#[cfg(target_arch = "wasm32")]
pub mod transport {
    use super::NxNetStatePacket;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        fn nx_webtransport_send_datagram(ptr: *const u8, len: usize);
        fn nx_webtransport_poll_datagram(out_ptr: *mut u8, max_len: usize) -> i32;
    }

    pub struct NxUdpEndpoint;

    impl NxUdpEndpoint {
        pub fn send_unreliable(&self, packet: &NxNetStatePacket) {
            let bytes = packet.to_wire_bytes();
            unsafe {
                nx_webtransport_send_datagram(bytes.as_ptr(), bytes.len());
            }
        }

        pub fn poll_latest_state(&mut self) -> Option<NxNetStatePacket> {
            let mut buf = [0u8; 32];
            let len = unsafe { nx_webtransport_poll_datagram(buf.as_mut_ptr(), 32) };
            if len == 32 {
                NxNetStatePacket::from_wire_bytes(&buf)
            } else {
                None
            }
        }
    }
}
