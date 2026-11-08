pub mod mat4;
pub mod memory;
pub mod quat;
pub mod transform;
pub mod vec2;
pub mod vec3;
pub mod vec4;

pub use mat4::Mat4;
pub use memory::{FrameArena, GenerationalPool, Handle, SpscRingBuffer};
pub use quat::Quat;
pub use transform::{Camera3D, Transform3D};
pub use vec2::Vec2;
pub use vec3::Vec3;
pub use vec4::Vec4;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_core_math_and_arena() {
        let mut arena = FrameArena::with_capacity(1024);
        let v = arena.alloc(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v.cross(Vec3::Y), Vec3::new(-3.0, 0.0, 1.0));
        arena.reset();
        assert_eq!(arena.used_bytes(), 0);
    }
}

pub mod stdlib;
