use crate::vec3::Vec3;
use bytemuck::{Pod, Zeroable};
use core::ops::{Add, Mul, MulAssign, Sub};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0, w: 1.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }

    #[inline(always)]
    pub const fn from_vec3(v: Vec3, w: f32) -> Self { Self::new(v.x, v.y, v.z, w) }

    #[inline(always)]
    pub const fn xyz(self) -> Vec3 { Vec3::new(self.x, self.y, self.z) }
}

impl Add for Vec4 {
    type Output = Self;
    #[inline(always)] fn add(self, r: Self) -> Self { Self::new(self.x + r.x, self.y + r.y, self.z + r.z, self.w + r.w) }
}
impl Sub for Vec4 {
    type Output = Self;
    #[inline(always)] fn sub(self, r: Self) -> Self { Self::new(self.x - r.x, self.y - r.y, self.z - r.z, self.w - r.w) }
}
impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline(always)] fn mul(self, r: f32) -> Self { Self::new(self.x * r, self.y * r, self.z * r, self.w * r) }
}
impl MulAssign<f32> for Vec4 {
    #[inline(always)] fn mul_assign(&mut self, r: f32) { self.x *= r; self.y *= r; self.z *= r; self.w *= r; }
}
