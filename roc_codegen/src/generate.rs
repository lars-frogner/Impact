//! Generation of code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

mod roc;

use crate::meta::{
    RocDependencies, RocMethod, RocType, RocTypeComposition, RocTypeFlags, RocTypeID,
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
    let type_map = gather_type_map(inventory::iter::<RocType>(), component_type_ids)?;

    let mut type_description_list = Vec::with_capacity(type_map.len());

    let print_list = |type_description_list: &mut Vec<String>, header: &str| {
        type_description_list.sort();

        println!("{header}");
        for description in &*type_description_list {
            println!("{description}");
        }
        if type_description_list.is_empty() {
            println!("<None>");
        }
        println!();

        type_description_list.clear();
    };

    if options
        .categories
        .contains(&ListedRocTypeCategory::Primitive)
    {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if matches!(ty.composition, RocTypeComposition::Primitive(_)) {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Primitives ******");
    }

    if options.categories.contains(&ListedRocTypeCategory::Pod) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_pod()
                && !ty.is_component()
                && !matches!(ty.composition, RocTypeComposition::Primitive(_))
            {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Plain Old Data ******");
    }

    if options
        .categories
        .contains(&ListedRocTypeCategory::Component)
    {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_component() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** ECS Components ******");
    }

    if options.categories.contains(&ListedRocTypeCategory::Inline) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if !ty.is_pod() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Inline ******");
    }

    Ok(())
}

pub fn list_constructors() -> Result<()> {
    let type_map = gather_type_map(inventory::iter::<RocType>(), &HashSet::new())?;

    let method_map = gather_method_map(inventory::iter::<RocMethod>());

    let mut type_list: Vec<_> = type_map
        .iter()
        .map(|(type_id, desc)| (type_id, desc.name))
        .collect();
    type_list.sort_by_key(|(_, name)| *name);

    for (type_id, type_name) in type_list {
        let Some(ty) = type_map.get(type_id) else {
            continue;
        };
        let Some(methods) = method_map.get(type_id) else {
            continue;
        };
        println!("****** {type_name} ******");
        for method in methods {
            let mut text = String::new();
            roc::write_method(&mut text, &type_map, ty, method)?;
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
    let type_iter = inventory::iter::<RocType>();
    let method_iter = inventory::iter::<RocMethod>();
    let dependencies_iter = inventory::iter::<RocDependencies>();

    let target_dir = target_dir.as_ref();

    let modules = generate_roc_modules(
        roc_options,
        type_iter,
        method_iter,
        dependencies_iter,
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
    type_iter: impl IntoIterator<Item = &'a RocType>,
    method_iter: impl IntoIterator<Item = &'a RocMethod>,
    dependencies_iter: impl IntoIterator<Item = &'a RocDependencies>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<Module>> {
    let type_map = gather_type_map(type_iter, component_type_ids)?;

    let method_map = gather_method_map(method_iter);

    let dependencies_map = gather_dependencies_map(dependencies_iter);

    type_map
        .values()
        .filter_map(|ty| {
            let methods = method_map
                .get(&ty.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            let dependencies = dependencies_map
                .get(&ty.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            match roc::generate_module(
                options,
                &type_map,
                ty,
                methods.as_ref(),
                dependencies.as_ref(),
            ) {
                Ok(Some(content)) => Some(Ok(Module {
                    name: ty.name,
                    content,
                })),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        })
        .collect::<Result<Vec<_>>>()
}

fn gather_type_map<'a>(
    type_iter: impl IntoIterator<Item = &'a RocType>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<HashMap<RocTypeID, RocType>> {
    let mut type_map = HashMap::new();

    for ty in type_iter {
        let mut ty = ty.clone();
        if component_type_ids.contains(&ty.id) {
            ty.flags |= RocTypeFlags::IS_COMPONENT;
        }
        if let Some(existing) = type_map.insert(ty.id, ty.clone()) {
            bail!(
                "Found two Roc types with the same ID:\n{:?}\n{:?}",
                existing,
                ty
            );
        }
    }
    Ok(type_map)
}

fn get_field_type<'a>(
    type_map: &'a HashMap<RocTypeID, RocType>,
    type_id: &RocTypeID,
    field: impl Display,
    parent_name: &impl Fn() -> String,
) -> Result<&'a RocType> {
    type_map.get(type_id).ok_or_else(|| {
        anyhow!(
            "Missing Roc type declaration for field {} with type ID {} in {}",
            field,
            type_id,
            parent_name(),
        )
    })
}

fn gather_method_map<'a>(
    method_iter: impl IntoIterator<Item = &'a RocMethod>,
) -> HashMap<RocTypeID, Vec<RocMethod>> {
    let mut method_map = HashMap::new();
    for method in method_iter {
        method_map
            .entry(method.for_type_id)
            .or_insert_with(Vec::new)
            .push(method.clone());
    }
    for methods in method_map.values_mut() {
        methods.sort_by_key(|desc| desc.sequence_number);
    }
    method_map
}

fn gather_dependencies_map<'a>(
    dependencies_iter: impl IntoIterator<Item = &'a RocDependencies>,
) -> HashMap<RocTypeID, Vec<RocDependencies>> {
    let mut dependencies_map = HashMap::new();
    for dependencies in dependencies_iter {
        dependencies_map
            .entry(dependencies.for_type_id)
            .or_insert_with(Vec::new)
            .push(dependencies.clone());
    }
    dependencies_map
}
