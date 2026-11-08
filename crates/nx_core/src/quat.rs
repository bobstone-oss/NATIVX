use crate::vec3::Vec3;
use bytemuck::{Pod, Zeroable};
use core::ops::Mul;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self { Self::IDENTITY }
}

impl Quat {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    #[inline(always)]
    pub const fn from_xyzw(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }

    #[inline(always)]
    pub fn from_axis_angle(axis: Vec3, angle_rad: f32) -> Self {
        let (s, c) = (angle_rad * 0.5).sin_cos();
        let n = axis.normalize_or_zero();
        Self { x: n.x * s, y: n.y * s, z: n.z * s, w: c }
    }

    #[inline(always)]
    pub fn normalize(self) -> Self {
        let lsq = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        if lsq > 1e-12 {
            let inv = 1.0 / lsq.sqrt();
            Self { x: self.x * inv, y: self.y * inv, z: self.z * inv, w: self.w * inv }
        } else {
            Self::IDENTITY
        }
    }

    #[inline(always)]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v) * 2.0;
        v + (t * self.w) + qv.cross(t)
    }
}

impl Mul<Quat> for Quat {
    type Output = Self;
    #[inline(always)]
    fn mul(self, r: Self) -> Self {
        Self {
            x: self.w * r.x + self.x * r.w + self.y * r.z - self.z * r.y,
            y: self.w * r.y - self.x * r.z + self.y * r.w + self.z * r.x,
            z: self.w * r.z + self.x * r.y - self.y * r.x + self.z * r.w,
            w: self.w * r.w - self.x * r.x - self.y * r.y - self.z * r.z,
        }
    }
}
