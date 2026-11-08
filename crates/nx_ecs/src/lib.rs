use bytemuck::{Pod, Zeroable};
use nx_core::{FrameArena, Mat4, Quat, Transform3D, Vec3, Vec4};
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

impl EntityId {
    pub const NULL: Self = Self { index: u32::MAX, generation: 0 };
    pub const fn is_null(self) -> bool { self.index == u32::MAX }
}

const EMPTY_SLOT: u32 = u32::MAX;

pub trait ErasedStorage: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove_entity(&mut self, entity: EntityId) -> bool;
}

pub struct SparseSet<T: 'static + Send + Sync> {
    sparse: Vec<u32>,
    dense: Vec<T>,
    entities: Vec<EntityId>,
}

impl<T: 'static + Send + Sync> SparseSet<T> {
    pub fn with_capacity(cap: usize) -> Self {
        Self { sparse: Vec::with_capacity(cap), dense: Vec::with_capacity(cap), entities: Vec::with_capacity(cap) }
    }

    pub fn insert(&mut self, entity: EntityId, component: T) {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() { self.sparse.resize(idx + 1, EMPTY_SLOT); }
        let d = self.sparse[idx];
        if d != EMPTY_SLOT {
            self.dense[d as usize] = component;
            self.entities[d as usize] = entity;
            return;
        }
        self.sparse[idx] = self.dense.len() as u32;
        self.dense.push(component);
        self.entities.push(entity);
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.sparse.get(entity.index as usize).map(|&d| d != EMPTY_SLOT && self.entities[d as usize] == entity).unwrap_or(false)
    }

    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let d = *self.sparse.get(entity.index as usize)?;
        if d != EMPTY_SLOT && self.entities[d as usize] == entity { Some(&self.dense[d as usize]) } else { None }
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let d = *self.sparse.get(entity.index as usize)?;
        if d != EMPTY_SLOT && self.entities[d as usize] == entity { Some(&mut self.dense[d as usize]) } else { None }
    }

    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let idx = entity.index as usize;
        let d = *self.sparse.get(idx)?;
        if d == EMPTY_SLOT || self.entities[d as usize] != entity { return None; }
        self.sparse[idx] = EMPTY_SLOT;
        let last = self.dense.len() - 1;
        let rem = self.dense.swap_remove(d as usize);
        self.entities.swap_remove(d as usize);
        if (d as usize) != last {
            let moved = self.entities[d as usize];
            self.sparse[moved.index as usize] = d;
        }
        Some(rem)
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.entities.iter().copied().zip(self.dense.iter_mut())
    }
}

impl<T: 'static + Send + Sync> ErasedStorage for SparseSet<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
    fn remove_entity(&mut self, entity: EntityId) -> bool { self.remove(entity).is_some() }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct LocalTransform(pub Transform3D);

impl LocalTransform {
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Self { Self(Transform3D::from_xyz(x, y, z)) }
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self(Transform3D { position, _pad0: 0.0, rotation, scale: Vec3::ONE, _pad1: 0.0 })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GlobalTransform(pub Mat4);

impl GlobalTransform {
    pub fn world_position(&self) -> Vec3 { self.0.cols[3].xyz() }
}

#[derive(Clone, Copy, Debug)]
pub struct Parent(pub EntityId);

#[derive(Clone, Copy, Debug, Default)]
pub struct Children {
    pub ids: [EntityId; 8],
    pub count: usize,
}

impl Children {
    pub fn add_child(&mut self, c: EntityId) {
        if self.count < 8 { self.ids[self.count] = c; self.count += 1; }
    }
    pub fn as_slice(&self) -> &[EntityId] { &self.ids[..self.count] }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct RigidBody3D {
    pub velocity: Vec3,
    pub inverse_mass: f32,
    pub angular_velocity: Vec3,
    pub linear_damping: f32,
    pub force_accumulator: Vec3,
    pub restitution: f32,
}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            inverse_mass: 1.0,
            angular_velocity: Vec3::ZERO,
            linear_damping: 0.99,
            force_accumulator: Vec3::ZERO,
            restitution: 0.8,
        }
    }
}

impl RigidBody3D {
    pub fn dynamic_with_mass(mass: f32) -> Self {
        Self { inverse_mass: if mass > 1e-6 { 1.0 / mass } else { 0.0 }, ..Default::default() }
    }
    pub fn apply_force(&mut self, f: Vec3) { self.force_accumulator += f; }
    pub fn apply_impulse(&mut self, j: Vec3) { self.velocity += j * self.inverse_mass; }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ColliderAabb {
    pub half_extents: Vec3,
    pub is_trigger: u32,
}

impl ColliderAabb {
    pub const fn new(hx: f32, hy: f32, hz: f32) -> Self {
        Self { half_extents: Vec3::new(hx, hy, hz), is_trigger: 0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct MeshRender {
    pub mesh_asset_id: u32,
    pub material_id: u32,
    pub base_color: Vec4,
    pub visible: u32,
}

impl Default for MeshRender {
    fn default() -> Self {
        Self { mesh_asset_id: 0, material_id: 0, base_color: Vec4::ONE, visible: 1 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuRenderInstance {
    pub world_matrix: Mat4,
    pub color: Vec4,
}

pub struct CommandBuffer {
    actions: Vec<Box<dyn FnOnce(&mut World) + Send + Sync>>,
}

impl Default for CommandBuffer {
    fn default() -> Self { Self::new() }
}

impl CommandBuffer {
    pub fn new() -> Self { Self { actions: Vec::new() } }
    pub fn despawn(&mut self, entity: EntityId) {
        self.actions.push(Box::new(move |w: &mut World| { w.despawn(entity); }));
    }
    pub fn apply(&mut self, world: &mut World) {
        for act in self.actions.drain(..) { act(world); }
    }
}

pub struct World {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free_list: Vec<u32>,
    alive_count: usize,
    storages: HashMap<TypeId, Box<dyn ErasedStorage>>,
}

impl Default for World {
    fn default() -> Self { Self::new() }
}

impl World {
    pub fn new() -> Self { Self::with_capacity(512) }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            generations: Vec::with_capacity(cap),
            alive: Vec::with_capacity(cap),
            free_list: Vec::new(),
            alive_count: 0,
            storages: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        self.alive_count += 1;
        if let Some(idx) = self.free_list.pop() {
            self.alive[idx as usize] = true;
            EntityId { index: idx, generation: self.generations[idx as usize] }
        } else {
            let idx = self.generations.len() as u32;
            self.generations.push(1);
            self.alive.push(true);
            EntityId { index: idx, generation: 1 }
        }
    }

    pub fn spawn_spatial(&mut self, local: LocalTransform) -> EntityId {
        let id = self.spawn();
        let mat = local.0.to_matrix();
        self.insert(id, local);
        self.insert(id, GlobalTransform(mat));
        id
    }

    pub fn is_alive(&self, e: EntityId) -> bool {
        self.alive.get(e.index as usize).copied().unwrap_or(false)
            && self.generations[e.index as usize] == e.generation
    }

    pub fn entity_count(&self) -> usize { self.alive_count }

    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: EntityId, comp: T) {
        if !self.is_alive(entity) { return; }
        let storage = self.storages.entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SparseSet::<T>::with_capacity(128)));
        storage.as_any_mut().downcast_mut::<SparseSet<T>>().unwrap().insert(entity, comp);
    }

    pub fn get<T: 'static + Send + Sync>(&self, e: EntityId) -> Option<&T> {
        self.storage::<T>()?.get(e)
    }

    pub fn get_mut<T: 'static + Send + Sync>(&mut self, e: EntityId) -> Option<&mut T> {
        self.storage_mut::<T>()?.get_mut(e)
    }

    pub fn storage<T: 'static + Send + Sync>(&self) -> Option<&SparseSet<T>> {
        self.storages.get(&TypeId::of::<T>())?.as_any().downcast_ref::<SparseSet<T>>()
    }

    pub fn storage_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut SparseSet<T>> {
        self.storages.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut::<SparseSet<T>>()
    }

    pub fn set_parent(&mut self, child: EntityId, parent: EntityId) {
        self.insert(child, Parent(parent));
        if self.get::<Children>(parent).is_none() {
            self.insert(parent, Children::default());
        }
        if let Some(c) = self.get_mut::<Children>(parent) { c.add_child(child); }
    }

    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.is_alive(entity) { return false; }
        if let Some(children) = self.get::<Children>(entity).copied() {
            for &c in children.as_slice() { self.despawn(c); }
        }
        for s in self.storages.values_mut() { s.remove_entity(entity); }
        let idx = entity.index as usize;
        self.alive[idx] = false;
        self.generations[idx] = self.generations[idx].wrapping_add(1);
        self.free_list.push(entity.index);
        self.alive_count -= 1;
        true
    }

    pub fn query_mut_ref<A: 'static + Send + Sync, B: 'static + Send + Sync, F: FnMut(EntityId, &mut A, &B)>(&mut self, mut f: F) {
        let id_a = TypeId::of::<A>();
        let id_b = TypeId::of::<B>();
        assert_ne!(id_a, id_b);
        let ( Some(ptr_a), Some(ptr_b) ) = (
            self.storages.get_mut(&id_a).map(|b| b as *mut Box<dyn ErasedStorage>),
            self.storages.get(&id_b).map(|b| b as *const Box<dyn ErasedStorage>),
        ) else { return; };
        unsafe {
            let sa = (*ptr_a).as_any_mut().downcast_mut::<SparseSet<A>>().unwrap();
            let sb = (*ptr_b).as_any().downcast_ref::<SparseSet<B>>().unwrap();
            for (e, ca) in sa.iter_mut() {
                if let Some(cb) = sb.get(e) { f(e, ca, cb); }
            }
        }
    }

    pub fn query_pair_mut<A: 'static + Send + Sync, B: 'static + Send + Sync, F: FnMut(EntityId, &mut A, &mut B)>(&mut self, mut f: F) {
        let id_a = TypeId::of::<A>();
        let id_b = TypeId::of::<B>();
        assert_ne!(id_a, id_b);
        let ( Some(ptr_a), Some(ptr_b) ) = (
            self.storages.get_mut(&id_a).map(|b| b as *mut Box<dyn ErasedStorage>),
            self.storages.get_mut(&id_b).map(|b| b as *mut Box<dyn ErasedStorage>),
        ) else { return; };
        unsafe {
            let sa = (*ptr_a).as_any_mut().downcast_mut::<SparseSet<A>>().unwrap();
            let sb = (*ptr_b).as_any_mut().downcast_mut::<SparseSet<B>>().unwrap();
            for (e, ca) in sa.iter_mut() {
                if let Some(cb) = sb.get_mut(e) { f(e, ca, cb); }
            }
        }
    }
}

pub fn system_integrate_physics(world: &mut World, gravity: Vec3, dt: f32) {
    world.query_pair_mut::<LocalTransform, RigidBody3D, _>(|_, tr, body| {
        if body.inverse_mass <= 0.0 { return; }
        let acc = gravity + (body.force_accumulator * body.inverse_mass);
        body.velocity += acc * dt;
        body.velocity *= body.linear_damping;
        tr.0.position += body.velocity * dt;

        let ang_sq = body.angular_velocity.length_squared();
        if ang_sq > 1e-8 {
            let speed = ang_sq.sqrt();
            let dq = Quat::from_axis_angle(body.angular_velocity / speed, speed * dt);
            tr.0.rotation = (dq * tr.0.rotation).normalize();
        }
        body.force_accumulator = Vec3::ZERO;
    });
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionManifold {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    pub normal: Vec3,
    pub penetration: f32,
}

pub fn system_resolve_aabb_collisions<'a>(world: &mut World, arena: &'a mut FrameArena) -> &'a [CollisionManifold] {
    let mut proxies = Vec::new();
    if let (Some(trs), Some(cols)) = (world.storage::<LocalTransform>(), world.storage::<ColliderAabb>()) {
        for (e, c) in cols.iter() {
            if let Some(t) = trs.get(e) { proxies.push((e, t.0.position, c.half_extents)); }
        }
    }
    let mut hits = Vec::new();
    for i in 0..proxies.len() {
        for j in (i + 1)..proxies.len() {
            let (ea, pa, ha) = proxies[i];
            let (eb, pb, hb) = proxies[j];
            let d = pb - pa;
            let ox = (ha.x + hb.x) - d.x.abs();
            let oy = (ha.y + hb.y) - d.y.abs();
            let oz = (ha.z + hb.z) - d.z.abs();
            if ox > 0.0 && oy > 0.0 && oz > 0.0 {
                let (normal, pen) = if ox < oy && ox < oz {
                    (Vec3::new(d.x.signum(), 0.0, 0.0), ox)
                } else if oy < oz {
                    (Vec3::new(0.0, d.y.signum(), 0.0), oy)
                } else {
                    (Vec3::new(0.0, 0.0, d.z.signum()), oz)
                };
                hits.push(CollisionManifold { entity_a: ea, entity_b: eb, normal, penetration: pen });
            }
        }
    }
    for m in &hits {
        let corr = m.normal * (m.penetration * 0.5);
        if let Some(ta) = world.get_mut::<LocalTransform>(m.entity_a) { ta.0.position -= corr; }
        if let Some(tb) = world.get_mut::<LocalTransform>(m.entity_b) { tb.0.position += corr; }
        if let Some(ba) = world.get_mut::<RigidBody3D>(m.entity_a) { ba.velocity = -ba.velocity * 0.85; }
        if let Some(bb) = world.get_mut::<RigidBody3D>(m.entity_b) { bb.velocity = -bb.velocity * 0.85; }
    }
    let slice = arena.alloc_slice_fill(hits.len(), CollisionManifold {
        entity_a: EntityId::NULL, entity_b: EntityId::NULL, normal: Vec3::ZERO, penetration: 0.0
    });
    slice.copy_from_slice(&hits);
    slice
}

pub fn system_propagate_transforms(world: &mut World) {
    let mut roots = Vec::new();
    if let Some(locals) = world.storage::<LocalTransform>() {
        let parents = world.storage::<Parent>();
        for (e, l) in locals.iter() {
            if !parents.map(|p| p.contains(e)).unwrap_or(false) {
                roots.push((e, l.0.to_matrix()));
            }
        }
    }
    for (re, rm) in roots {
        world.insert(re, GlobalTransform(rm));
        propagate_children(world, re, rm);
    }
}

fn propagate_children(world: &mut World, parent: EntityId, parent_mat: Mat4) {
    let Some(children) = world.get::<Children>(parent).copied() else { return; };
    for &cid in children.as_slice() {
        if let Some(cl) = world.get::<LocalTransform>(cid).copied() {
            let cm = parent_mat * cl.0.to_matrix();
            world.insert(cid, GlobalTransform(cm));
            propagate_children(world, cid, cm);
        }
    }
}

pub fn extract_render_batch<'a>(world: &World, arena: &'a mut FrameArena) -> &'a [GpuRenderInstance] {
    let (Some(meshes), Some(globals)) = (world.storage::<MeshRender>(), world.storage::<GlobalTransform>()) else {
        return &[];
    };
    let count = meshes.iter().filter(|(e, m)| m.visible != 0 && globals.contains(*e)).count();
    let batch = arena.alloc_slice_fill(count, GpuRenderInstance::default());
    let mut i = 0;
    for (e, m) in meshes.iter() {
        if m.visible != 0 {
            if let Some(g) = globals.get(e) {
                batch[i] = GpuRenderInstance { world_matrix: g.0, color: m.base_color };
                i += 1;
            }
        }
    }
    batch
}

pub mod debug_overlay;
