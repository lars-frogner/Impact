//! Generation of code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

mod roc;

use crate::{RegisteredType, RegisteredTypeFlags, RocTypeID, ir};
use anyhow::{Context, Result, anyhow, bail};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
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

#[derive(Clone, Debug)]
struct Module {
    path_from_package_root: PathBuf,
    name: &'static str,
    content: String,
}

pub fn list_types(options: &ListOptions, component_type_ids: &HashSet<RocTypeID>) -> Result<()> {
    let type_map = gather_type_map(inventory::iter::<RegisteredType>(), component_type_ids)?;

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
            if ty.is_primitive() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Primitives ******");
    }

    if options.categories.contains(&ListedRocTypeCategory::Pod) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_pod() && !ty.is_component() && !ty.is_primitive() {
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

pub fn list_associated_items(for_types: Vec<String>) -> Result<()> {
    let type_map = gather_type_map(inventory::iter::<RegisteredType>(), &HashSet::new())?;
    let associated_constant_map =
        gather_associated_constant_map(inventory::iter::<ir::AssociatedConstant>());
    let associated_function_map =
        gather_associated_function_map(inventory::iter::<ir::AssociatedFunction>());

    let mut type_list: Vec<_> = type_map
        .iter()
        .map(|(type_id, ty)| (type_id, ty.ty.name))
        .collect();
    type_list.sort_by_key(|(_, name)| *name);

    if !for_types.is_empty() {
        let for_types: HashSet<_> = for_types.into_iter().collect();
        type_list.retain(|(_, name)| for_types.contains(*name));
    }

    for (type_id, type_name) in type_list {
        let Some(ty) = type_map.get(type_id) else {
            continue;
        };
        if !associated_constant_map.contains_key(type_id)
            && !associated_function_map.contains_key(type_id)
        {
            continue;
        }
        println!("****** {type_name} ******");
        if let Some(associated_constants) = associated_constant_map.get(type_id) {
            for associated_constant in associated_constants {
                let mut text = String::new();
                roc::write_associated_constant(&mut text, &type_map, ty, associated_constant)?;
                println!("{text}");
            }
        };
        if let Some(associated_functions) = associated_function_map.get(type_id) {
            for associated_function in associated_functions {
                let mut text = String::new();
                roc::write_associated_function(&mut text, &type_map, ty, associated_function)?;
                println!("{text}");
            }
        };
    }

    Ok(())
}

/// Generates Roc source files in the package at the given path for all
/// [`roc`](crate::roc)-annotated Rust types in linked crates.
pub fn generate_roc(
    package_root: impl AsRef<Path>,
    options: &GenerateOptions,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<()> {
    let type_iter = inventory::iter::<RegisteredType>();
    let associated_dependencies_iter = inventory::iter::<ir::AssociatedDependencies>();
    let associated_constant_iter = inventory::iter::<ir::AssociatedConstant>();
    let associated_function_iter = inventory::iter::<ir::AssociatedFunction>();

    let package_root = package_root.as_ref();

    let modules = generate_roc_modules(
        type_iter,
        associated_dependencies_iter,
        associated_constant_iter,
        associated_function_iter,
        component_type_ids,
    )?;

    if !package_root.exists() {
        fs::create_dir_all(package_root)?;
    }

    for Module {
        path_from_package_root,
        name,
        content,
    } in modules
    {
        let module_dir = package_root.join(path_from_package_root);

        fs::create_dir_all(&module_dir)?;

        let module_path = module_dir.join(format!("{name}.roc"));

        let existed = module_path.exists();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .create_new(!options.overwrite)
            .open(&module_path)
            .with_context(|| {
                format!(
                    "Could not create file {} for writing",
                    module_path.display()
                )
            })?;

        write!(&mut file, "{}", content)?;

        if options.verbose {
            println!(
                "Generated {}{}",
                module_path.display(),
                if existed { " (replaced existing)" } else { "" }
            );
        }
    }

    Ok(())
}

fn generate_roc_modules<'a>(
    type_iter: impl IntoIterator<Item = &'a RegisteredType>,
    associated_dependencies_iter: impl IntoIterator<Item = &'a ir::AssociatedDependencies>,
    associated_constant_iter: impl IntoIterator<Item = &'a ir::AssociatedConstant>,
    associated_function_iter: impl IntoIterator<Item = &'a ir::AssociatedFunction>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<Module>> {
    let type_map = gather_type_map(type_iter, component_type_ids)?;
    let associated_dependencies_map =
        gather_associated_dependencies_map(associated_dependencies_iter);
    let associated_constant_map = gather_associated_constant_map(associated_constant_iter);
    let associated_function_map = gather_associated_function_map(associated_function_iter);

    type_map
        .values()
        .filter_map(|ty| {
            let associated_dependencies = associated_dependencies_map
                .get(&ty.ty.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            let associated_constants = associated_constant_map
                .get(&ty.ty.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            let associated_functions = associated_function_map
                .get(&ty.ty.id)
                .map_or_else(Cow::default, Cow::Borrowed);

            match roc::generate_module(
                &type_map,
                ty,
                associated_dependencies.as_ref(),
                associated_constants.as_ref(),
                associated_functions.as_ref(),
            ) {
                Ok(Some(content)) => Some(Ok(Module {
                    path_from_package_root: ty.module_prefix.split(".").collect(),
                    name: ty.ty.name,
                    content,
                })),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        })
        .collect::<Result<Vec<_>>>()
}

fn gather_type_map<'a>(
    type_iter: impl IntoIterator<Item = &'a RegisteredType>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<HashMap<RocTypeID, RegisteredType>> {
    let mut type_map = HashMap::new();

    for ty in type_iter {
        let mut ty = ty.clone();
        if component_type_ids.contains(&ty.ty.id) {
            ty.flags |= RegisteredTypeFlags::IS_COMPONENT;
        }
        if let Some(existing) = type_map.insert(ty.ty.id, ty.clone()) {
            bail!(
                "Found two Roc types with the same ID:\n{:?}\n{:?}",
                existing,
                ty
            );
        }
    }
    Ok(type_map)
}

fn gather_associated_dependencies_map<'a>(
    associated_dependencies_iter: impl IntoIterator<Item = &'a ir::AssociatedDependencies>,
) -> HashMap<RocTypeID, Vec<ir::AssociatedDependencies>> {
    let mut associated_dependencies_map = HashMap::new();
    for associated_dependencies in associated_dependencies_iter {
        associated_dependencies_map
            .entry(associated_dependencies.for_type_id)
            .or_insert_with(Vec::new)
            .push(associated_dependencies.clone());
    }
    associated_dependencies_map
}

fn gather_associated_constant_map<'a>(
    associated_constant_iter: impl IntoIterator<Item = &'a ir::AssociatedConstant>,
) -> HashMap<RocTypeID, Vec<ir::AssociatedConstant>> {
    let mut associated_constant_map = HashMap::new();
    for associated_constant in associated_constant_iter {
        associated_constant_map
            .entry(associated_constant.for_type_id)
            .or_insert_with(Vec::new)
            .push(associated_constant.clone());
    }
    for associated_constants in associated_constant_map.values_mut() {
        associated_constants.sort_by_key(|desc| desc.sequence_number);
    }
    associated_constant_map
}

fn gather_associated_function_map<'a>(
    associated_function_iter: impl IntoIterator<Item = &'a ir::AssociatedFunction>,
) -> HashMap<RocTypeID, Vec<ir::AssociatedFunction>> {
    let mut associated_function_map = HashMap::new();
    for associated_function in associated_function_iter {
        associated_function_map
            .entry(associated_function.for_type_id)
            .or_insert_with(Vec::new)
            .push(associated_function.clone());
    }
    for associated_functions in associated_function_map.values_mut() {
        associated_functions.sort_by_key(|desc| desc.sequence_number);
    }
    associated_function_map
}

fn get_field_type<'a>(
    type_map: &'a HashMap<RocTypeID, RegisteredType>,
    type_id: &RocTypeID,
    field: impl Display,
    parent_name: &impl Fn() -> String,
) -> Result<&'a RegisteredType> {
    type_map.get(type_id).ok_or_else(|| {
        anyhow!(
            "Missing Roc type declaration for field {} with type ID {} in {}",
            field,
            type_id,
            parent_name(),
        )
    })
}
