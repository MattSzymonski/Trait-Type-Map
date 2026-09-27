# Trait Type Map

A type-indexed map for storing values that implement a specific trait, then retrieving them by concrete type or through the trait.

Each registered type chooses its storage: single value, multiple values, or sparse optional slots.

`ErasedVecFamily` columns are `ErasedVecStorage`, a type-erased column whose per-type behavior (drop, upcast, boxing) comes from a function table. Columns can also be built directly and inserted with `insert_erased`.

## Features

- **Type-safe storage**: Store different types implementing the same trait in one map
- **Flexible storage backends**: Choose `OptionFamily` (single value), `VecFamily` (multiple values), `VecOptionFamily` (sparse optional slots), or `ErasedVecFamily` (type-erased columns) per type
- **Trait object access**: Access stored values as trait objects without knowing the concrete type
- **Type-indexed retrieval**: Retrieve values by their concrete type with one `TypeId` lookup

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
trait_type_map = "1.0.0"
```

## Quick Start

```rust
use trait_type_map::{impl_trait_accessible, TraitTypeMap, VecFamily};

// Define a trait
trait Animal {
    fn speak(&self) -> &str;
}

// Implement the trait for your types
struct Dog;
impl Animal for Dog {
    fn speak(&self) -> &str { "Woof!" }
}

struct Cat;
impl Animal for Cat {
    fn speak(&self) -> &str { "Meow!" }
}

// Register types as accessible via the trait
impl_trait_accessible!(dyn Animal; Dog, Cat);

fn main() {
    // Create a map with vector storage (multiple instances per type)
    let mut map: TraitTypeMap<dyn Animal, VecFamily> = TraitTypeMap::new();
    
    // Register types
    map.register_type_storage::<Dog>();
    map.register_type_storage::<Cat>();
    
    // Store values
    let dog_idx = map.get_storage_mut::<Dog>().push(Dog);
    map.get_storage_mut::<Cat>().push(Cat);
    
    // Access via trait object
    let dog_storage = map.get_storage::<Dog>();
    println!("{}", dog_storage.get_dyn(dog_idx).speak()); // Prints: Woof!
}
```

## Storage Families

### VecFamily - Multiple Values Per Type

Stores multiple instances of each type in a vector:

```rust
use trait_type_map::{TraitTypeMap, VecFamily};

let mut map: TraitTypeMap<dyn Animal, VecFamily> = TraitTypeMap::new();
map.register_type_storage::<Dog>();

let storage = map.get_storage_mut::<Dog>();
let idx1 = storage.push(Dog { name: "Rex".into() });
let idx2 = storage.push(Dog { name: "Buddy".into() });

// Iterate over all dogs
for dog in map.get_storage::<Dog>().iter() {
    println!("{}", dog.name);
}
```

### VecOptionFamily - Sparse Index-Addressed Slots

Cleared slots stay in place as `None`; `len()` counts the values still present:

```rust
use trait_type_map::{TraitTypeMap, VecOptionFamily};

let mut map: TraitTypeMap<dyn Animal, VecOptionFamily> = TraitTypeMap::new();
map.register_type_storage::<Dog>();

let index = map.get_storage_mut::<Dog>().push(Dog);
let dog = map.get_storage_mut::<Dog>().take(index); // Some(Dog), slot stays behind

assert!(map.get_storage::<Dog>().get(index).is_none());
```

### ErasedVecFamily - Type-Erased Columns

Like `VecFamily`, but the column is an `ErasedVecStorage`: a concrete type with no element type in its signature, whose per-type behavior (drop, upcast, boxing) comes from a replaceable function table. Columns can also be built directly and handed to a map with `insert_erased`:

```rust
use trait_type_map::{ErasedVecFamily, ErasedVecStorage, ErasedVecStorageInfo, TraitTypeMap};

// Used as a family, the column behaves like any other vector storage:
let mut map: TraitTypeMap<dyn Animal, ErasedVecFamily> = TraitTypeMap::new();
map.register_type_storage::<Dog>();
map.get_storage_mut::<Dog>().push(Dog);

// Or build the column yourself and insert it:
let mut column = ErasedVecStorage::<dyn Animal>::new(ErasedVecStorageInfo::of::<Dog>());
column.push(Dog);
let mut direct = TraitTypeMap::<dyn Animal, ErasedVecFamily>::new();
direct.insert_erased(column);
```

### OptionFamily - One Value Per Type

Stores at most one instance of each type:

```rust
use trait_type_map::{TraitTypeMap, OptionFamily};

let mut map: TraitTypeMap<dyn Animal, OptionFamily> = TraitTypeMap::new();
map.register_type_storage::<Dog>();

map.get_storage_mut::<Dog>().set(Dog { name: "Max".into() });

if let Some(dog) = map.get_storage::<Dog>().get() {
    println!("{}", dog.name);
}
```

## API Overview

### TraitTypeMap

The main container type, generic over:
- `Dyn`: The trait object type (e.g., `dyn Animal`)
- `F`: The storage family (`VecFamily`, `ErasedVecFamily`, `VecOptionFamily`, or `OptionFamily`)

**Methods:**
- `new()` / `with_capacity(n)` - Create an empty map
- `register_type_storage::<T>()` - Register a type and create its storage
- `get_storage::<T>()` - Get immutable access to a type's storage
- `get_storage_mut::<T>()` - Get mutable access to a type's storage
- `get_trait_storage(TypeId)` - Access storage by type ID as the family's trait object
- `get_trait_storage_mut(TypeId)` - Mutable version of the above
- `remove_trait_storage(TypeId)` - Remove a storage and return it
- `insert_erased(column)` - Insert a hand-built `ErasedVecStorage` (`ErasedVecFamily` only)

### VecStorage (VecFamily)

Storage for multiple values of a single type, backed by `Vec<T>`:

**Methods:**
- `push(value)` - Add a value, returns its index
- `get(idx)` - Get reference by index
- `get_mut(idx)` - Get mutable reference by index
- `iter()` - Iterate over all stored values
- `swap_remove(idx)` - Remove a value by swapping in the last one
- `take_boxed(idx)` - Remove a value and return it as a boxed trait object
- `get_dyn(idx)` - Get value as trait object reference
- `get_dyn_mut(idx)` - Get value as mutable trait object reference

### VecOptionStorage (VecOptionFamily)

Index-addressed slots that can be emptied; `len()` counts the values still present:

**Methods:**
- `push(value)` - Add a value, returns its index
- `get(idx)` - Get `Option<&T>` by index
- `get_mut(idx)` - Get `Option<&mut T>` by index
- `take(idx)` - Empty a slot and return its value
- `iter()` - Iterate over present values only
- `swap_remove(idx)` - Remove the slot entirely (does not update the count)
- `get_dyn(idx)` - Get value as optional trait object reference
- `get_dyn_mut(idx)` - Get value as mutable optional trait object reference
- `take_boxed(idx)` - Remove value and return it as a boxed trait object

### OptionStorage (OptionFamily)

Storage for a single value of a type:

**Methods:**
- `set(value)` - Set the stored value
- `get()` - Get reference to stored value
- `get_mut()` - Get mutable reference to stored value
- `take()` - Remove and return the stored value
- `is_some()` - Check if a value is stored
- `get_dyn()` - Get value as trait object reference
- `get_dyn_mut()` - Get value as mutable trait object reference
- `take_boxed()` - Remove value and return as boxed trait object

### ErasedVecStorage (ErasedVecFamily)

A concrete column with no element type in its signature:

**Methods:**
- `push::<T>(value)` / `get::<T>(idx)` / `get_mut::<T>(idx)` / `iter::<T>()` - Typed access against the column's `TypeId`
- `swap_remove::<T>(idx)` / `swap_remove_discard(idx)` - Remove a row by index
- `get_dyn(idx)` / `get_dyn_mut(idx)` / `take_boxed_dyn(idx)` - Trait object access through the stored function table
- `refresh_ops(ops)` - Replace the per-type function table without touching the rows
- `elem_size()` / `raw_ptr()` / `push_bytes(src, size)` - Byte-level row access
- `ErasedVecStorageInfo::of::<T>()` / `of_shared::<T>()` - Describe a column, keyed by exact `TypeId` or by layout

## Examples

See the [`examples/`](examples/) directory for complete examples:

- [`basic_usage.rs`](examples/basic_usage.rs) - Comprehensive demonstration of the storage families

Run examples with:
```bash
cargo run --example basic_usage
```

## Use Cases

- **Entity Component Systems**: Store components of different types implementing a common trait
- **Plugin Systems**: Manage plugins with a common interface but different implementations
- **Heterogeneous Collections**: Store related but differently-typed objects together
- **Type Registries**: Build type-based registries with trait object access

## How It Works

The library uses Rust's type system to maintain type safety while allowing heterogeneous storage:

1. **Type Registration**: Registering a type gives it its own storage column, keyed by `TypeId`
2. **Per-type layout**: Columns hold values in their natural form: a `Vec<T>`, an `Option<T>`, or a raw byte buffer inside `ErasedVecStorage`
3. **Trait object access**: Stored accessor functions upcast rows to `&dyn` / `&mut dyn` without naming the concrete type
4. **Type recovery**: `get_storage::<T>` returns the typed column through the map's `TypeId` lookup

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
