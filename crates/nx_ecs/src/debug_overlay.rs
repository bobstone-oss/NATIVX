use crate::{ColliderAabb, CollisionManifold, GlobalTransform, LocalTransform, World};
use bytemuck::{Pod, Zeroable};
use nx_core::{FrameArena, Vec3};

pub const FRAME_HISTORY_LEN: usize = 120;

/// Vértice ligero de 32 bytes para dibujar líneas 3D de depuración (Wireframe AABB, Rayos y Ejes)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct DebugLineVertex {
    pub position: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 4],
}

/// Primitiva 2D para el HUD de telemetría en esquina superior izquierda (Compatible con `nx_hal_ui`)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct DebugHudQuad {
    pub rect: [f32; 4],   // [x, y, w, h] en coordenadas normalizadas (0.0..1.0)
    pub color: [f32; 4],  // [R, G, B, A]
}

/// Estado global del inspector de rendimiento en tiempo real
pub struct DebugInspectorState {
    pub enabled: bool,
    pub show_colliders: bool,
    pub show_contact_normals: bool,
    pub show_memory_hud: bool,
    pub frame_times_ms: [f32; FRAME_HISTORY_LEN],
    pub frame_cursor: usize,
    pub peak_frame_time_ms: f32,
}

impl Default for DebugInspectorState {
    fn default() -> Self {
        Self {
            enabled: true,
            show_colliders: true,
            show_contact_normals: true,
            show_memory_hud: true,
            frame_times_ms: [16.66; FRAME_HISTORY_LEN],
            frame_cursor: 0,
            peak_frame_time_ms: 16.66,
        }
    }
}

impl DebugInspectorState {
    #[inline]
    pub fn record_frame(&mut self, dt_seconds: f32) {
        let ms = dt_seconds * 1000.0;
        self.frame_times_ms[self.frame_cursor] = ms;
        self.frame_cursor = (self.frame_cursor + 1) % FRAME_HISTORY_LEN;
        if ms > self.peak_frame_time_ms {
            self.peak_frame_time_ms = ms;
        }
    }

    #[inline]
    pub fn average_fps(&self) -> f32 {
        let sum: f32 = self.frame_times_ms.iter().sum();
        let avg_ms = sum / (FRAME_HISTORY_LEN as f32);
        if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 }
    }
}

/// Genera la geometría 3D de depuración usando el `FrameArena` (Zero allocations permanentes)
pub fn generate_debug_geometry<'a>(
    world: &World,
    state: &DebugInspectorState,
    arena: &'a mut FrameArena,
    collisions: &[CollisionManifold],
) -> &'a [DebugLineVertex] {
    if !state.enabled {
        return &[];
    }

    let mut lines = Vec::new();

    if state.show_colliders {
        if let Some(cols) = world.storage::<ColliderAabb>() {
            for (entity, col) in cols.iter() {
                let pos = if let Some(gt) = world.get::<GlobalTransform>(entity) {
                    Vec3::new(gt.0.cols[3].x, gt.0.cols[3].y, gt.0.cols[3].z)
                } else if let Some(lt) = world.get::<LocalTransform>(entity) {
                    lt.0.position
                } else {
                    Vec3::ZERO
                };
                let half = col.half_extents;
                push_aabb_wireframe(&mut lines, pos - half, pos + half, [0.0, 1.0, 0.4, 0.85]);
            }
        }
    }

    if state.show_contact_normals {
        for col in collisions {
            let start = Vec3::ZERO;
            let end = start + col.normal * 0.8;
            lines.push(DebugLineVertex { position: [start.x, start.y, start.z], _pad: 0.0, color: [1.0, 0.2, 0.2, 1.0] });
            lines.push(DebugLineVertex { position: [end.x, end.y, end.z], _pad: 0.0, color: [1.0, 0.8, 0.1, 1.0] });
        }
    }

    if lines.is_empty() {
        return &[];
    }

    let slice = arena.alloc_slice_fill(lines.len(), DebugLineVertex::default());
    slice.copy_from_slice(&lines);
    slice
}

#[inline]
fn push_aabb_wireframe(out: &mut Vec<DebugLineVertex>, min: Vec3, max: Vec3, color: [f32; 4]) {
    let p = [
        [min.x, min.y, min.z], [max.x, min.y, min.z], [max.x, max.y, min.z], [min.x, max.y, min.z],
        [min.x, min.y, max.z], [max.x, min.y, max.z], [max.x, max.y, max.z], [min.x, max.y, max.z],
    ];
    let edges = [
        (0,1), (1,2), (2,3), (3,0),
        (4,5), (5,6), (6,7), (7,4),
        (0,4), (1,5), (2,6), (3,7)
    ];
    for (i, j) in edges {
        out.push(DebugLineVertex { position: p[i], _pad: 0.0, color });
        out.push(DebugLineVertex { position: p[j], _pad: 0.0, color });
    }
}
