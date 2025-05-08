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
    pub categories: Vec<ListedRocTypeCategory>,
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

/// Roc code generation options.
#[derive(Clone, Debug)]
pub struct RocGenerateOptions {
    /// Name of the Roc package being generated into. Defaults to the directory
    /// name.
    pub package_name: Option<String>,
    /// Specific modules to generate. May be parent modules, in which case all
    /// their children are generated.
    pub only_modules: Vec<String>,
}

#[derive(Clone, Debug)]
struct Module {
    path_from_package_root: PathBuf,
    name: &'static str,
    content: String,
}

#[derive(Clone, Debug)]
struct ModuleFilter {
    allow_all: bool,
    allowed_module_paths: Vec<ModulePath>,
}

#[derive(Clone, Debug)]
struct ModulePath {
    parts: Vec<String>,
}

pub fn list_types(options: ListOptions, component_type_ids: &HashSet<RocTypeID>) -> Result<()> {
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

    let categories: HashSet<_> = if options.categories.is_empty() {
        vec![
            ListedRocTypeCategory::Primitive,
            ListedRocTypeCategory::Pod,
            ListedRocTypeCategory::Component,
            ListedRocTypeCategory::Inline,
        ]
    } else {
        options.categories
    }
    .into_iter()
    .collect();

    if categories.contains(&ListedRocTypeCategory::Primitive) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_primitive() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Primitives ******");
    }

    if categories.contains(&ListedRocTypeCategory::Pod) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_pod() && !ty.is_component() && !ty.is_primitive() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Plain Old Data ******");
    }

    if categories.contains(&ListedRocTypeCategory::Component) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_component() {
                Some(ty.description())
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** ECS Components ******");
    }

    if categories.contains(&ListedRocTypeCategory::Inline) {
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

/// Generates Roc source files in the package at the given path for
/// [`roc`](crate::roc)-annotated Rust types in linked crates.
pub fn generate_roc(
    target_dir: impl AsRef<Path>,
    options: GenerateOptions,
    roc_options: RocGenerateOptions,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<()> {
    let target_dir = target_dir.as_ref().canonicalize()?;

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    if !target_dir.is_dir() {
        bail!("{} is not a directory", target_dir.display());
    }

    let target_dir_name = if let Some(name) = target_dir.file_name() {
        name.to_string_lossy()
    } else {
        bail!("{} is not a valid package directory", target_dir.display());
    };

    let package_name = roc_options
        .package_name
        .as_deref()
        .map_or(target_dir_name, Cow::Borrowed);

    let type_iter = inventory::iter::<RegisteredType>();
    let associated_dependencies_iter = inventory::iter::<ir::AssociatedDependencies>();
    let associated_constant_iter = inventory::iter::<ir::AssociatedConstant>();
    let associated_function_iter = inventory::iter::<ir::AssociatedFunction>();

    let modules = generate_roc_modules(
        package_name,
        roc_options.only_modules,
        type_iter,
        associated_dependencies_iter,
        associated_constant_iter,
        associated_function_iter,
        component_type_ids,
    )?;

    for Module {
        path_from_package_root,
        name,
        content,
    } in modules
    {
        let module_dir = target_dir.join(path_from_package_root);

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

fn generate_roc_modules<'a, 'b>(
    package_name: Cow<'a, str>,
    only_modules: Vec<String>,
    type_iter: impl IntoIterator<Item = &'b RegisteredType>,
    associated_dependencies_iter: impl IntoIterator<Item = &'b ir::AssociatedDependencies>,
    associated_constant_iter: impl IntoIterator<Item = &'b ir::AssociatedConstant>,
    associated_function_iter: impl IntoIterator<Item = &'b ir::AssociatedFunction>,
    component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<Module>> {
    let type_map = gather_type_map(type_iter, component_type_ids)?;
    let associated_dependencies_map =
        gather_associated_dependencies_map(associated_dependencies_iter);
    let associated_constant_map = gather_associated_constant_map(associated_constant_iter);
    let associated_function_map = gather_associated_function_map(associated_function_iter);

    let module_filter = ModuleFilter::from_dot_separated(only_modules);

    type_map
        .values()
        .filter_map(|ty: &RegisteredType| {
            if !module_filter.permits(ty.parent_modules, ty.module_name) {
                return None;
            }

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
                &package_name,
                &type_map,
                ty,
                associated_dependencies.as_ref(),
                associated_constants.as_ref(),
                associated_functions.as_ref(),
            ) {
                Ok(Some(content)) => Some(Ok(Module {
                    path_from_package_root: ty
                        .parent_modules
                        .map_or_else(PathBuf::new, |parents| parents.split(".").collect()),
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

impl ModuleFilter {
    fn from_dot_separated(module_paths: Vec<String>) -> Self {
        if module_paths.is_empty() {
            Self {
                allow_all: true,
                allowed_module_paths: Vec::new(),
            }
        } else {
            Self {
                allow_all: false,
                allowed_module_paths: module_paths
                    .into_iter()
                    .map(|path| ModulePath::from_dot_separated(&path))
                    .collect(),
            }
        }
    }

    fn permits(&self, parent_modules: Option<&str>, module_name: &str) -> bool {
        if self.allow_all {
            return true;
        }

        let path = ModulePath::from_dot_separated_parents(parent_modules, module_name);

        self.allowed_module_paths
            .iter()
            .any(|allowed| allowed.is_parent_to(&path))
    }
}

impl ModulePath {
    fn empty() -> Self {
        Self { parts: Vec::new() }
    }

    fn from_dot_separated(module_path: &str) -> Self {
        Self {
            parts: module_path.split(".").map(ToString::to_string).collect(),
        }
    }

    fn from_dot_separated_parents(parent_modules: Option<&str>, module_name: &str) -> Self {
        let mut path = parent_modules.map_or_else(Self::empty, Self::from_dot_separated);
        path.parts.push(module_name.to_string());
        path
    }

    fn is_parent_to(&self, other: &Self) -> bool {
        if self.parts.len() > other.parts.len() {
            return false;
        }
        self.parts.iter().zip(&other.parts).all(|(a, b)| a == b)
    }
}
