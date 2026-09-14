use ahash::AHashMap;
use std::alloc::Layout;
use std::any::{Any, TypeId};
use std::ptr::NonNull;

/// Accessor functions for converting a concrete type to a trait object.
///
/// This struct contains function pointers that handle upcasting from a concrete type `T`
/// to a trait object `Dyn`.
pub struct TraitAccessor<T, Dyn: ?Sized> {
    pub up_ref: fn(&T) -> &Dyn,
    pub up_mut: fn(&mut T) -> &mut Dyn,
    pub up_box: fn(T) -> Box<Dyn>,
}

/// Macro for implementing `TraitAccessible` for types.
///
/// This macro generates the necessary implementation to make types accessible
/// via a trait object in the map.
///
/// # Examples
///
/// ```rust
/// # use trait_type_map::impl_trait_accessible;
/// trait MyTrait {}
/// struct TypeA;
/// struct TypeB;
/// impl MyTrait for TypeA {}
/// impl MyTrait for TypeB {}
///
/// impl_trait_accessible!(dyn MyTrait; TypeA, TypeB);
/// ```
#[macro_export]
macro_rules! impl_trait_accessible {
    (dyn $dyn:path; $($ty:ty),+ $(,)?) => {$(
        impl $crate::TraitAccessible<dyn $dyn> for $ty {
            fn get_accessor() -> $crate::TraitAccessor<Self, dyn $dyn> {
                $crate::TraitAccessor { up_ref: |v| v, up_mut: |v| v, up_box: |v| Box::new(v) }
            }
        }
    )+};
}

// =============================================================================
// Vector Backend
// =============================================================================

/// Storage for multiple values of a single type in a vector.
///
/// Values can be accessed by index.
pub struct VecStorage<T, Dyn: ?Sized> {
    pub data: Vec<T>,
    trait_accessor: TraitAccessor<T, Dyn>,
}
impl<T, Dyn: ?Sized> VecStorage<T, Dyn> {
    /// Creates a new empty storage backed by the given upcast accessor.
    pub fn new(trait_accessor: TraitAccessor<T, Dyn>) -> Self {
        Self {
            data: Vec::new(),
            trait_accessor,
        }
    }

    /// Appends a value to the end of the storage and returns its index.
    pub fn push(&mut self, v: T) -> usize {
        let idx = self.data.len();
        self.data.push(v);
        idx
    }

    /// Returns an iterator over all stored values as concrete references.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    /// Returns a shared reference to the value at the given index. Panics if out of bounds.
    pub fn get(&self, i: usize) -> &T {
        self.data.get(i).unwrap()
    }

    /// Returns a mutable reference to the value at the given index. Panics if out of bounds.
    pub fn get_mut(&mut self, i: usize) -> &mut T {
        self.data.get_mut(i).unwrap()
    }

    //// # Safety
    //// `i` must be < `self.data.len()`. The query system guarantees this
    //// because `self.current_entity_idx < self.current_archetype_len`
    //// and archetype entity count == storage length.
    #[inline]
    pub unsafe fn get_unchecked(&self, i: usize) -> &T {
        unsafe { self.data.get_unchecked(i) }
    }

    //// # Safety
    //// `i` must be < `self.data.len()`. Same invariant as [`get_unchecked`].
    #[inline]
    pub unsafe fn get_mut_unchecked(&mut self, i: usize) -> &mut T {
        unsafe { &mut *self.data.as_mut_ptr().add(i) }
    }

    /// Returns a trait object reference to the value at index via the stored upcast accessor.
    pub fn get_dyn(&self, i: usize) -> &Dyn {
        (self.trait_accessor.up_ref)(self.get(i))
    }

    /// Returns a mutable trait object reference to the value at index.
    pub fn get_dyn_mut(&mut self, i: usize) -> &mut Dyn {
        let up_mut = self.trait_accessor.up_mut;
        up_mut(self.get_mut(i))
    }

    /// Removes and returns the value at index by swapping it with the last element (O(1)).
    pub fn swap_remove(&mut self, i: usize) -> T {
        self.data.swap_remove(i)
    }

    /// Removes the value at index via swap-remove and returns it boxed as a trait object.
    pub fn take_boxed(&mut self, i: usize) -> Box<Dyn> {
        (self.trait_accessor.up_box)(self.data.swap_remove(i))
    }
}

/// Trait object interface for vector storage.
///
/// This allows accessing stored values as trait objects without knowing the concrete type.
pub trait TraitVecStorage<Dyn: ?Sized>: Any {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, idx: usize) -> &Dyn;
    fn get_mut(&mut self, idx: usize) -> &mut Dyn;
    fn take_boxed(&mut self, idx: usize) -> Box<Dyn>;
    fn swap_remove(&mut self, idx: usize);
    /// Refresh the per-type function table (see [`ErasedVecStorage::refresh_ops`]).
    ///
    /// Backends that store per-type function pointers can re-point them at
    /// still-alive code; other backends ignore the call.
    fn refresh_ops(&mut self, _ops: ErasedVecStorageOps<Dyn>) {}
    fn as_storage_any(&self) -> &dyn Any;
    fn as_storage_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: 'static, Dyn: ?Sized + 'static> TraitVecStorage<Dyn> for VecStorage<T, Dyn> {
    /// Returns the number of stored values.
    fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns a trait object reference to the value at index.
    fn get(&self, idx: usize) -> &Dyn {
        VecStorage::<T, Dyn>::get_dyn(self, idx)
    }

    /// Returns a mutable trait object reference to the value at index.
    fn get_mut(&mut self, idx: usize) -> &mut Dyn {
        VecStorage::<T, Dyn>::get_dyn_mut(self, idx)
    }

    /// Removes the value at index via swap-remove and returns it as a boxed trait object.
    fn take_boxed(&mut self, idx: usize) -> Box<Dyn> {
        VecStorage::<T, Dyn>::take_boxed(self, idx)
    }

    /// Removes the value at index via swap-remove, discarding it.
    fn swap_remove(&mut self, idx: usize) {
        VecStorage::<T, Dyn>::swap_remove(self, idx);
    }

    /// Provides access to self as `&dyn Any` for downcasting to the concrete storage type.
    fn as_storage_any(&self) -> &dyn Any {
        self
    }

    /// Provides mutable access to self as `&mut dyn Any` for downcasting.
    fn as_storage_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker type for the vector storage family.
pub struct VecFamily;

// =============================================================================
// Type-Erased Vector Storage
// =============================================================================

/// Per-type function table for an [`ErasedVecStorage`].
///
/// Carries only data (function pointers) so it can be assembled by whichever
/// module defines the concrete element type and stored inside a column that
/// outlives that module. The table can be replaced wholesale via
/// [`ErasedVecStorage::refresh_ops`] when the pointers must be re-pointed at
/// code that is still alive.
pub struct ErasedVecStorageOps<Dyn: ?Sized> {
    /// Drop `count` initialized elements starting at `ptr`.
    pub drop_range: unsafe fn(*mut u8, usize),
    /// Upcast the element at `ptr` to a shared `Dyn` reference pointer.
    pub up_ref: unsafe fn(*const u8) -> *const Dyn,
    /// Upcast the element at `ptr` to a mutable `Dyn` reference pointer.
    pub up_mut: unsafe fn(*mut u8) -> *mut Dyn,
    /// Move the element at `index` out of `ptr` and box it as `Dyn`.
    pub take_boxed: unsafe fn(*mut u8, usize) -> Box<Dyn>,
}

// Function pointers are always `Copy` regardless of their signature, so the
// table is copyable even though `Dyn` is unsized and unbounded.
impl<Dyn: ?Sized> Clone for ErasedVecStorageOps<Dyn> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Dyn: ?Sized> Copy for ErasedVecStorageOps<Dyn> {}

impl<Dyn: ?Sized + 'static> ErasedVecStorageOps<Dyn> {
    /// Assemble the function table for a concrete element type.
    ///
    /// The returned table is plain data; it is stored in the column and never
    /// monomorphized into a persistent trait-object vtable.
    pub fn of<T: 'static + TraitAccessible<Dyn>>() -> Self {
        Self {
            drop_range: drop_range_of::<T>,
            up_ref: up_ref_of::<T, Dyn>,
            up_mut: up_mut_of::<T, Dyn>,
            take_boxed: take_boxed_of::<T, Dyn>,
        }
    }
}

/// Runtime description of an erased column, assembled by the caller that knows
/// the concrete element type so the column can be built without monomorphizing
/// that type into the storage backend.
pub struct ErasedVecStorageInfo<Dyn: ?Sized> {
    /// Runtime type identity of the stored element.
    pub type_id: TypeId,
    /// Size in bytes of one element.
    pub size: usize,
    /// Alignment in bytes of one element.
    pub align: usize,
    /// Whether the column's element type is identified by its layout rather
    /// than by [`TypeId`]; see [`ErasedVecStorage`] for what that trades away.
    pub shared_identity: bool,
    /// Per-type function table.
    pub ops: ErasedVecStorageOps<Dyn>,
}

impl<Dyn: ?Sized> Clone for ErasedVecStorageInfo<Dyn> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Dyn: ?Sized> Copy for ErasedVecStorageInfo<Dyn> {}

impl<Dyn: ?Sized + 'static> ErasedVecStorageInfo<Dyn> {
    /// Build a column description for a concrete element type.
    ///
    /// The column identifies its element by `TypeId`, which is the strict
    /// default: only the exact `T` this was built from can read or write it.
    pub fn of<T: 'static + TraitAccessible<Dyn>>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            shared_identity: false,
            ops: ErasedVecStorageOps::<Dyn>::of::<T>(),
        }
    }

    /// Build a column description whose element type is identified by its
    /// **layout** (size and alignment) rather than by `TypeId`.
    ///
    /// This exists for one situation: a single type that has been compiled
    /// into two binaries loaded in one process. `TypeId` is a hash over the
    /// crate name, its `-C metadata` disambiguator and the type path, computed
    /// per compilation unit, so each binary gets a different `TypeId` for what
    /// the programmer wrote as one type. A column shared between them cannot
    /// be keyed on either.
    ///
    /// The caller takes on the obligation `TypeId` was discharging: that every
    /// `T` used with this column really is the same type. Matching size and
    /// alignment is necessary but not sufficient - `{f32, f32}` and
    /// `{u32, u32}` agree on both - so the caller must verify the full field
    /// layout before creating the column, and only then is the reduced check
    /// here sound.
    pub fn of_shared<T: 'static + TraitAccessible<Dyn>>() -> Self {
        Self {
            shared_identity: true,
            ..Self::of::<T>()
        }
    }
}

/// Drop `count` values of type `T` starting at `ptr`.
///
/// # Safety
/// `ptr` must point to `count` live, correctly aligned `T` values.
unsafe fn drop_range_of<T>(ptr: *mut u8, count: usize) {
    if count == 0 {
        return;
    }
    // SAFETY: guaranteed by the caller (column invariants).
    let slice = std::ptr::slice_from_raw_parts_mut(ptr.cast::<T>(), count);
    // SAFETY: dropping the slice drops each element exactly once.
    unsafe { std::ptr::drop_in_place(slice) };
}

/// Upcast the `T` at `ptr` to a shared `Dyn` reference pointer.
///
/// # Safety
/// `ptr` must point to a live, aligned `T` value.
unsafe fn up_ref_of<T, Dyn: ?Sized>(ptr: *const u8) -> *const Dyn
where
    T: TraitAccessible<Dyn> + 'static,
{
    // SAFETY: guaranteed by the caller (column invariants).
    let value = unsafe { &*ptr.cast::<T>() };
    (T::get_accessor().up_ref)(value)
}

/// Upcast the `T` at `ptr` to a mutable `Dyn` reference pointer.
///
/// # Safety
/// `ptr` must point to a live, aligned `T` value.
unsafe fn up_mut_of<T, Dyn: ?Sized>(ptr: *mut u8) -> *mut Dyn
where
    T: TraitAccessible<Dyn> + 'static,
{
    // SAFETY: guaranteed by the caller (column invariants).
    let value = unsafe { &mut *ptr.cast::<T>() };
    (T::get_accessor().up_mut)(value)
}

/// Move the `T` at `index` out of `ptr` and box it as `Dyn`.
///
/// # Safety
/// `ptr + index * size_of::<T>()` must point to a live `T` value.
unsafe fn take_boxed_of<T, Dyn: ?Sized>(ptr: *mut u8, index: usize) -> Box<Dyn>
where
    T: TraitAccessible<Dyn> + 'static,
{
    // SAFETY: guaranteed by the caller (column invariants).
    let value = unsafe { std::ptr::read(ptr.add(index * std::mem::size_of::<T>()).cast::<T>()) };
    (T::get_accessor().up_box)(value)
}

/// Type-erased contiguous storage column whose element type is opaque.
///
/// Unlike [`VecStorage`], the concrete column type is **not** generic over the
/// element type, so a map can store it as a concrete `Box<ErasedVecStorage>`
/// without any trait-object vtable. Per-type behavior (drop, upcast, boxing)
/// is carried by an [`ErasedVecStorageOps`] function table supplied by the
/// caller; callers that store columns across module boundaries can refresh the
/// table via [`ErasedVecStorage::refresh_ops`] so the pointers always reference
/// code that is still alive.
pub struct ErasedVecStorage<Dyn: ?Sized> {
    /// Runtime type identity of the stored element.
    type_id: TypeId,
    /// Size in bytes of one element.
    elem_size: usize,
    /// Alignment in bytes of one element.
    elem_align: usize,
    /// Whether element-type checks compare layout instead of `type_id`; set
    /// through [`ErasedVecStorageInfo::of_shared`].
    shared_identity: bool,
    /// Heap allocation holding the rows (dangling before the first growth).
    data: NonNull<u8>,
    /// Number of initialized rows.
    len: usize,
    /// Number of rows the current allocation can hold.
    capacity: usize,
    /// Per-type function table; replaceable via [`ErasedVecStorage::refresh_ops`].
    ops: ErasedVecStorageOps<Dyn>,
}

// SAFETY: Rows are written and read with disjoint row ownership during
// parallel iteration. The function table is plain data and type-specific
// functions run under the same contract as the rest of this crate's storage.
unsafe impl<Dyn: ?Sized> Send for ErasedVecStorage<Dyn> {}
unsafe impl<Dyn: ?Sized> Sync for ErasedVecStorage<Dyn> {}

impl<Dyn: ?Sized + 'static> ErasedVecStorage<Dyn> {
    /// Creates an empty column from a type description.
    pub fn new(info: ErasedVecStorageInfo<Dyn>) -> Self {
        Self {
            type_id: info.type_id,
            elem_size: info.size,
            elem_align: info.align,
            shared_identity: info.shared_identity,
            data: NonNull::dangling(),
            len: 0,
            capacity: 0,
            ops: info.ops,
        }
    }

    /// Returns the number of stored rows.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no rows are stored.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the runtime type identity of the stored element.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the stored per-type function table.
    pub fn ops(&self) -> ErasedVecStorageOps<Dyn> {
        self.ops
    }

    /// Whether this column identifies its element type by layout rather than
    /// by `TypeId`; see [`ErasedVecStorageInfo::of_shared`].
    pub fn has_shared_identity(&self) -> bool {
        self.shared_identity
    }

    /// Check that `T` is an acceptable element type for this column, panicking
    /// with `method` named if it is not.
    ///
    /// An ordinary column demands the exact `TypeId` it was built from. A
    /// shared-identity column cannot: its whole purpose is to be reached from
    /// a second binary, whose `TypeId` for the same type differs. It checks
    /// layout instead, which catches an outright wrong `T` while accepting the
    /// other binary's copy of the right one.
    #[inline]
    fn assert_element_type<T: 'static>(&self, method: &str) {
        if self.shared_identity {
            assert!(
                std::mem::size_of::<T>() == self.elem_size
                    && std::mem::align_of::<T>() == self.elem_align,
                "ErasedVecStorage::{method} with a type whose layout does not match the                  shared column ({} bytes / {} align against the column's {} / {})",
                std::mem::size_of::<T>(),
                std::mem::align_of::<T>(),
                self.elem_size,
                self.elem_align,
            );
        } else {
            assert_eq!(
                self.type_id,
                TypeId::of::<T>(),
                "ErasedVecStorage::{method} with mismatched type"
            );
        }
    }

    /// Replace the per-type function table.
    ///
    /// Lets the owner re-point the stored function pointers at still-alive
    /// code (for example after the module that supplied the table is
    /// reloaded) without touching the rows.
    pub fn refresh_ops(&mut self, ops: ErasedVecStorageOps<Dyn>) {
        self.ops = ops;
    }

    /// Returns a shared reference to the row at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `T` does not match the column's type or `index` is out of
    /// bounds.
    pub fn get<T: 'static>(&self, index: usize) -> &T {
        self.assert_element_type::<T>("get");
        assert!(
            index < self.len,
            "ErasedVecStorage::get index out of bounds"
        );
        // SAFETY: verified above.
        unsafe { self.get_unchecked::<T>(index) }
    }

    /// Returns a mutable reference to the row at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `T` does not match the column's type or `index` is out of
    /// bounds.
    pub fn get_mut<T: 'static>(&mut self, index: usize) -> &mut T {
        self.assert_element_type::<T>("get_mut");
        assert!(
            index < self.len,
            "ErasedVecStorage::get_mut index out of bounds"
        );
        // SAFETY: verified above.
        unsafe { self.get_mut_unchecked::<T>(index) }
    }

    /// Returns a shared reference to the row at `index` without checks.
    ///
    /// # Safety
    /// `index` must be in bounds and `T` must match the column's type.
    pub unsafe fn get_unchecked<T: 'static>(&self, index: usize) -> &T {
        // SAFETY: guaranteed by the caller.
        unsafe { &*self.data.as_ptr().add(index * self.elem_size).cast::<T>() }
    }

    /// Returns a mutable reference to the row at `index` without checks.
    ///
    /// # Safety
    /// `index` must be in bounds and `T` must match the column's type.
    pub unsafe fn get_mut_unchecked<T: 'static>(&mut self, index: usize) -> &mut T {
        // SAFETY: guaranteed by the caller.
        unsafe { &mut *self.data.as_ptr().add(index * self.elem_size).cast::<T>() }
    }

    /// Appends a value to the column.
    ///
    /// # Panics
    ///
    /// Panics when `T` does not match the column's type.
    pub fn push<T: 'static>(&mut self, value: T) {
        self.assert_element_type::<T>("push");
        self.reserve_one();
        // SAFETY: reserve_one guarantees one writable, aligned slot.
        unsafe {
            std::ptr::write(
                self.data
                    .as_ptr()
                    .add(self.len * self.elem_size)
                    .cast::<T>(),
                value,
            )
        };
        self.len += 1;
    }

    /// Removes and returns the row at `index` by swapping in the last row.
    ///
    /// # Panics
    ///
    /// Panics when `T` does not match the column's type or `index` is out of
    /// bounds.
    pub fn swap_remove<T: 'static>(&mut self, index: usize) -> T {
        self.assert_element_type::<T>("swap_remove");
        assert!(
            index < self.len,
            "ErasedVecStorage::swap_remove index out of bounds"
        );
        // SAFETY: bounds checked above.
        unsafe {
            let value = std::ptr::read(self.data.as_ptr().add(index * self.elem_size).cast::<T>());
            if index + 1 < self.len {
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add((self.len - 1) * self.elem_size),
                    self.data.as_ptr().add(index * self.elem_size),
                    self.elem_size,
                );
            }
            self.len -= 1;
            value
        }
    }

    /// Returns an iterator over the stored rows.
    pub fn iter<T: 'static>(&self) -> impl Iterator<Item = &T> {
        // SAFETY: the rows form a contiguous, correctly aligned `T` slice.
        let slice = unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast::<T>(), self.len) };
        slice.iter()
    }

    /// Reserves capacity for at least `additional` more rows.
    pub fn reserve<T: 'static>(&mut self, additional: usize) {
        self.assert_element_type::<T>("reserve");
        if self.capacity.saturating_sub(self.len) < additional {
            self.grow_to(self.len.saturating_add(additional));
        }
    }

    /// Returns a shared slice over the stored rows.
    pub fn as_slice<T: 'static>(&self) -> &[T] {
        // SAFETY: rows form a contiguous, aligned `T` slice.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast::<T>(), self.len) }
    }

    /// Returns a mutable slice over the stored rows.
    pub fn as_mut_slice<T: 'static>(&mut self) -> &mut [T] {
        // SAFETY: rows form a contiguous, aligned `T` slice.
        unsafe { std::slice::from_raw_parts_mut(self.data.as_ptr().cast::<T>(), self.len) }
    }

    /// Returns a raw pointer to the first row.
    pub fn as_ptr<T: 'static>(&self) -> *const T {
        self.data.as_ptr().cast::<T>()
    }

    // =========================================================================
    // Type-erased byte access
    //
    // The engine exposes native columns to the C# backend as raw byte chunks
    // (mirroring the dynamic-column path). These methods are project-agnostic:
    // they only re-expose facts the column already owns (element size and the
    // raw row buffer) plus a byte push for deferred commands. The caller is
    // responsible for matching the element size and not retaining the pointer
    // beyond the active managed-system invocation.
    // =========================================================================

    /// Size in bytes of one stored row.
    pub fn elem_size(&self) -> usize {
        self.elem_size
    }

    /// Alignment in bytes of one stored row.
    pub fn elem_align(&self) -> usize {
        self.elem_align
    }

    /// Returns a raw pointer to the first row, without a concrete element type.
    ///
    /// The pointer stays valid until the column grows or is dropped; callers
    /// must not retain it beyond the scope that owns the column.
    pub fn raw_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Returns a mutable raw pointer to the first row, without a concrete
    /// element type.
    ///
    /// The pointer stays valid until the column grows or is dropped; callers
    /// must not retain it beyond the scope that owns the column.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// Appends one raw row copied from `src`.
    ///
    /// # Panics
    ///
    /// Panics when `size` does not match the column's element size.
    pub fn push_bytes(&mut self, src: *const u8, size: usize) {
        assert_eq!(
            size, self.elem_size,
            "ErasedVecStorage::push_bytes with a size that does not match the element size"
        );
        self.reserve_one();
        // SAFETY: reserve_one guarantees one writable, aligned slot of
        // `elem_size` bytes; the caller must supply `size == elem_size` valid
        // bytes, which is asserted above.
        unsafe {
            std::ptr::copy_nonoverlapping(
                src,
                self.data.as_ptr().add(self.len * self.elem_size),
                self.elem_size,
            );
        }
        self.len += 1;
    }

    /// Grows the buffer to hold at least one more row.
    fn reserve_one(&mut self) {
        if self.len == self.capacity {
            let new_capacity = if self.capacity == 0 {
                4
            } else {
                self.capacity * 2
            };
            self.grow_to(new_capacity);
        }
    }

    /// Reallocates the buffer to hold at least `min_capacity` rows, moving the
    /// initialized prefix.
    fn grow_to(&mut self, min_capacity: usize) {
        let new_capacity = min_capacity.max(self.capacity * 2).max(4);
        let new_layout = Layout::from_size_align(new_capacity * self.elem_size, self.elem_align)
            .expect("invalid erased column layout");
        let new_ptr = unsafe { std::alloc::alloc(new_layout) };
        if new_ptr.is_null() {
            std::alloc::handle_alloc_error(new_layout);
        }
        if self.capacity > 0 {
            // SAFETY: move the initialized prefix into the fresh allocation,
            // then release the old one.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr(),
                    new_ptr,
                    self.len * self.elem_size,
                );
                let old_layout =
                    Layout::from_size_align(self.capacity * self.elem_size, self.elem_align)
                        .expect("invalid old erased column layout");
                std::alloc::dealloc(self.data.as_ptr(), old_layout);
            }
        }
        self.data = NonNull::new(new_ptr).expect("alloc returned null");
        self.capacity = new_capacity;
    }
}

impl<Dyn: ?Sized + 'static> ErasedVecStorage<Dyn> {
    /// Returns a trait-object reference to the value at index via the stored
    /// per-type upcast function.
    ///
    /// # Panics
    ///
    /// Panics when `index` is out of bounds.
    pub fn get_dyn(&self, idx: usize) -> &Dyn {
        assert!(
            idx < self.len,
            "ErasedVecStorage::get_dyn index out of bounds"
        );
        // SAFETY: bounds checked; the function table is refreshed so the
        // upcast function references still-alive code.
        let ptr = unsafe { (self.ops.up_ref)(self.data.as_ptr().add(idx * self.elem_size)) };
        unsafe { &*ptr }
    }

    /// Returns a mutable trait-object reference to the value at index.
    ///
    /// # Panics
    ///
    /// Panics when `index` is out of bounds.
    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut Dyn {
        assert!(
            idx < self.len,
            "ErasedVecStorage::get_dyn_mut index out of bounds"
        );
        // SAFETY: bounds checked; see [`ErasedVecStorage::get_dyn`].
        let ptr = unsafe { (self.ops.up_mut)(self.data.as_ptr().add(idx * self.elem_size)) };
        unsafe { &mut *ptr }
    }

    /// Removes the value at index via swap-remove and returns it boxed as
    /// `Dyn`, discarding the row.
    pub fn take_boxed_dyn(&mut self, idx: usize) -> Box<Dyn> {
        assert!(
            idx < self.len,
            "ErasedVecStorage::take_boxed_dyn index out of bounds"
        );
        // SAFETY: the function table reads the value out of the slot; the
        // swap-remove below keeps the column dense.
        let result = unsafe { (self.ops.take_boxed)(self.data.as_ptr(), idx) };
        if idx + 1 < self.len {
            // SAFETY: move the last row's bytes into the vacated slot; the
            // old last slot is considered uninitialized after the length
            // decrement.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add((self.len - 1) * self.elem_size),
                    self.data.as_ptr().add(idx * self.elem_size),
                    self.elem_size,
                );
            }
        }
        self.len -= 1;
        result
    }

    /// Removes the value at index via swap-remove, discarding it.
    pub fn swap_remove_discard(&mut self, idx: usize) {
        assert!(
            idx < self.len,
            "ErasedVecStorage::swap_remove_discard index out of bounds"
        );
        // SAFETY: the function table drops the value at the slot, then the
        // last row is moved into the vacated slot.
        unsafe {
            (self.ops.drop_range)(self.data.as_ptr().add(idx * self.elem_size), 1);
            if idx + 1 < self.len {
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add((self.len - 1) * self.elem_size),
                    self.data.as_ptr().add(idx * self.elem_size),
                    self.elem_size,
                );
            }
        }
        self.len -= 1;
    }
}

impl<Dyn: ?Sized> Drop for ErasedVecStorage<Dyn> {
    fn drop(&mut self) {
        if self.capacity == 0 {
            return;
        }
        // SAFETY: drop the initialized rows through the per-type function,
        // then release the allocation.
        unsafe {
            (self.ops.drop_range)(self.data.as_ptr(), self.len);
            let layout = Layout::from_size_align(self.capacity * self.elem_size, self.elem_align)
                .expect("valid erased column layout");
            std::alloc::dealloc(self.data.as_ptr(), layout);
        }
    }
}

// =============================================================================
// Vector Option Backend
// =============================================================================

/// Storage for multiple optional values of a single type in a vector.
///
/// Values can be accessed by index, and removed values leave `None` in their place.
pub struct VecOptionStorage<T, Dyn: ?Sized> {
    pub data: Vec<Option<T>>,
    trait_accessor: TraitAccessor<T, Dyn>,
    //// Cached count of non-None elements for O(1) len()
    count: usize,
}
impl<T, Dyn: ?Sized> VecOptionStorage<T, Dyn> {
    /// Creates a new empty option storage backed by the given upcast accessor.
    pub fn new(trait_accessor: TraitAccessor<T, Dyn>) -> Self {
        Self {
            data: Vec::new(),
            trait_accessor,
            count: 0,
        }
    }

    /// Appends a value wrapped in Some and returns its index. Increments the non-None count.
    #[inline(always)]
    pub fn push(&mut self, v: T) -> usize {
        let idx = self.data.len();
        self.data.push(Some(v));
        self.count += 1;
        idx
    }

    /// Returns an iterator over all present (non-None) values.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter().filter_map(|o| o.as_ref())
    }

    /// Returns a shared reference to the value at index, or None if the slot was cleared.
    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<&T> {
        self.data.get(i).and_then(|o| o.as_ref())
    }

    /// Returns a mutable reference to the value at index, or None if the slot was cleared.
    #[inline(always)]
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.data.get_mut(i).and_then(|o| o.as_mut())
    }

    /// Removes the value at index in-place (leaves None) and decrements the non-None count.
    #[inline(always)]
    pub fn take(&mut self, i: usize) -> Option<T> {
        let result = self.data.get_mut(i).and_then(|o| o.take());
        if result.is_some() {
            self.count -= 1;
        }
        result
    }

    /// Returns a trait object reference to the value at index if the slot is occupied.
    #[inline(always)]
    pub fn get_dyn(&self, i: usize) -> Option<&Dyn> {
        self.get(i).map(|v| (self.trait_accessor.up_ref)(v))
    }

    /// Returns a mutable trait object reference to the value at index if the slot is occupied.
    #[inline(always)]
    pub fn get_dyn_mut(&mut self, i: usize) -> Option<&mut Dyn> {
        let up_mut = self.trait_accessor.up_mut;
        self.get_mut(i).map(up_mut)
    }

    /// Removes the value at index and returns it boxed as a trait object if the slot was occupied.
    #[inline(always)]
    pub fn take_boxed(&mut self, i: usize) -> Option<Box<Dyn>> {
        self.take(i).map(|v| (self.trait_accessor.up_box)(v))
    }

    /// Removes the slot at index via swap-remove; does not update the non-None count.
    pub fn swap_remove(&mut self, i: usize) -> Option<T> {
        self.data.swap_remove(i)
    }
}

/// Trait object interface for vector option storage.
///
/// This allows accessing stored values as trait objects without knowing the concrete type.
pub trait TraitVecOptionStorage<Dyn: ?Sized>: Any {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, idx: usize) -> Option<&Dyn>;
    fn get_mut(&mut self, idx: usize) -> Option<&mut Dyn>;
    fn take_boxed(&mut self, idx: usize) -> Option<Box<Dyn>>;
    fn swap_remove(&mut self, idx: usize);
    fn as_storage_any(&self) -> &dyn Any;
    fn as_storage_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: 'static, Dyn: ?Sized + 'static> TraitVecOptionStorage<Dyn> for VecOptionStorage<T, Dyn> {
    /// Returns the number of present (non-None) values using the cached counter.
    fn len(&self) -> usize {
        self.count
    }

    /// Returns a trait object reference to the value at index if the slot is occupied.
    #[inline]
    fn get(&self, idx: usize) -> Option<&Dyn> {
        VecOptionStorage::<T, Dyn>::get_dyn(self, idx)
    }

    /// Returns a mutable trait object reference to the value at index if the slot is occupied.
    #[inline]
    fn get_mut(&mut self, idx: usize) -> Option<&mut Dyn> {
        VecOptionStorage::<T, Dyn>::get_dyn_mut(self, idx)
    }

    /// Removes the value at index and returns it boxed as a trait object if occupied.
    #[inline]
    fn take_boxed(&mut self, idx: usize) -> Option<Box<Dyn>> {
        VecOptionStorage::<T, Dyn>::take_boxed(self, idx)
    }

    /// Removes the slot at index via swap-remove without updating the non-None count.
    fn swap_remove(&mut self, idx: usize) {
        self.data.swap_remove(idx);
    }

    /// Provides access to self as `&dyn Any` for downcasting to the concrete storage type.
    fn as_storage_any(&self) -> &dyn Any {
        self
    }

    /// Provides mutable access to self as `&mut dyn Any` for downcasting.
    fn as_storage_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker type for the vector option storage family.
pub struct VecOptionFamily;

// =============================================================================
// Single Option Backend
// =============================================================================

/// Storage for a single optional value of a type.
pub struct OptionStorage<T, Dyn: ?Sized> {
    pub data: Option<T>,
    trait_accessor: TraitAccessor<T, Dyn>,
}
impl<T, Dyn: ?Sized> OptionStorage<T, Dyn> {
    /// Creates a new empty single-slot storage backed by the given upcast accessor.
    pub fn new(trait_accessor: TraitAccessor<T, Dyn>) -> Self {
        Self {
            data: None,
            trait_accessor,
        }
    }

    /// Stores a value, replacing any previously held value.
    #[inline(always)]
    pub fn set(&mut self, v: T) {
        self.data = Some(v);
    }

    /// Returns a shared reference to the stored value if present.
    #[inline(always)]
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Returns a mutable reference to the stored value if present.
    #[inline(always)]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    /// Removes and returns the stored value, leaving the slot empty.
    #[inline(always)]
    pub fn take(&mut self) -> Option<T> {
        self.data.take()
    }

    /// Returns true if a value is currently stored.
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.data.is_some()
    }

    /// Returns a trait object reference to the stored value if present.
    #[inline(always)]
    pub fn get_dyn(&self) -> Option<&Dyn> {
        self.get().map(|v| (self.trait_accessor.up_ref)(v))
    }

    /// Returns a mutable trait object reference to the stored value if present.
    #[inline(always)]
    pub fn get_dyn_mut(&mut self) -> Option<&mut Dyn> {
        let up_mut = self.trait_accessor.up_mut;
        self.get_mut().map(up_mut)
    }

    /// Removes the stored value and returns it boxed as a trait object if present.
    #[inline(always)]
    pub fn take_boxed(&mut self) -> Option<Box<Dyn>> {
        self.take().map(|v| (self.trait_accessor.up_box)(v))
    }
}

/// Trait object interface for single-value storage.
///
/// This allows accessing the stored value as a trait object without knowing the concrete type.
pub trait TraitOptionStorage<Dyn: ?Sized>: Any {
    fn is_some(&self) -> bool;
    fn get(&self) -> Option<&Dyn>;
    fn get_mut(&mut self) -> Option<&mut Dyn>;
    fn take_boxed(&mut self) -> Option<Box<Dyn>>;
    fn as_storage_any(&self) -> &dyn Any;
    fn as_storage_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: 'static, Dyn: ?Sized + 'static> TraitOptionStorage<Dyn> for OptionStorage<T, Dyn> {
    /// Returns true if a value is currently held.
    fn is_some(&self) -> bool {
        self.is_some()
    }

    /// Returns a trait object reference to the stored value if present.
    #[inline]
    fn get(&self) -> Option<&Dyn> {
        OptionStorage::<T, Dyn>::get_dyn(self)
    }

    /// Returns a mutable trait object reference to the stored value if present.
    #[inline]
    fn get_mut(&mut self) -> Option<&mut Dyn> {
        OptionStorage::<T, Dyn>::get_dyn_mut(self)
    }

    /// Removes and returns the stored value boxed as a trait object if present.
    #[inline]
    fn take_boxed(&mut self) -> Option<Box<Dyn>> {
        OptionStorage::<T, Dyn>::take_boxed(self)
    }

    /// Provides access to self as `&dyn Any` for downcasting to the concrete storage type.
    fn as_storage_any(&self) -> &dyn Any {
        self
    }

    /// Provides mutable access to self as `&mut dyn Any` for downcasting.
    fn as_storage_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker type for the single-value storage family.
pub struct OptionFamily;

// =============================================================================
// Storage Family Binding
// =============================================================================

/// Storage family trait that determines how values are stored. The family trait is generic over the trait object `Dyn`.
/// Each impl chooses its trait type (`dyn TraitVecStorage<Dyn>`, `dyn TraitVecOptionStorage<Dyn>`, or `dyn TraitOptionStorage<Dyn>`)
/// and its typed storage (`VecStorage<T, Dyn>`, `VecOptionStorage<T, Dyn>`, or `OptionStorage<T, Dyn>`).
pub trait StorageFamily<Dyn: ?Sized + 'static> {
    type Trait: ?Sized + 'static;
    type Storage<T: 'static>: 'static;

    /// Allocate a storage column for `T`.
    ///
    /// `ops` carries the per-type function table required by the
    /// [`ErasedVecStorage`] backend; the option/option-vector families ignore
    /// it.
    fn make<T: 'static>(
        trait_accessor: TraitAccessor<T, Dyn>,
        ops: Option<ErasedVecStorageOps<Dyn>>,
    ) -> Box<Self::Trait>;
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T>;
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T>;
}

impl<D: ?Sized + 'static> StorageFamily<D> for VecFamily {
    // The storage is intentionally the CONCRETE `ErasedVecStorage`: storing it
    // as `Box<ErasedVecStorage<D>>` (instead of `Box<dyn TraitVecStorage<D>>`)
    // means the column carries no trait-object vtable at all, so nothing can
    // dangle when the code that supplied its function table is unloaded.
    type Trait = ErasedVecStorage<D>;
    type Storage<T: 'static> = ErasedVecStorage<D>;

    /// Allocates a new empty erased column for type T.
    fn make<T: 'static>(
        _trait_accessor: TraitAccessor<T, D>,
        ops: Option<ErasedVecStorageOps<D>>,
    ) -> Box<Self::Trait> {
        let ops = ops.expect("VecFamily storage requires ErasedVecStorageOps");
        Box::new(ErasedVecStorage::<D>::new(ErasedVecStorageInfo {
            type_id: TypeId::of::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            // `register_type_storage` keys the map on `TypeId::of::<T>()`, so a
            // column built through it is reached by exactly that type; shared
            // identity is opted into by building the column directly.
            shared_identity: false,
            ops,
        }))
    }

    /// Returns the erased column directly; no downcast is needed.
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e
    }

    /// Returns the erased column directly; no downcast is needed.
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e
    }
}

impl<D: ?Sized + 'static> StorageFamily<D> for VecOptionFamily {
    type Trait = dyn TraitVecOptionStorage<D>;
    type Storage<T: 'static> = VecOptionStorage<T, D>;

    /// Allocates a new empty VecOptionStorage for type T with the appropriate upcast accessor.
    fn make<T: 'static>(
        trait_accessor: TraitAccessor<T, D>,
        _ops: Option<ErasedVecStorageOps<D>>,
    ) -> Box<Self::Trait> {
        Box::new(VecOptionStorage::<T, D>::new(trait_accessor))
    }

    /// Downcasts the trait object to a concrete VecOptionStorage<T, D> for typed read access.
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e.as_storage_any()
            .downcast_ref::<VecOptionStorage<T, D>>()
            .expect("wrong T for VecOptionFamily")
    }

    /// Downcasts the trait object to a concrete VecOptionStorage<T, D> for typed write access.
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e.as_storage_any_mut()
            .downcast_mut::<VecOptionStorage<T, D>>()
            .expect("wrong T for VecOptionFamily")
    }
}

impl<D: ?Sized + 'static> StorageFamily<D> for OptionFamily {
    type Trait = dyn TraitOptionStorage<D>;
    type Storage<T: 'static> = OptionStorage<T, D>;

    /// Allocates a new empty OptionStorage for type T with the appropriate upcast accessor.
    fn make<T: 'static>(
        trait_accessor: TraitAccessor<T, D>,
        _ops: Option<ErasedVecStorageOps<D>>,
    ) -> Box<Self::Trait> {
        Box::new(OptionStorage::<T, D>::new(trait_accessor))
    }

    /// Downcasts the trait object to a concrete OptionStorage<T, D> for typed read access.
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e.as_storage_any()
            .downcast_ref::<OptionStorage<T, D>>()
            .expect("wrong T for OptionFamily")
    }

    /// Downcasts the trait object to a concrete OptionStorage<T, D> for typed write access.
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e.as_storage_any_mut()
            .downcast_mut::<OptionStorage<T, D>>()
            .expect("wrong T for OptionFamily")
    }
}

// =============================================================================
// One Map Type
// =============================================================================

/// Trait for types that can be accessed via a trait object.
///
/// Types implementing this trait can be stored in a `TraitTypeMap`.
/// Use the `impl_trait_accessible!` macro to implement this trait.
pub trait TraitAccessible<Dyn: ?Sized> {
    fn get_accessor() -> TraitAccessor<Self, Dyn>
    where
        Self: Sized;
}

/// A type-indexed map for storing values implementing a specific trait.
///
/// # Type Parameters
///
/// - `Dyn`: The trait object type (e.g., `dyn MyTrait`)
/// - `F`: The storage family (`VecFamily`, `VecOptionFamily`, or `OptionFamily`)
///
/// # Examples
///
/// ```rust
/// use trait_type_map::{impl_trait_accessible, TraitTypeMap, OptionFamily};
///
/// trait Animal {
///     fn name(&self) -> &str;
/// }
///
/// struct Dog { name: String }
/// impl Animal for Dog {
///     fn name(&self) -> &str { &self.name }
/// }
///
/// impl_trait_accessible!(dyn Animal; Dog);
///
/// let mut map: TraitTypeMap<dyn Animal, OptionFamily> = TraitTypeMap::new();
/// map.register_type_storage::<Dog>();
/// map.get_storage_mut::<Dog>().set(Dog { name: "Rex".into() });
///
/// if let Some(dog) = map.get_storage::<Dog>().get() {
///     assert_eq!(dog.name(), "Rex");
/// }
/// ```
pub struct TraitTypeMap<Dyn: ?Sized + 'static, F: StorageFamily<Dyn>> {
    entries: AHashMap<TypeId, Box<F::Trait>>,
}

impl<Dyn: ?Sized + 'static, F: StorageFamily<Dyn>> Default for TraitTypeMap<Dyn, F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Dyn: ?Sized + 'static, F: StorageFamily<Dyn>> TraitTypeMap<Dyn, F> {
    /// Creates an empty map with no pre-allocated capacity.
    pub fn new() -> Self {
        Self {
            entries: AHashMap::new(),
        }
    }

    //// Create a new map with pre-allocated capacity for the given number of types.
    //// This can improve performance when you know how many types you'll store.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: AHashMap::with_capacity(capacity),
        }
    }

    /// Registers a new concrete type T and creates its storage column.
    /// Panics if the same type is registered more than once.
    pub fn register_type_storage<T>(&mut self)
    where
        T: 'static + TraitAccessible<Dyn>,
    {
        let id = TypeId::of::<T>();
        let ops = ErasedVecStorageOps::<Dyn>::of::<T>();
        let inserted = self
            .entries
            .insert(id, F::make::<T>(T::get_accessor(), Some(ops)))
            .is_none();
        assert!(inserted, "type already registered");
    }

    /// Returns a shared reference to the concrete storage column for type T.
    /// Panics if T was not registered via `register_type_storage`.
    #[inline(always)]
    pub fn get_storage<T>(&self) -> &F::Storage<T>
    where
        T: 'static,
    {
        let e = self
            .entries
            .get(&TypeId::of::<T>())
            .expect("type not registered");
        F::storage_ref::<T>(&e)
    }

    /// Returns a mutable reference to the concrete storage column for type T.
    /// Panics if T was not registered via `register_type_storage`.
    #[inline(always)]
    pub fn get_storage_mut<T>(&mut self) -> &mut F::Storage<T>
    where
        T: 'static,
    {
        let e = self
            .entries
            .get_mut(&TypeId::of::<T>())
            .expect("type not registered");
        F::storage_mut::<T>(e)
    }

    //// Fetch family-trait storage by TypeId.
    //// - For `VecFamily`: `&dyn TraitVecStorage<Dyn>`
    //// - For `VecOptionFamily`: `&dyn TraitVecOptionStorage<Dyn>`
    //// - For `OptionFamily`: `&dyn TraitOptionStorage<Dyn>`
    #[inline(always)]
    pub fn get_trait_storage(&self, id: TypeId) -> Option<&F::Trait> {
        self.entries.get(&id).map(|boxed| &**boxed)
    }

    /// Returns a mutable type-erased storage for the given TypeId, or None if not registered.
    #[inline(always)]
    pub fn get_trait_storage_mut(&mut self, id: TypeId) -> Option<&mut F::Trait> {
        self.entries.get_mut(&id).map(|boxed| &mut **boxed)
    }

    //// Remove and return trait storage by TypeId.
    #[inline]
    pub fn remove_trait_storage(&mut self, id: TypeId) -> Option<Box<F::Trait>> {
        self.entries.remove(&id)
    }
}

impl<Dyn: ?Sized + 'static> TraitTypeMap<Dyn, VecFamily> {
    /// Insert an erased storage column into the map.
    ///
    /// The map stores the concrete `ErasedVecStorage` directly (no trait-object
    /// coercion), so the column carries no vtable that could dangle when the
    /// code that supplied its function table is unloaded.
    pub fn insert_erased(&mut self, column: ErasedVecStorage<Dyn>) {
        let id = column.type_id();
        let inserted = self.entries.insert(id, Box::new(column)).is_none();
        assert!(inserted, "erased column type already registered");
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    trait Marker {}

    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct Point {
        x: f32,
        y: f32,
    }
    impl Marker for Point {}

    // A second type with the same layout as `Point`, standing in for the copy
    // of one type that a second binary compiled and therefore holds under a
    // different `TypeId`.
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct PointFromAnotherBinary {
        x: f32,
        y: f32,
    }
    impl Marker for PointFromAnotherBinary {}

    // A type whose layout differs, to prove the shared check still rejects.
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct WiderPoint {
        x: f64,
        y: f64,
    }
    impl Marker for WiderPoint {}

    impl_trait_accessible!(dyn Marker; Point, PointFromAnotherBinary, WiderPoint);

    /// Indexing the column's raw base pointer by `n * elem_size` addresses the
    /// same bytes as `get_unchecked(n)`.
    ///
    /// This is the identity that lets a caller hoist the base pointer out of a
    /// row loop and index it with a compile-time stride, which is what makes
    /// erased row access cost the same as typed access.
    #[test]
    fn raw_base_pointer_indexing_matches_get_unchecked() {
        let mut column = ErasedVecStorage::<dyn Marker>::new(ErasedVecStorageInfo::of::<Point>());
        for index in 0..8 {
            column.push(Point {
                x: index as f32,
                y: -(index as f32),
            });
        }

        let base = column.raw_ptr();
        let stride = column.elem_size();
        assert_eq!(stride, std::mem::size_of::<Point>());
        assert_eq!(column.len(), 8);

        for index in 0..column.len() {
            // SAFETY: `index` is below `len`, and the column holds `Point`s.
            let from_accessor = unsafe { column.get_unchecked::<Point>(index) } as *const Point;
            // SAFETY: same allocation, same in-bounds row.
            let from_base = unsafe { base.add(index * stride) } as *const Point;
            assert_eq!(from_accessor, from_base, "row {index} addresses differ");
        }
    }

    /// An ordinary column is reachable only through the exact type it was
    /// built from, even by a type with an identical layout.
    #[test]
    #[should_panic(expected = "mismatched type")]
    fn an_ordinary_column_rejects_a_layout_compatible_other_type() {
        let mut column = ErasedVecStorage::<dyn Marker>::new(ErasedVecStorageInfo::of::<Point>());
        column.push(Point { x: 1.0, y: 2.0 });
        column.get::<PointFromAnotherBinary>(0);
    }

    /// A shared-identity column accepts a different type with the same layout,
    /// which is the whole point: the second binary's copy of one type.
    #[test]
    fn a_shared_column_accepts_another_binarys_copy_of_the_type() {
        let mut column =
            ErasedVecStorage::<dyn Marker>::new(ErasedVecStorageInfo::of_shared::<Point>());
        assert!(column.has_shared_identity());

        // Written as the type the column was built from...
        column.push(Point { x: 1.0, y: 2.0 });
        // ...and written as the other binary's copy of it.
        column.push(PointFromAnotherBinary { x: 3.0, y: 4.0 });

        assert_eq!(column.get::<Point>(0), &Point { x: 1.0, y: 2.0 });
        assert_eq!(
            column.get::<PointFromAnotherBinary>(1),
            &PointFromAnotherBinary { x: 3.0, y: 4.0 }
        );
        // Both views address the same rows.
        assert_eq!(column.get::<PointFromAnotherBinary>(0).x, 1.0);
        assert_eq!(column.get::<Point>(1).x, 3.0);
    }

    /// Relaxing the check to layout does not relax it to nothing: a type whose
    /// size or alignment differs is still rejected.
    #[test]
    #[should_panic(expected = "layout does not match")]
    fn a_shared_column_still_rejects_a_differently_sized_type() {
        let mut column =
            ErasedVecStorage::<dyn Marker>::new(ErasedVecStorageInfo::of_shared::<Point>());
        column.push(Point { x: 1.0, y: 2.0 });
        column.get::<WiderPoint>(0);
    }

    /// Shared identity is opt-in: a column built the ordinary way does not
    /// have it, so nothing acquires the weaker check by accident.
    #[test]
    fn shared_identity_is_off_unless_asked_for() {
        let ordinary = ErasedVecStorage::<dyn Marker>::new(ErasedVecStorageInfo::of::<Point>());
        assert!(!ordinary.has_shared_identity());

        let mut map: TraitTypeMap<dyn Marker, VecFamily> = TraitTypeMap::new();
        map.register_type_storage::<Point>();
        assert!(!map.get_storage::<Point>().has_shared_identity());
    }
}
