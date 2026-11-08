use core::marker::PhantomData;

pub struct FrameArena {
    storage: Vec<u8>,
    cursor: usize,
    peak_usage: usize,
}

impl FrameArena {
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self { storage: vec![0u8; capacity_bytes], cursor: 0, peak_usage: 0 }
    }

    #[inline(always)]
    pub fn alloc<T: Copy>(&mut self, val: T) -> &mut T {
        let align = core::mem::align_of::<T>().max(8);
        let size = core::mem::size_of::<T>();
        let aligned = (self.cursor + align - 1) & !(align - 1);
        let next = aligned + size;
        assert!(next <= self.storage.len(), "FrameArena agotada");
        self.cursor = next;
        if next > self.peak_usage { self.peak_usage = next; }
        unsafe {
            let ptr = self.storage.as_mut_ptr().add(aligned) as *mut T;
            core::ptr::write(ptr, val);
            &mut *ptr
        }
    }

    #[inline(always)]
    pub fn alloc_slice_fill<T: Copy>(&mut self, len: usize, default_val: T) -> &mut [T] {
        let align = core::mem::align_of::<T>().max(16);
        let byte_size = core::mem::size_of::<T>() * len;
        let aligned = (self.cursor + align - 1) & !(align - 1);
        let next = aligned + byte_size;
        assert!(next <= self.storage.len(), "FrameArena agotada en slice");
        self.cursor = next;
        if next > self.peak_usage { self.peak_usage = next; }
        unsafe {
            let ptr = self.storage.as_mut_ptr().add(aligned) as *mut T;
            for i in 0..len {
                core::ptr::write(ptr.add(i), default_val);
            }
            core::slice::from_raw_parts_mut(ptr, len)
        }
    }

    #[inline(always)] pub fn reset(&mut self) { self.cursor = 0; }
    #[inline(always)] pub fn used_bytes(&self) -> usize { self.cursor }
    #[inline(always)] pub fn peak_bytes(&self) -> usize { self.peak_usage }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub index: u32,
    pub generation: u32,
    _marker: PhantomData<T>,
}
impl<T> Clone for Handle<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy for Handle<T> {}

struct PoolSlot<T> { generation: u32, value: Option<T> }

pub struct GenerationalPool<T> {
    slots: Vec<PoolSlot<T>>,
    free_indices: Vec<u32>,
}

impl<T> GenerationalPool<T> {
    pub fn with_capacity(cap: usize) -> Self {
        Self { slots: Vec::with_capacity(cap), free_indices: Vec::new() }
    }

    pub fn insert(&mut self, item: T) -> Handle<T> {
        if let Some(idx) = self.free_indices.pop() {
            let slot = &mut self.slots[idx as usize];
            slot.value = Some(item);
            Handle { index: idx, generation: slot.generation, _marker: PhantomData }
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(PoolSlot { generation: 1, value: Some(item) });
            Handle { index: idx, generation: 1, _marker: PhantomData }
        }
    }

    pub fn get(&self, h: Handle<T>) -> Option<&T> {
        let slot = self.slots.get(h.index as usize)?;
        if slot.generation == h.generation { slot.value.as_ref() } else { None }
    }

    pub fn remove(&mut self, h: Handle<T>) -> Option<T> {
        let slot = self.slots.get_mut(h.index as usize)?;
        if slot.generation == h.generation && slot.value.is_some() {
            slot.generation = slot.generation.wrapping_add(1);
            self.free_indices.push(h.index);
            slot.value.take()
        } else {
            None
        }
    }
}

// ============================================================================
// LOCK-FREE SPSC RING BUFFER (Comunicación entre Hilos sin Mutex)
// ============================================================================
pub struct SpscRingBuffer<T: Copy, const N: usize> {
    buffer: core::cell::UnsafeCell<[Option<T>; N]>,
    head: core::sync::atomic::AtomicUsize,
    tail: core::sync::atomic::AtomicUsize,
}

unsafe impl<T: Copy + Send, const N: usize> Sync for SpscRingBuffer<T, N> {}
unsafe impl<T: Copy + Send, const N: usize> Send for SpscRingBuffer<T, N> {}

impl<T: Copy, const N: usize> SpscRingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: core::cell::UnsafeCell::new([const { None }; N]),
            head: core::sync::atomic::AtomicUsize::new(0),
            tail: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    pub fn push(&self, item: T) -> bool {
        let head = self.head.load(core::sync::atomic::Ordering::Relaxed);
        let next_head = (head + 1) % N;
        if next_head == self.tail.load(core::sync::atomic::Ordering::Acquire) {
            return false;
        }
        unsafe {
            (*self.buffer.get())[head] = Some(item);
        }
        self.head.store(next_head, core::sync::atomic::Ordering::Release);
        true
    }

    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(core::sync::atomic::Ordering::Relaxed);
        if tail == self.head.load(core::sync::atomic::Ordering::Acquire) {
            return None;
        }
        let item = unsafe { (*self.buffer.get())[tail].take() };
        self.tail.store((tail + 1) % N, core::sync::atomic::Ordering::Release);
        item
    }
}
