// Archivo: crates/nx_hal_gpu/src/skinning_compute.rs
use bytemuck::{Pod, Zeroable};
use nx_core::{Mat4, Quat, Vec3, Vec4};

pub const MAX_JOINTS_PER_SKELETON: usize = 64;

/// Vértice 3D con pesos de huesos para Animación Esquelética (Skeletal Skinning)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct NxSkinnedVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joint_indices: [u16; 4], // Índices de hasta 4 huesos que influyen en este vértice
    pub joint_weights: [f32; 4], // Pesos normalizados (suman 1.0)
}

/// Articulación individual dentro de un esqueleto 3D
#[derive(Clone, Copy, Debug)]
pub struct SkeletonJoint {
    pub parent_index: i32, // -1 si es el hueso raíz (Pelvis / Root)
    pub inverse_bind_matrix: Mat4,
    pub local_position: Vec3,
    pub local_rotation: Quat,
    pub local_scale: Vec3,
}

/// Pose completa de 64 matrices 4x4 (4 KB) lista para subirse al Uniform/Storage Buffer de la GPU
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuSkeletonPalette {
    pub joint_matrices: [Mat4; MAX_JOINTS_PER_SKELETON],
}

impl Default for GpuSkeletonPalette {
    fn default() -> Self {
        Self {
            joint_matrices: [Mat4::IDENTITY; MAX_JOINTS_PER_SKELETON],
        }
    }
}

impl GpuSkeletonPalette {
    /// Calcula la jerarquía de huesos y multiplica por la `inverse_bind_matrix` para el Vertex Shader
    pub fn compute_palette(&mut self, joints: &[SkeletonJoint]) {
        let count = joints.len().min(MAX_JOINTS_PER_SKELETON);
        let mut global_poses = [Mat4::IDENTITY; MAX_JOINTS_PER_SKELETON];

        for i in 0..count {
            let j = &joints[i];
            let local_mat = Mat4::from_scale_rotation_translation(
                j.local_scale,
                j.local_rotation,
                j.local_position,
            );

            if j.parent_index >= 0 && (j.parent_index as usize) < i {
                global_poses[i] = global_poses[j.parent_index as usize] * local_mat;
            } else {
                global_poses[i] = local_mat;
            }

            // Matriz final que deforma el vértice desde la pose T original a la pose animada actual
            self.joint_matrices[i] = global_poses[i] * j.inverse_bind_matrix;
        }
    }
}

/// Partícula individual procesada al 100% dentro de la tarjeta gráfica (`@gpu_compute`)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuComputeParticle {
    pub position: Vec3,
    pub lifetime: f32,
    pub velocity: Vec3,
    pub scale: f32,
    pub color: Vec4,
}

/// Shader WGSL de Compute + Linear Blend Skinning (Vulkan / Metal / WebGPU)
pub const SKINNING_AND_COMPUTE_WGSL: &str = r#"
struct GpuComputeParticle {
    position: vec3<f32>,
    lifetime: f32,
    velocity: vec3<f32>,
    scale: f32,
    color: vec4<f32>,
};

struct SimParams {
    dt: f32,
    gravity_y: f32,
    turbulence: f32,
    total_particles: u32,
};

@group(0) @binding(0) var<uniform> u_sim: SimParams;
@group(0) @binding(1) var<storage, read_write> particles: array<GpuComputeParticle>;

// Compute Shader: actualiza 64 partículas por hilo de GPU en paralelo
@compute @workgroup_size(64)
fn cs_update_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= u_sim.total_particles) {
        return;
    }

    var p = particles[idx];
    if (p.lifetime > 0.0) {
        p.velocity.y += u_sim.gravity_y * u_sim.dt;
        p.position += p.velocity * u_sim.dt;
        p.lifetime -= u_sim.dt;
        p.color.a = clamp(p.lifetime, 0.0, 1.0);
        particles[idx] = p;
    }
}
"#;
