//! # trait_type_map
//!
//! A type-indexed map for storing values implementing a specific trait and fetching them by said trait or concrete type.
//! With support for single-value, multi-value, and sparse-optional storage per type.
//!
//! ## Features
//!
//! - Type-safe storage: Store different types implementing the same trait in a single map
//! - Flexible storage backends: Choose `OptionFamily` (single value), `VecFamily` (multiple values), or `VecOptionFamily` (sparse optional values) per type
//! - Trait object access: Access stored values as trait objects without knowing the concrete type
//! - Type-indexed retrieval: Retrieve values by their concrete type with zero runtime overhead
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
//! map.get_storage_mut::<Dog>().push(Dog);
//! map.get_storage_mut::<Cat>().push(Cat);
//!
//! // Access via trait object
//! let dog_storage = map.get_storage::<Dog>();
//! assert_eq!(dog_storage.get_dyn(0).speak(), "Woof!");
//! # }
//! ```

mod trait_type_map;
pub use trait_type_map::*;
