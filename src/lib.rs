//! # trait_type_map
//!
//! A type-indexed map for storing values implementing a specific trait and fetching them by said trait or concrete type.
//! With support for both single-value and multi-value storage per type.
//!
//! ## Features
//!
//! - **Type-safe storage**: Store different types implementing the same trait in a single map
//! - **Flexible storage backends**: Choose between single-value (`SingleFamily`) or multi-value (`VecFamily`) storage per type
//! - **Trait object access**: Access stored values as trait objects without knowing the concrete type
//! - **Type-indexed retrieval**: Retrieve values by their concrete type with zero runtime overhead
//!
//! ## Quick Start
//!
//! ```rust
//! use trait_type_map::{impl_trait_accessible, TraitTypeMap, VecFamily};
//!
//! // Define a trait
//! trait Animal {
//!     fn speak(&self) -> &str;
//! }
//!
//! // Implement the trait for your types
//! struct Dog;
//! impl Animal for Dog {
//!     fn speak(&self) -> &str { "Woof!" }
//! }
//!
//! struct Cat;
//! impl Animal for Cat {
//!     fn speak(&self) -> &str { "Meow!" }
//! }
//!
//! // Register types as accessible via the trait
//! impl_trait_accessible!(dyn Animal; Dog, Cat);
//!
//! # fn main() {
//! // Create a map with vector storage (multiple instances per type)
//! let mut map: TraitTypeMap<dyn Animal, VecFamily> = TraitTypeMap::new();
//!
//! // Register types
//! map.register_type_storage::<Dog>();
//! map.register_type_storage::<Cat>();
//!
//! // Store values
//! let dog_idx = map.get_storage_mut::<Dog>().push(Dog);
//! let cat_idx = map.get_storage_mut::<Cat>().push(Cat);
//!
//! // Access via trait object
//! let dog_storage = map.get_storage::<Dog>();
//! if let Some(animal) = dog_storage.get_dyn(dog_idx) {
//!     assert_eq!(animal.speak(), "Woof!");
//! }
//! # }
//! ```
//!
//! ## Storage Families
//!
//! ### VecFamily - Multiple Values Per Type
//!
//! Use `VecFamily` when you need to store multiple instances of each type.
//!
//! ### SingleFamily - One Value Per Type
//!
//! Use `SingleFamily` when you only need to store one instance of each type.

mod trait_type_map;
pub use trait_type_map::*;
