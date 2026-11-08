use crate::quat::Quat;
use crate::vec3::Vec3;
use crate::vec4::Vec4;
use bytemuck::{Pod, Zeroable};
use core::ops::Mul;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Mat4 {
    pub cols: [Vec4; 4],
}

impl Default for Mat4 {
    fn default() -> Self { Self::IDENTITY }
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        cols: [
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        ],
    };

    #[inline(always)]
    pub const fn from_cols(c0: Vec4, c1: Vec4, c2: Vec4, c3: Vec4) -> Self {
        Self { cols: [c0, c1, c2, c3] }
    }

    pub fn from_quat(q: Quat) -> Self {
        let (x2, y2, z2) = (q.x + q.x, q.y + q.y, q.z + q.z);
        let (xx, xy, xz) = (q.x * x2, q.x * y2, q.x * z2);
        let (yy, yz, zz) = (q.y * y2, q.y * z2, q.z * z2);
        let (wx, wy, wz) = (q.w * x2, q.w * y2, q.w * z2);

        Self::from_cols(
            Vec4::new(1.0 - (yy + zz), xy + wz, xz - wy, 0.0),
            Vec4::new(xy - wz, 1.0 - (xx + zz), yz + wx, 0.0),
            Vec4::new(xz + wy, yz - wx, 1.0 - (xx + yy), 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    pub fn from_scale_rotation_translation(scale: Vec3, rot: Quat, trans: Vec3) -> Self {
        let mut m = Self::from_quat(rot);
        m.cols[0] *= scale.x;
        m.cols[1] *= scale.y;
        m.cols[2] *= scale.z;
        m.cols[3] = Vec4::new(trans.x, trans.y, trans.z, 1.0);
        m
    }

    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize_or_zero();
        let s = f.cross(up).normalize_or_zero();
        let u = s.cross(f);
        Self::from_cols(
            Vec4::new(s.x, u.x, -f.x, 0.0),
            Vec4::new(s.y, u.y, -f.y, 0.0),
            Vec4::new(s.z, u.z, -f.z, 0.0),
            Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    pub fn perspective_rh_zo(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        let r_inv = 1.0 / (z_near - z_far);
        Self::from_cols(
            Vec4::new(f / aspect_ratio, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, z_far * r_inv, -1.0),
            Vec4::new(0.0, 0.0, z_near * z_far * r_inv, 0.0),
        )
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    #[inline(always)]
    fn mul(self, r: Vec4) -> Vec4 {
        self.cols[0] * r.x + self.cols[1] * r.y + self.cols[2] * r.z + self.cols[3] * r.w
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;
    #[inline(always)]
    fn mul(self, r: Mat4) -> Mat4 {
        Mat4::from_cols(self * r.cols[0], self * r.cols[1], self * r.cols[2], self * r.cols[3])
    }
}
