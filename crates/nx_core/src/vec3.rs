use crate::vec2::Vec2;
use bytemuck::{Pod, Zeroable};
use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };
    pub const RIGHT: Self = Self::X;
    pub const UP: Self = Self::Y;
    pub const FORWARD: Self = Self { x: 0.0, y: 0.0, z: -1.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }

    #[inline(always)]
    pub const fn from_array(a: [f32; 3]) -> Self { Self { x: a[0], y: a[1], z: a[2] } }

    #[inline(always)]
    pub const fn to_array(self) -> [f32; 3] { [self.x, self.y, self.z] }

    #[inline(always)]
    pub const fn xy(self) -> Vec2 { Vec2::new(self.x, self.y) }

    #[inline(always)]
    pub fn dot(self, r: Self) -> f32 { self.x * r.x + self.y * r.y + self.z * r.z }

    #[inline(always)]
    pub fn cross(self, r: Self) -> Self {
        Self::new(
            self.y * r.z - self.z * r.y,
            self.z * r.x - self.x * r.z,
            self.x * r.y - self.y * r.x,
        )
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 { self.dot(self) }

    #[inline(always)]
    pub fn length(self) -> f32 { self.length_squared().sqrt() }

    #[inline(always)]
    pub fn normalize_or_zero(self) -> Self {
        let lsq = self.length_squared();
        if lsq > 1e-12 { self * (1.0 / lsq.sqrt()) } else { Self::ZERO }
    }

    #[inline(always)]
    pub fn abs(self) -> Self { Self::new(self.x.abs(), self.y.abs(), self.z.abs()) }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline(always)] fn add(self, r: Self) -> Self { Self::new(self.x + r.x, self.y + r.y, self.z + r.z) }
}
impl AddAssign for Vec3 {
    #[inline(always)] fn add_assign(&mut self, r: Self) { self.x += r.x; self.y += r.y; self.z += r.z; }
}
impl Sub for Vec3 {
    type Output = Self;
    #[inline(always)] fn sub(self, r: Self) -> Self { Self::new(self.x - r.x, self.y - r.y, self.z - r.z) }
}
impl SubAssign for Vec3 {
    #[inline(always)] fn sub_assign(&mut self, r: Self) { self.x -= r.x; self.y -= r.y; self.z -= r.z; }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline(always)] fn mul(self, r: f32) -> Self { Self::new(self.x * r, self.y * r, self.z * r) }
}
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline(always)] fn mul(self, r: Vec3) -> Vec3 { r * self }
}
impl Mul<Vec3> for Vec3 {
    type Output = Self;
    #[inline(always)] fn mul(self, r: Self) -> Self { Self::new(self.x * r.x, self.y * r.y, self.z * r.z) }
}
impl MulAssign<f32> for Vec3 {
    #[inline(always)] fn mul_assign(&mut self, r: f32) { self.x *= r; self.y *= r; self.z *= r; }
}
impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline(always)] fn div(self, r: f32) -> Self { self * (1.0 / r) }
}
impl Neg for Vec3 {
    type Output = Self;
    #[inline(always)] fn neg(self) -> Self { Self::new(-self.x, -self.y, -self.z) }
}
