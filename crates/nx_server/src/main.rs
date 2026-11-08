// Archivo: crates/nx_server/src/main.rs
use dashmap::DashMap;
use nx_hal_net::NxNetStatePacket;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use wtransport::endpoint::IncomingSession;
use wtransport::{Endpoint, Identity, ServerConfig};

const SERVER_TICK_HZ: u64 = 60; // 60 actualizaciones físicas globales por segundo
const MAX_SPEED_UNITS_PER_SEC: f32 = 35.0; // Protección Anti-Cheat de velocidad máxima

#[derive(Clone)]
pub struct ConnectedPlayer {
    pub last_packet: NxNetStatePacket,
    pub last_seen: Instant,
    pub udp_addr: Option<SocketAddr>, // Si entró desde APK (Android) o IPA (iOS)
}

pub struct WorldServer {
    pub players: DashMap<u32, ConnectedPlayer>,
}

impl WorldServer {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            players: DashMap::new(),
        })
    }

    /// Valida el paquete entrante (Anti-Cheat básico) y actualiza la memoria global del mundo
    pub fn process_incoming_packet(
        &self,
        pkt: NxNetStatePacket,
        udp_origin: Option<SocketAddr>,
    ) -> bool {
        let now = Instant::now();

        if let Some(mut existing) = self.players.get_mut(&pkt.player_id) {
            let dt = (now - existing.last_seen).as_secs_f32().max(0.001);
            let dx = pkt.pos[0] - existing.last_packet.pos[0];
            let dy = pkt.pos[1] - existing.last_packet.pos[1];
            let dz = pkt.pos[2] - existing.last_packet.pos[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            // Verificación autoritativa: rechaza teletransportaciones ilegales (Speedhack)
            if (dist / dt) > MAX_SPEED_UNITS_PER_SEC * 1.5 {
                return false;
            }

            existing.last_packet = pkt;
            existing.last_seen = now;
            if udp_origin.is_some() {
                existing.udp_addr = udp_origin;
            }
        } else {
            println!("🎮 [Cross-Play] Nuevo jugador conectado (ID: {})", pkt.player_id);
            self.players.insert(
                pkt.player_id,
                ConnectedPlayer {
                    last_packet: pkt,
                    last_seen: now,
                    udp_addr: udp_origin,
                },
            );
        }
        true
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let world = WorldServer::new();
    println!("🚀 [NativX Authoritative Server] Iniciando motor Cross-Play (UDP + WebTransport)...");

    // ========================================================================
    // 1. CANAL NATIVO: SOCKET UDP PURO (Para Android APK, iPhone IPA y PC)
    // ========================================================================
    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:4432").await?);
    println!("⚡ Bus UDP Nativo escuchando en 0.0.0.0:4432");

    let world_udp = world.clone();
    let udp_recv = udp_socket.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 64];
        while let Ok((len, src_addr)) = udp_recv.recv_from(&mut buf).await {
            if let Some(pkt) = NxNetStatePacket::from_wire_bytes(&buf[..len]) {
                world_udp.process_incoming_packet(pkt, Some(src_addr));
            }
        }
    });

    // Bucle de Broadcast Autoritativo a 60Hz para todos los clientes UDP
    let world_tick = world.clone();
    let udp_send = udp_socket.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1000 / SERVER_TICK_HZ));
        loop {
            interval.tick().await;
            let now = Instant::now();

            // Limpiar jugadores desconectados (> 5 segundos sin enviar latido)
            world_tick.players.retain(|_, p| now.duration_since(p.last_seen).as_secs() < 5);

            // Retransmitir estado de cada rival a los demás jugadores conectados
            for sender in world_tick.players.iter() {
                let wire_bytes = sender.last_packet.to_wire_bytes();
                for receiver in world_tick.players.iter() {
                    if sender.key() != receiver.key() {
                        if let Some(target_addr) = receiver.udp_addr {
                            let _ = udp_send.send_to(wire_bytes, target_addr).await;
                        }
                    }
                }
            }
        }
    });

    // ========================================================================
    // 2. CANAL WEB: SERVIDOR WEBTRANSPORT QUIC (Para Navegadores en Puerto 4433)
    // ========================================================================
    let identity = Identity::self_signed(["localhost", "127.0.0.1"])?;
    let config = ServerConfig::builder()
        .with_bind_default(4433)
        .with_identity(identity)
        .build();

    let web_endpoint = Endpoint::server(config)?;
    println!("🌐 Bus WebTransport (QUIC/HTTP3) escuchando en https://0.0.0.0:4433");

    loop {
        let incoming = web_endpoint.accept().await;
        let world_web = world.clone();
        tokio::spawn(async move {
            let _ = handle_webtransport_client(incoming, world_web).await;
        });
    }
}

async fn handle_webtransport_client(
    incoming: IncomingSession,
    world: Arc<WorldServer>,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_request = incoming.await?;
    let connection = session_request.accept().await?;
    let conn_recv = Arc::new(connection);
    let conn_send = conn_recv.clone();
    let world_recv = world.clone();

    // Hilo de lectura de Datagramas QUIC desde el navegador (.wasm)
    tokio::spawn(async move {
        while let Ok(datagram) = conn_recv.receive_datagram().await {
            if let Some(pkt) = NxNetStatePacket::from_wire_bytes(&datagram) {
                world_recv.process_incoming_packet(pkt, None);

                // Envía de vuelta el estado de los rivales de Android/iOS/Web al navegador
                for rival in world_recv.players.iter() {
                    if *rival.key() != pkt.player_id {
                        let _ = conn_send.send_datagram(rival.last_packet.to_wire_bytes());
                    }
                }
            }
        }
    });

    Ok(())
}
