//!

mod roc;
mod rust;

use crate::meta::{RocTypeDescriptor, RocTypeID};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Clone, Debug)]
pub struct GenerateOptions {
    pub verbose: bool,
    pub overwrite: bool,
}

#[derive(Clone, Debug)]
pub struct RocGenerateOptions {
    pub module_prefix: String,
    pub core_prefix: String,
    pub include_roundtrip_test: bool,
}

#[derive(Clone, Debug)]
struct Module {
    name: &'static str,
    content: String,
}

pub fn generate_roc(
    target_dir: impl AsRef<Path>,
    options: &GenerateOptions,
    roc_options: &RocGenerateOptions,
) -> Result<()> {
    let descriptors = inventory::iter::<RocTypeDescriptor>();

    let target_dir = target_dir.as_ref();

    let modules = generate_roc_modules(roc_options, descriptors)?;

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
) -> Result<Vec<Module>> {
    let descriptors = gather_descriptors(descriptors)?;

    let mut modules = descriptors
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
        .collect::<Result<Vec<_>>>()?;

    if options.include_roundtrip_test {
        modules.push(Module {
            name: "RoundtripTest",
            content: roc::generate_roundtrip_test(options, &descriptors)?,
        });
    }

    Ok(modules)
}

fn gather_descriptors<'a>(
    descriptor_iter: impl IntoIterator<Item = &'a RocTypeDescriptor>,
) -> Result<HashMap<RocTypeID, RocTypeDescriptor>> {
    let mut descriptors = HashMap::new();

    for descriptor in descriptor_iter {
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
