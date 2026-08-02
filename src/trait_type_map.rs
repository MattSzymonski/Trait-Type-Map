use ahash::AHashMap;
use std::any::{Any, TypeId};

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

    fn make<T: 'static>(trait_accessor: TraitAccessor<T, Dyn>) -> Box<Self::Trait>;
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T>;
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T>;
}

impl<D: ?Sized + 'static> StorageFamily<D> for VecFamily {
    type Trait = dyn TraitVecStorage<D>;
    type Storage<T: 'static> = VecStorage<T, D>;

    /// Allocates a new empty VecStorage for type T with the appropriate upcast accessor.
    fn make<T: 'static>(trait_accessor: TraitAccessor<T, D>) -> Box<Self::Trait> {
        Box::new(VecStorage::<T, D>::new(trait_accessor))
    }

    /// Downcasts the trait object to a concrete VecStorage<T, D> for typed read access.
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e.as_storage_any()
            .downcast_ref::<VecStorage<T, D>>()
            .expect("wrong T for VecFamily")
    }

    /// Downcasts the trait object to a concrete VecStorage<T, D> for typed write access.
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e.as_storage_any_mut()
            .downcast_mut::<VecStorage<T, D>>()
            .expect("wrong T for VecFamily")
    }
}

impl<D: ?Sized + 'static> StorageFamily<D> for VecOptionFamily {
    type Trait = dyn TraitVecOptionStorage<D>;
    type Storage<T: 'static> = VecOptionStorage<T, D>;

    /// Allocates a new empty VecOptionStorage for type T with the appropriate upcast accessor.
    fn make<T: 'static>(trait_accessor: TraitAccessor<T, D>) -> Box<Self::Trait> {
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
    fn make<T: 'static>(trait_accessor: TraitAccessor<T, D>) -> Box<Self::Trait> {
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
        let inserted = self
            .entries
            .insert(id, F::make::<T>(T::get_accessor()))
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
