// Run with: cargo run --example basic_usage

use trait_type_map::{impl_trait_accessible, SingleFamily, TraitTypeMap, VecFamily};

// Define a trait that our types will implement
trait Animal {
    fn speak(&self) -> &str;
    fn name(&self) -> &str;
}

// Define some concrete types
struct Dog {
    name: String,
}

impl Animal for Dog {
    fn speak(&self) -> &str {
        "Woof!"
    }
    fn name(&self) -> &str {
        &self.name
    }
}

struct Cat {
    name: String,
}

impl Animal for Cat {
    fn speak(&self) -> &str {
        "Meow!"
    }
    fn name(&self) -> &str {
        &self.name
    }
}

struct Bird {
    name: String,
}

impl Animal for Bird {
    fn speak(&self) -> &str {
        "Tweet!"
    }
    fn name(&self) -> &str {
        &self.name
    }
}

// Register types as accessible via the Animal trait
impl_trait_accessible!(dyn Animal; Dog, Cat, Bird);

fn main() {
    println!("=== Vector Storage Example ===\n");
    vector_storage_example();

    println!("\n=== Single Storage Example ===\n");
    single_storage_example();
}

fn vector_storage_example() {
    // Create a map using VecFamily (stores multiple instances per type)
    let mut map: TraitTypeMap<dyn Animal, VecFamily> = TraitTypeMap::new();

    // Register the types we want to store
    map.register_type_storage::<Dog>();
    map.register_type_storage::<Cat>();
    map.register_type_storage::<Bird>();

    // Add multiple instances of each type
    let dog_storage = map.get_storage_mut::<Dog>();
    let dog1_idx = dog_storage.push(Dog {
        name: "Rex".to_string(),
    });
    let dog2_idx = dog_storage.push(Dog {
        name: "Buddy".to_string(),
    });

    let cat_storage = map.get_storage_mut::<Cat>();
    cat_storage.push(Cat {
        name: "Whiskers".to_string(),
    });
    cat_storage.push(Cat {
        name: "Mittens".to_string(),
    });

    let bird_storage = map.get_storage_mut::<Bird>();
    bird_storage.push(Bird {
        name: "Tweety".to_string(),
    });

    // Access via concrete types
    println!("Accessing via concrete types:");
    let dog_storage = map.get_storage::<Dog>();
    if let Some(dog) = dog_storage.get(dog1_idx) {
        println!(
            "  Dog at index {}: {} says {}",
            dog1_idx,
            dog.name(),
            dog.speak()
        );
    }
    if let Some(dog) = dog_storage.get(dog2_idx) {
        println!(
            "  Dog at index {}: {} says {}",
            dog2_idx,
            dog.name(),
            dog.speak()
        );
    }

    // Access via trait object
    println!("\nAccessing via trait object:");
    let dog_storage = map.get_storage::<Dog>();
    if let Some(animal) = dog_storage.get_dyn(dog1_idx) {
        println!("  Animal: {} says {}", animal.name(), animal.speak());
    }

    // Iterate over all dogs
    println!("\nIterating over all dogs:");
    let dog_storage = map.get_storage::<Dog>();
    for (i, dog) in dog_storage.iter().enumerate() {
        println!("  Dog {}: {} says {}", i, dog.name(), dog.speak());
    }

    // Iterate over all cats
    println!("\nIterating over all cats:");
    let cat_storage = map.get_storage::<Cat>();
    for (i, cat) in cat_storage.iter().enumerate() {
        println!("  Cat {}: {} says {}", i, cat.name(), cat.speak());
    }

    // Take ownership (removes from storage)
    println!("\nTaking ownership of a dog:");
    let dog_storage_mut = map.get_storage_mut::<Dog>();
    if let Some(boxed_animal) = dog_storage_mut.take_boxed(dog1_idx) {
        println!(
            "  Took: {} says {}",
            boxed_animal.name(),
            boxed_animal.speak()
        );
    }

    // Verify it's removed
    println!("\nAfter removal:");
    let dog_storage = map.get_storage::<Dog>();
    println!(
        "  Dog at index {} exists: {}",
        dog1_idx,
        dog_storage.get(dog1_idx).is_some()
    );
    println!(
        "  Dog at index {} exists: {}",
        dog2_idx,
        dog_storage.get(dog2_idx).is_some()
    );
}

fn single_storage_example() {
    // Create a map using SingleFamily (stores at most one instance per type)
    let mut map: TraitTypeMap<dyn Animal, SingleFamily> = TraitTypeMap::new();

    // Register the types we want to store
    map.register_type_storage::<Dog>();
    map.register_type_storage::<Cat>();
    map.register_type_storage::<Bird>();

    // Add one instance of each type
    map.get_storage_mut::<Dog>().set(Dog {
        name: "Max".to_string(),
    });
    map.get_storage_mut::<Cat>().set(Cat {
        name: "Felix".to_string(),
    });
    map.get_storage_mut::<Bird>().set(Bird {
        name: "Polly".to_string(),
    });

    // Access via concrete types
    println!("Accessing via concrete types:");
    if let Some(dog) = map.get_storage::<Dog>().get() {
        println!("  Dog: {} says {}", dog.name(), dog.speak());
    }
    if let Some(cat) = map.get_storage::<Cat>().get() {
        println!("  Cat: {} says {}", cat.name(), cat.speak());
    }

    // Access via trait object
    println!("\nAccessing via trait object:");
    if let Some(animal) = map.get_storage::<Dog>().get_dyn() {
        println!("  Animal: {} says {}", animal.name(), animal.speak());
    }

    // Mutable access
    println!("\nMutable access:");
    if let Some(dog) = map.get_storage_mut::<Dog>().get_mut() {
        dog.name = "Maxwell".to_string();
        println!("  Modified dog name to: {}", dog.name());
    }

    // Check if storage contains a value
    println!("\nChecking storage:");
    println!("  Has Dog: {}", map.get_storage::<Dog>().is_some());
    println!("  Has Cat: {}", map.get_storage::<Cat>().is_some());
    println!("  Has Bird: {}", map.get_storage::<Bird>().is_some());

    // Take ownership (removes from storage)
    println!("\nTaking ownership of the cat:");
    if let Some(boxed_animal) = map.get_storage_mut::<Cat>().take_boxed() {
        println!(
            "  Took: {} says {}",
            boxed_animal.name(),
            boxed_animal.speak()
        );
    }

    // Verify it's removed
    println!("\nAfter removal:");
    println!("  Has Cat: {}", map.get_storage::<Cat>().is_some());
}
