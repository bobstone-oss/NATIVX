use crate::{Mat4, Quat, Vec3};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Transform3D {
    pub position: Vec3,
    pub _pad0: f32,
    pub rotation: Quat,
    pub scale: Vec3,
    pub _pad1: f32,
}

impl Default for Transform3D {
    fn default() -> Self { Self::IDENTITY }
}

impl Transform3D {
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        _pad0: 0.0,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        _pad1: 0.0,
    };

    #[inline(always)]
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self { position: Vec3::new(x, y, z), ..Self::IDENTITY }
    }

    #[inline(always)]
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera3D {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 2.0, 12.0),
            target: Vec3::ZERO,
            up: Vec3::UP,
            fov_y_radians: 60.0_f32.to_radians(),
            aspect_ratio: 16.0 / 9.0,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }
}

impl Camera3D {
    #[inline(always)]
    pub fn view_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh_zo(self.fov_y_radians, self.aspect_ratio, self.z_near, self.z_far)
            * Mat4::look_at_rh(self.eye, self.target, self.up)
    }
}
