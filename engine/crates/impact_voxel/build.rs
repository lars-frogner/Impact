use std::fs;
use std::path::Path;

fn main() {
    let shader_dir = Path::new("shaders");

    // Rebuild if any shader file changes
    watch_directory(shader_dir);
}

fn watch_directory(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_dir() {
            // Recursively watch subdirectories
            watch_directory(&path);
        } else if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
