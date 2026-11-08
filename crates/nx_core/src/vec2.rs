use bytemuck::{Pod, Zeroable};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32) -> Self { Self { x, y } }

    #[inline(always)]
    pub fn dot(self, rhs: Self) -> f32 { self.x * rhs.x + self.y * rhs.y }

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
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        Self::new(self.x + (rhs.x - self.x) * t, self.y + (rhs.y - self.y) * t)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline(always)] fn add(self, r: Self) -> Self { Self::new(self.x + r.x, self.y + r.y) }
}
impl AddAssign for Vec2 {
    #[inline(always)] fn add_assign(&mut self, r: Self) { self.x += r.x; self.y += r.y; }
}
impl Sub for Vec2 {
    type Output = Self;
    #[inline(always)] fn sub(self, r: Self) -> Self { Self::new(self.x - r.x, self.y - r.y) }
}
impl SubAssign for Vec2 {
    #[inline(always)] fn sub_assign(&mut self, r: Self) { self.x -= r.x; self.y -= r.y; }
}
impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline(always)] fn mul(self, r: f32) -> Self { Self::new(self.x * r, self.y * r) }
}
impl MulAssign<f32> for Vec2 {
    #[inline(always)] fn mul_assign(&mut self, r: f32) { self.x *= r; self.y *= r; }
}
impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline(always)] fn div(self, r: f32) -> Self { let inv = 1.0 / r; self * inv }
}
impl DivAssign<f32> for Vec2 {
    #[inline(always)] fn div_assign(&mut self, r: f32) { let inv = 1.0 / r; *self *= inv; }
}
impl Neg for Vec2 {
    type Output = Self;
    #[inline(always)] fn neg(self) -> Self { Self::new(-self.x, -self.y) }
}
