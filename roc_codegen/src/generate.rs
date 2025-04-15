//! Generation of code for working with types deriving the
//! [`Roc`](crate::meta::Roc) or [`RocPod`](crate::meta::RocPod) trait.

mod roc;

use crate::meta::{RocTypeDescriptor, RocTypeFlags, RocTypeID};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// General code generation options.
#[derive(Clone, Debug)]
pub struct GenerateOptions {
    /// Whether to print progress and status messages.
    pub verbose: bool,
    /// Whether to automatically overwrite existing files.
    pub overwrite: bool,
}

/// Roc-specific code generation options.
#[derive(Clone, Debug)]
pub struct RocGenerateOptions {
    /// String to prepend to imports from generated modules.
    pub import_prefix: String,
    /// Name to use for the platform package in imports.
    pub platform_package_name: String,
    /// Name to use for the `packages/core` package in imports.
    pub core_package_name: String,
}

#[derive(Clone, Debug)]
struct Module {
    name: &'static str,
    content: String,
}

/// Generates Roc source files in the target directory for all Rust types in
/// linked crates deriving the [`Roc`](crate::meta::Roc) or
/// [`RocPod`](crate::meta::RocPod) trait.
pub fn generate_roc(
    target_dir: impl AsRef<Path>,
    options: &GenerateOptions,
    roc_options: &RocGenerateOptions,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<()> {
    let descriptors = inventory::iter::<RocTypeDescriptor>();

    let target_dir = target_dir.as_ref();

    let modules = generate_roc_modules(roc_options, descriptors, component_type_ids)?;

    if !target_dir.exists() {
        fs::create_dir_all(target_dir)?;
    }

    for Module { name, content } in modules {
        let file_path = target_dir.join(format!("{name}.roc"));

        let existed = file_path.exists();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!options.overwrite)
            .open(&file_path)
            .with_context(|| {
                format!("Could not create file {} for writing", file_path.display())
            })?;

        write!(&mut file, "{}", content)?;

        if options.verbose {
            println!(
                "Generated {}{}",
                file_path.display(),
                if existed { " (replaced existing)" } else { "" }
            );
        }
    }

    Ok(())
}

fn generate_roc_modules<'a>(
    options: &RocGenerateOptions,
    descriptors: impl IntoIterator<Item = &'a RocTypeDescriptor>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<Module>> {
    let descriptors = gather_descriptors(descriptors, component_type_ids)?;

    descriptors
        .values()
        .filter_map(
            |descriptor| match roc::generate_module(options, &descriptors, descriptor) {
                Ok(Some(content)) => Some(Ok(Module {
                    name: descriptor.roc_name,
                    content,
                })),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            },
        )
        .collect::<Result<Vec<_>>>()
}

fn gather_descriptors<'a>(
    descriptor_iter: impl IntoIterator<Item = &'a RocTypeDescriptor>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<HashMap<RocTypeID, RocTypeDescriptor>> {
    let mut descriptors = HashMap::new();

    for descriptor in descriptor_iter {
        let mut descriptor = descriptor.clone();
        if component_type_ids.contains(&descriptor.id) {
            descriptor.flags |= RocTypeFlags::IS_COMPONENT;
        }
        if let Some(existing) = descriptors.insert(descriptor.id, descriptor.clone()) {
            bail!(
                "Found two Roc types with the same ID:\n{:?}\n{:?}",
                existing,
                descriptor
            );
        }
    }
    Ok(descriptors)
}

fn field_descriptor<'a>(
    descriptors: &'a HashMap<RocTypeID, RocTypeDescriptor>,
    type_id: &RocTypeID,
    field: impl Display,
    parent_name: &impl Fn() -> String,
) -> Result<&'a RocTypeDescriptor> {
    descriptors.get(type_id).ok_or_else(|| {
        anyhow!(
            "Missing Roc type declaration for field {} with type ID {} in {}",
            field,
            type_id,
            parent_name(),
        )
    })
}
