//! Generation of code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

mod roc;

use crate::meta::{
    RocConstructorDescriptor, RocDependencies, RocTypeComposition, RocTypeDescriptor, RocTypeFlags,
    RocTypeID,
};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// Options for listing types.
#[derive(Clone, Debug)]
pub struct ListOptions {
    pub categories: HashSet<ListedRocTypeCategory>,
}

/// The categories of `roc`-annotated types that can be listed.
#[cfg_attr(feature = "cli", derive(::clap::ValueEnum))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListedRocTypeCategory {
    Primitive,
    Pod,
    Component,
    Inline,
}

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

pub fn list_types(options: &ListOptions, component_type_ids: &HashSet<RocTypeID>) -> Result<()> {
    let descriptors =
        gather_type_descriptors(inventory::iter::<RocTypeDescriptor>(), component_type_ids)?;

    let mut descriptor_list = Vec::with_capacity(descriptors.len());

    let print_list = |descriptor_list: &mut Vec<String>, header: &str| {
        descriptor_list.sort();

        println!("{header}");
        for name in &*descriptor_list {
            println!("{name}");
        }
        if descriptor_list.is_empty() {
            println!("<None>");
        }
        println!();

        descriptor_list.clear();
    };

    if options
        .categories
        .contains(&ListedRocTypeCategory::Primitive)
    {
        descriptor_list.extend(descriptors.values().filter_map(|descriptor| {
            if matches!(descriptor.composition, RocTypeComposition::Primitive(_)) {
                Some(descriptor.description())
            } else {
                None
            }
        }));
        print_list(&mut descriptor_list, "****** Primitives ******");
    }

    if options.categories.contains(&ListedRocTypeCategory::Pod) {
        descriptor_list.extend(descriptors.values().filter_map(|descriptor| {
            if descriptor.is_pod()
                && !descriptor.is_component()
                && !matches!(descriptor.composition, RocTypeComposition::Primitive(_))
            {
                Some(descriptor.description())
            } else {
                None
            }
        }));
        print_list(&mut descriptor_list, "****** Plain Old Data ******");
    }

    if options
        .categories
        .contains(&ListedRocTypeCategory::Component)
    {
        descriptor_list.extend(descriptors.values().filter_map(|descriptor| {
            if descriptor.is_component() {
                Some(descriptor.description())
            } else {
                None
            }
        }));
        print_list(&mut descriptor_list, "****** ECS Components ******");
    }

    if options.categories.contains(&ListedRocTypeCategory::Inline) {
        descriptor_list.extend(descriptors.values().filter_map(|descriptor| {
            if !descriptor.is_pod() {
                Some(descriptor.description())
            } else {
                None
            }
        }));
        print_list(&mut descriptor_list, "****** Inline ******");
    }

    Ok(())
}

pub fn list_constructors() -> Result<()> {
    let type_descriptors =
        gather_type_descriptors(inventory::iter::<RocTypeDescriptor>(), &HashSet::new())?;

    let constructor_descriptors =
        gather_constructor_descriptors(inventory::iter::<RocConstructorDescriptor>());

    let mut type_descriptor_list: Vec<_> = type_descriptors
        .iter()
        .map(|(type_id, desc)| (type_id, desc.type_name))
        .collect();
    type_descriptor_list.sort_by_key(|(_, name)| *name);

    for (type_id, type_name) in type_descriptor_list {
        let Some(type_descriptor) = type_descriptors.get(type_id) else {
            continue;
        };
        let Some(descriptors) = constructor_descriptors.get(type_id) else {
            continue;
        };
        println!("****** {type_name} ******");
        for desc in descriptors {
            let mut text = String::new();
            roc::write_constructor(&mut text, &type_descriptors, type_descriptor, desc)?;
            println!("{text}");
        }
    }

    Ok(())
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
    let type_descriptors = inventory::iter::<RocTypeDescriptor>();
    let constructor_descriptors = inventory::iter::<RocConstructorDescriptor>();
    let explicit_dependencies = inventory::iter::<RocDependencies>();

    let target_dir = target_dir.as_ref();

    let modules = generate_roc_modules(
        roc_options,
        type_descriptors,
        constructor_descriptors,
        explicit_dependencies,
        component_type_ids,
    )?;

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
    type_descriptors: impl IntoIterator<Item = &'a RocTypeDescriptor>,
    constructor_descriptors: impl IntoIterator<Item = &'a RocConstructorDescriptor>,
    explicit_dependencies: impl IntoIterator<Item = &'a RocDependencies>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<Module>> {
    let type_descriptors = gather_type_descriptors(type_descriptors, component_type_ids)?;

    let constructor_descriptors = gather_constructor_descriptors(constructor_descriptors);

    let explicit_dependencies = gather_explicit_dependencies(explicit_dependencies);

    type_descriptors
        .values()
        .filter_map(|type_descriptor| {
            let constructor_descriptors = constructor_descriptors
                .get(&type_descriptor.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            let explicit_dependencies = explicit_dependencies
                .get(&type_descriptor.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            match roc::generate_module(
                options,
                &type_descriptors,
                type_descriptor,
                constructor_descriptors.as_ref(),
                explicit_dependencies.as_ref(),
            ) {
                Ok(Some(content)) => Some(Ok(Module {
                    name: type_descriptor.type_name,
                    content,
                })),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        })
        .collect::<Result<Vec<_>>>()
}

fn gather_type_descriptors<'a>(
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

fn field_type_descriptor<'a>(
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

fn gather_constructor_descriptors<'a>(
    descriptor_iter: impl IntoIterator<Item = &'a RocConstructorDescriptor>,
) -> HashMap<RocTypeID, Vec<RocConstructorDescriptor>> {
    let mut descriptors = HashMap::new();
    for descriptor in descriptor_iter {
        descriptors
            .entry(descriptor.for_type_id)
            .or_insert_with(Vec::new)
            .push(descriptor.clone());
    }
    for constructor_descriptors in descriptors.values_mut() {
        constructor_descriptors.sort_by_key(|desc| desc.sequence_number);
    }
    descriptors
}

fn gather_explicit_dependencies<'a>(
    dependencies_iter: impl IntoIterator<Item = &'a RocDependencies>,
) -> HashMap<RocTypeID, Vec<RocDependencies>> {
    let mut dependencies = HashMap::new();
    for deps in dependencies_iter {
        dependencies
            .entry(deps.for_type_id)
            .or_insert_with(Vec::new)
            .push(deps.clone());
    }
    dependencies
}
