use ahash::AHashMap;
use std::any::{Any, TypeId};

/// Accessor functions for converting a concrete type to a trait object.
///
/// This struct contains function pointers that handle upcasting from a concrete type `T`
/// to a trait object `Dyn`.
// ---------- Upcast adapter (user supplies per T) ----------
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

/* ==================== Vector backend ==================== */

/// Storage for multiple values of a single type in a vector.
///
/// Values can be accessed by index, and removed values leave `None` in their place.
pub struct VecOptionStorage<T, Dyn: ?Sized> {
    pub data: Vec<Option<T>>,
    trait_accessor: TraitAccessor<T, Dyn>,
    /// Cached count of non-None elements for O(1) len()
    count: usize,
}
impl<T, Dyn: ?Sized> VecOptionStorage<T, Dyn> {
    pub fn new(trait_accessor: TraitAccessor<T, Dyn>) -> Self {
        Self {
            data: Vec::new(),
            trait_accessor,
            count: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, v: T) -> usize {
        let idx = self.data.len();
        self.data.push(Some(v));
        self.count += 1;
        idx
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter().filter_map(|o| o.as_ref())
    }

    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<&T> {
        self.data.get(i).and_then(|o| o.as_ref())
    }

    #[inline(always)]
    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.data.get_mut(i).and_then(|o| o.as_mut())
    }

    #[inline(always)]
    pub fn take(&mut self, i: usize) -> Option<T> {
        let result = self.data.get_mut(i).and_then(|o| o.take());
        if result.is_some() {
            self.count -= 1;
        }
        result
    }

    #[inline(always)]
    pub fn get_dyn(&self, i: usize) -> Option<&Dyn> {
        self.get(i).map(|v| (self.trait_accessor.up_ref)(v))
    }

    #[inline(always)]
    pub fn get_dyn_mut(&mut self, i: usize) -> Option<&mut Dyn> {
        let up_mut = self.trait_accessor.up_mut;
        self.get_mut(i).map(up_mut)
    }

    #[inline(always)]
    pub fn take_boxed(&mut self, i: usize) -> Option<Box<Dyn>> {
        self.take(i).map(|v| (self.trait_accessor.up_box)(v))
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
    fn get(&self, idx: usize) -> Option<&Dyn>;
    fn get_mut(&mut self, idx: usize) -> Option<&mut Dyn>;
    fn take_boxed(&mut self, idx: usize) -> Option<Box<Dyn>>;
    fn as_storage_any(&self) -> &dyn Any;
    fn as_storage_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: 'static, Dyn: ?Sized + 'static> TraitVecStorage<Dyn> for VecOptionStorage<T, Dyn> {
    #[inline]
    fn len(&self) -> usize {
        self.count
    }

    #[inline]
    fn get(&self, idx: usize) -> Option<&Dyn> {
        VecOptionStorage::<T, Dyn>::get_dyn(self, idx)
    }

    #[inline]
    fn get_mut(&mut self, idx: usize) -> Option<&mut Dyn> {
        VecOptionStorage::<T, Dyn>::get_dyn_mut(self, idx)
    }

    #[inline]
    fn take_boxed(&mut self, idx: usize) -> Option<Box<Dyn>> {
        VecOptionStorage::<T, Dyn>::take_boxed(self, idx)
    }

    fn as_storage_any(&self) -> &dyn Any {
        self
    }

    fn as_storage_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker type for the vector storage family.
pub struct VecFamily;

/* ==================== Single backend ==================== */

/// Storage for a single optional value of a type.
pub struct OptionStorage<T, Dyn: ?Sized> {
    pub data: Option<T>,
    trait_accessor: TraitAccessor<T, Dyn>,
}
impl<T, Dyn: ?Sized> OptionStorage<T, Dyn> {
    pub fn new(trait_accessor: TraitAccessor<T, Dyn>) -> Self {
        Self {
            data: None,
            trait_accessor,
        }
    }

    #[inline(always)]
    pub fn set(&mut self, v: T) {
        self.data = Some(v);
    }

    #[inline(always)]
    pub fn get(&self) -> Option<&T> {
        self.data.as_ref()
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }

    #[inline(always)]
    pub fn take(&mut self) -> Option<T> {
        self.data.take()
    }

    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.data.is_some()
    }

    #[inline(always)]
    pub fn get_dyn(&self) -> Option<&Dyn> {
        self.get().map(|v| (self.trait_accessor.up_ref)(v))
    }

    #[inline(always)]
    pub fn get_dyn_mut(&mut self) -> Option<&mut Dyn> {
        let up_mut = self.trait_accessor.up_mut;
        self.get_mut().map(up_mut)
    }

    #[inline(always)]
    pub fn take_boxed(&mut self) -> Option<Box<Dyn>> {
        self.take().map(|v| (self.trait_accessor.up_box)(v))
    }
}

/// Trait object interface for single-value storage.
///
/// This allows accessing the stored value as a trait object without knowing the concrete type.
pub trait TraitSingleStorage<Dyn: ?Sized>: Any {
    fn is_some(&self) -> bool;
    fn get(&self) -> Option<&Dyn>;
    fn get_mut(&mut self) -> Option<&mut Dyn>;
    fn take_boxed(&mut self) -> Option<Box<Dyn>>;
    fn as_storage_any(&self) -> &dyn Any;
    fn as_storage_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: 'static, Dyn: ?Sized + 'static> TraitSingleStorage<Dyn> for OptionStorage<T, Dyn> {
    #[inline]
    fn is_some(&self) -> bool {
        self.is_some()
    }

    #[inline]
    fn get(&self) -> Option<&Dyn> {
        OptionStorage::<T, Dyn>::get_dyn(self)
    }

    #[inline]
    fn get_mut(&mut self) -> Option<&mut Dyn> {
        OptionStorage::<T, Dyn>::get_dyn_mut(self)
    }

    #[inline]
    fn take_boxed(&mut self) -> Option<Box<Dyn>> {
        OptionStorage::<T, Dyn>::take_boxed(self)
    }

    fn as_storage_any(&self) -> &dyn Any {
        self
    }

    fn as_storage_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker type for the single-value storage family.
pub struct SingleFamily;

/* =============== Family binding (stable) ================= */

/// Storage family trait that determines how values are stored. The family trait is generic over the **trait object** `Dyn`.
/// Each impl chooses its trait trait (`dyn TraitVecStorage<Dyn>` or `dyn TraitSingleStorage<Dyn>`)
/// and its typed storage (`VecOptionStorage<T, Dyn>` or `OptionStorage<T, Dyn>`).
pub trait StorageFamily<Dyn: ?Sized + 'static> {
    type Trait: ?Sized + 'static;
    type Storage<T: 'static>: 'static;

    fn make<T: 'static>(trait_accessor: TraitAccessor<T, Dyn>) -> Box<Self::Trait>;
    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T>;
    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T>;
}

impl<D: ?Sized + 'static> StorageFamily<D> for VecFamily {
    type Trait = dyn TraitVecStorage<D>;
    type Storage<T: 'static> = VecOptionStorage<T, D>;

    fn make<T: 'static>(trait_accessor: TraitAccessor<T, D>) -> Box<Self::Trait> {
        Box::new(VecOptionStorage::<T, D>::new(trait_accessor))
    }

    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e.as_storage_any()
            .downcast_ref::<VecOptionStorage<T, D>>()
            .expect("wrong T for VecFamily")
    }

    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e.as_storage_any_mut()
            .downcast_mut::<VecOptionStorage<T, D>>()
            .expect("wrong T for VecFamily")
    }
}

impl<D: ?Sized + 'static> StorageFamily<D> for SingleFamily {
    type Trait = dyn TraitSingleStorage<D>;
    type Storage<T: 'static> = OptionStorage<T, D>;

    fn make<T: 'static>(trait_accessor: TraitAccessor<T, D>) -> Box<Self::Trait> {
        Box::new(OptionStorage::<T, D>::new(trait_accessor))
    }

    fn storage_ref<T: 'static>(e: &Self::Trait) -> &Self::Storage<T> {
        e.as_storage_any()
            .downcast_ref::<OptionStorage<T, D>>()
            .expect("wrong T for SingleFamily")
    }

    fn storage_mut<T: 'static>(e: &mut Self::Trait) -> &mut Self::Storage<T> {
        e.as_storage_any_mut()
            .downcast_mut::<OptionStorage<T, D>>()
            .expect("wrong T for SingleFamily")
    }
}

/* ===================== One map type ====================== */

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
/// - `F`: The storage family (`VecFamily` or `SingleFamily`)
///
/// # Examples
///
/// ```rust
/// use trait_type_map::{impl_trait_accessible, TraitTypeMap, SingleFamily};
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
/// let mut map: TraitTypeMap<dyn Animal, SingleFamily> = TraitTypeMap::new();
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
    pub fn new() -> Self {
        Self {
            entries: AHashMap::new(),
        }
    }

    /// Create a new map with pre-allocated capacity for the given number of types.
    /// This can improve performance when you know how many types you'll store.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: AHashMap::with_capacity(capacity),
        }
    }

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

    #[inline(always)]
    pub fn get_storage<T>(&self) -> &F::Storage<T>
    where
        T: 'static,
    {
        let e = self
            .entries
            .get(&TypeId::of::<T>())
            .expect("type not registered");
        F::storage_ref::<T>(&**e)
    }

    #[inline(always)]
    pub fn get_storage_mut<T>(&mut self) -> &mut F::Storage<T>
    where
        T: 'static,
    {
        let e = self
            .entries
            .get_mut(&TypeId::of::<T>())
            .expect("type not registered");
        F::storage_mut::<T>(&mut **e)
    }

    /// Fetch family-trait storage by TypeId.
    /// - For `VecFamily`: `&dyn TraitVecStorage<Dyn>`
    /// - For `SingleFamily`: `&dyn TraitSingleStorage<Dyn>`
    #[inline(always)]
    pub fn get_trait_storage(&self, id: TypeId) -> Option<&F::Trait> {
        self.entries.get(&id).map(|b| &**b)
    }

    #[inline(always)]
    pub fn get_trait_storage_mut(&mut self, id: TypeId) -> Option<&mut F::Trait> {
        self.entries.get_mut(&id).map(|b| &mut **b)
    }
}
