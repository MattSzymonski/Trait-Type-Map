# Trait Type Map

A type-indexed map for storing values of multiple types implementing a specific trait and fetching them by said trait or concrete type.
With support for both single-value and multi-value storage per type.

## Features

- **Type-safe storage**: Store different types implementing the same trait in a single map
- **Flexible storage backends**: Choose between single-value (`SingleFamily`) or multi-value (`VecFamily`) storage per type
- **Trait object access**: Access stored values as trait objects without knowing the concrete type
- **Type-indexed retrieval**: Retrieve values by their concrete type with zero runtime overhead
- **Zero-cost abstractions**: No performance penalty for type safety

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
    let cat_idx = map.get_storage_mut::<Cat>().push(Cat);
    
    // Access via trait object
    let dog_storage = map.get_storage::<Dog>();
    if let Some(animal) = dog_storage.get_dyn(dog_idx) {
        println!("{}", animal.speak()); // Prints: Woof!
    }
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

### SingleFamily - One Value Per Type

Stores at most one instance of each type:

```rust
use trait_type_map::{TraitTypeMap, SingleFamily};

let mut map: TraitTypeMap<dyn Animal, SingleFamily> = TraitTypeMap::new();
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
- `F`: The storage family (`VecFamily` or `SingleFamily`)

**Methods:**
- `new()` - Create a new empty map
- `register_type_storage::<T>()` - Register a type for storage
- `get_storage::<T>()` - Get immutable access to type's storage
- `get_storage_mut::<T>()` - Get mutable access to type's storage
- `get_trait_storage(TypeId)` - Access storage by type ID as trait object

### VecOptionStorage (VecFamily)

Storage for multiple values of a single type:

**Methods:**
- `push(value)` - Add a value, returns index
- `get(idx)` - Get reference by index
- `get_mut(idx)` - Get mutable reference by index
- `take(idx)` - Remove and return value by index
- `iter()` - Iterate over all stored values
- `get_dyn(idx)` - Get value as trait object reference
- `get_dyn_mut(idx)` - Get value as mutable trait object reference
- `take_boxed(idx)` - Remove value and return as boxed trait object

### OptionStorage (SingleFamily)

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

## Examples

See the [`examples/`](examples/) directory for complete examples:

- [`basic_usage.rs`](examples/basic_usage.rs) - Comprehensive demonstration of both storage families

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

1. **Type Registration**: Each concrete type implementing the trait is registered with the map
2. **Type Erasure**: Values are stored as trait objects internally
3. **Type Recovery**: The concrete type can be recovered using `TypeId` for type-safe access
4. **Accessor Functions**: User-provided functions handle safe upcasting from concrete types to trait objects

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
