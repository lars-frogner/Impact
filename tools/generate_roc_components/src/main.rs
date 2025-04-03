use impact::{self, application::components, component::ComponentRegistry};

fn main() {
    let mut component_registry = ComponentRegistry::new();
    if let Err(err) = components::register_all_components(&mut component_registry) {
        eprintln!("Failed to register components: {}", err);
        std::process::exit(1);
    }
}
