//! Generation of code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

mod roc;

use crate::{HashMap, HashSet, RegisteredType, RegisteredTypeFlags, RocTypeID, ir};
use anyhow::{Context, Result, anyhow, bail};
use chrono::{SecondsFormat, Utc};
use roc::OptionalExports;
use std::{
    borrow::Cow,
    fmt::{self, Display},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

/// Options for listing types.
#[derive(Clone, Debug)]
pub struct ListOptions {
    pub categories: Vec<ListedRocTypeCategory>,
    pub show_type_ids: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum GeneratedTypeCategory {
    Pod,
    Component,
    Inline,
}

/// General code generation options.
#[derive(Clone, Debug)]
pub struct GenerateOptions {
    /// Whether to print progress and status messages.
    pub verbose: bool,
}

/// Options for cleaning generated code.
#[derive(Clone, Debug)]
pub struct CleanOptions {
    /// Whether to print progress and status messages.
    pub verbose: bool,
    /// Whether to recursively clean subdirectories.
    pub recursive: bool,
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
    header: ModuleHeader,
    code: String,
}

#[derive(Clone, Debug)]
struct ModuleHeader {
    code_hash: String,
    timestamp: String,
    rust_type_path: Option<String>,
    type_category: GeneratedTypeCategory,
    commit_sha: Option<String>,
    commit_dirty: bool,
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

pub fn list_types(
    options: ListOptions,
    component_type_ids: &HashSet<RocTypeID>,
    setup_component_type_ids: &HashSet<RocTypeID>,
) -> Result<()> {
    let type_map = gather_type_map(
        inventory::iter::<RegisteredType>(),
        component_type_ids,
        setup_component_type_ids,
    )?;

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
                Some(ty.description(options.show_type_ids))
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Primitives ******");
    }

    if categories.contains(&ListedRocTypeCategory::Pod) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_pod() && !ty.is_component() && !ty.is_primitive() {
                Some(ty.description(options.show_type_ids))
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Plain Old Data ******");
    }

    if categories.contains(&ListedRocTypeCategory::Component) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if ty.is_component() {
                Some(ty.description(options.show_type_ids))
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** ECS Components ******");
    }

    if categories.contains(&ListedRocTypeCategory::Inline) {
        type_description_list.extend(type_map.values().filter_map(|ty| {
            if !ty.is_pod() {
                Some(ty.description(options.show_type_ids))
            } else {
                None
            }
        }));
        print_list(&mut type_description_list, "****** Inline ******");
    }

    Ok(())
}

pub fn list_associated_items(for_types: Vec<String>) -> Result<()> {
    let type_map = gather_type_map(
        inventory::iter::<RegisteredType>(),
        &HashSet::default(),
        &HashSet::default(),
    )?;
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

    // (We don't actually care about exports here)
    let mut exports = OptionalExports::new();

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
                roc::write_associated_constant(
                    &mut text,
                    &mut exports,
                    &type_map,
                    ty,
                    associated_constant,
                )?;
                println!("{text}");
            }
        };
        if let Some(associated_functions) = associated_function_map.get(type_id) {
            for associated_function in associated_functions {
                let mut text = String::new();
                roc::write_associated_function(
                    &mut text,
                    &mut exports,
                    &type_map,
                    ty,
                    associated_function,
                )?;
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
    setup_component_type_ids: &HashSet<RocTypeID>,
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
        &package_name,
        roc_options.only_modules,
        type_iter,
        associated_dependencies_iter,
        associated_constant_iter,
        associated_function_iter,
        component_type_ids,
        setup_component_type_ids,
    )?;

    let mut unchanged = Vec::new();
    let mut generated = Vec::new();
    let mut warnings = Vec::new();

    for (module, path_from_package_root) in modules {
        module.save(
            &target_dir,
            &path_from_package_root,
            &mut unchanged,
            &mut generated,
            &mut warnings,
            options.verbose,
        )?;
    }

    if !unchanged.is_empty() && (!generated.is_empty() || !warnings.is_empty()) {
        println!("Unchanged:");
    }
    for msg in &unchanged {
        println!("{msg}");
    }
    if !generated.is_empty() && (!unchanged.is_empty() || !warnings.is_empty()) {
        if !unchanged.is_empty() {
            println!();
        }
        println!("Generated:");
    }
    for msg in &generated {
        println!("{msg}");
    }
    if !warnings.is_empty() && (!unchanged.is_empty() || !generated.is_empty()) {
        if !generated.is_empty() || !unchanged.is_empty() {
            println!();
        }
        println!("Warnings:");
    }
    for msg in &warnings {
        eprintln!("{msg}");
    }

    Ok(())
}

pub fn clean_generated_roc(target_dir: impl AsRef<Path>, options: CleanOptions) -> Result<()> {
    let target_dir = target_dir.as_ref();

    if !target_dir.exists() {
        return Ok(());
    }

    clean_generated_roc_impl(target_dir, &options)
}

fn clean_generated_roc_impl(dir: &Path, options: &CleanOptions) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && options.recursive {
            clean_generated_roc_impl(&path, options)?;
        } else if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("roc")
            && is_generated_roc_file(&path)?
        {
            if options.verbose {
                println!("Deleting generated roc file: {}", path.display());
            }
            fs::remove_file(&path).with_context(|| {
                format!("Failed to delete generated roc file: {}", path.display())
            })?;
        }
    }

    Ok(())
}

fn is_generated_roc_file(path: &Path) -> Result<bool> {
    let file =
        File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
    let mut reader = BufReader::new(file);

    // Try to parse the header - if it succeeds, this is a generated file
    match ModuleHeader::parse_if_valid(&mut reader) {
        Ok(Some(_)) => Ok(true),
        Ok(None) | Err(_) => Ok(false), // If parsing fails, assume it's not a generated file
    }
}

fn generate_roc_modules<'a, 'b>(
    package_name: &str,
    only_modules: Vec<String>,
    type_iter: impl IntoIterator<Item = &'b RegisteredType>,
    associated_dependencies_iter: impl IntoIterator<Item = &'b ir::AssociatedDependencies>,
    associated_constant_iter: impl IntoIterator<Item = &'b ir::AssociatedConstant>,
    associated_function_iter: impl IntoIterator<Item = &'b ir::AssociatedFunction>,
    component_type_ids: &HashSet<RocTypeID>,
    setup_component_type_ids: &HashSet<RocTypeID>,
) -> Result<Vec<(Module, PathBuf)>> {
    let timestamp = obtain_timestamp();

    let type_map = gather_type_map(type_iter, component_type_ids, setup_component_type_ids)?;
    let associated_dependencies_map =
        gather_associated_dependencies_map(associated_dependencies_iter);
    let associated_constant_map = gather_associated_constant_map(associated_constant_iter);
    let associated_function_map = gather_associated_function_map(associated_function_iter);

    let module_filter = ModuleFilter::from_dot_separated(only_modules);

    let mut modules = type_map
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
                package_name,
                &type_map,
                ty,
                associated_dependencies.as_ref(),
                associated_constants.as_ref(),
                associated_functions.as_ref(),
            ) {
                Ok(Some(code)) => {
                    let path_from_package_root = ty
                        .parent_modules
                        .map_or_else(PathBuf::new, |parents| parents.split(".").collect())
                        .join(format!("{}.roc", ty.module_name));

                    let header = ModuleHeader::new(timestamp.clone(), ty, &code);

                    Some(Ok((Module { header, code }, path_from_package_root)))
                }
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            }
        })
        .collect::<Result<Vec<_>>>()?;

    modules.sort_by_key(|(_, path)| path.clone());

    Ok(modules)
}

fn gather_type_map<'a>(
    type_iter: impl IntoIterator<Item = &'a RegisteredType>,
    component_type_ids: &HashSet<RocTypeID>,
    setup_component_type_ids: &HashSet<RocTypeID>,
) -> Result<HashMap<RocTypeID, RegisteredType>> {
    let mut type_map = HashMap::default();

    for ty in type_iter {
        let mut ty = ty.clone();
        if component_type_ids.contains(&ty.ty.id) {
            ty.flags |= RegisteredTypeFlags::IS_COMPONENT;
        }
        if setup_component_type_ids.contains(&ty.ty.id) {
            ty.flags |= RegisteredTypeFlags::IS_SETUP_COMPONENT;
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
    let mut associated_dependencies_map = HashMap::default();
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
    let mut associated_constant_map = HashMap::default();
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
    let mut associated_function_map = HashMap::default();
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

fn obtain_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, false)
}

fn obtain_git_commit_sha() -> Option<&'static str> {
    let sha = env!("VERGEN_GIT_SHA");
    if sha.is_empty() { None } else { Some(sha) }
}

fn git_commit_dirty() -> bool {
    env!("VERGEN_GIT_DIRTY") == "true"
}

fn compute_module_code_hash(code: &str) -> blake3::Hash {
    blake3::hash(code.as_bytes())
}

impl GeneratedTypeCategory {
    fn from_type(ty: &RegisteredType) -> Self {
        assert!(!ty.is_primitive());
        if ty.is_component() {
            Self::Component
        } else if ty.is_pod() {
            Self::Pod
        } else {
            Self::Inline
        }
    }

    fn parse(string: &str) -> Result<Self> {
        match string {
            "Component" => Ok(Self::Component),
            "POD" => Ok(Self::Pod),
            "Inline" => Ok(Self::Inline),
            category => Err(anyhow!("Invalid type category `{category}`")),
        }
    }
}

impl fmt::Display for GeneratedTypeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Component => "Component",
                Self::Pod => "POD",
                Self::Inline => "Inline",
            }
        )
    }
}

impl Module {
    fn save(
        &self,
        target_dir: &Path,
        path_from_package_root: &Path,
        unchanged: &mut Vec<String>,
        generated: &mut Vec<String>,
        warnings: &mut Vec<String>,
        verbose: bool,
    ) -> Result<()> {
        let display_path = path_from_package_root
            .iter()
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join(".");
        let display_path = display_path.strip_suffix(".roc").unwrap_or(&display_path);

        let module_path = target_dir.join(path_from_package_root);

        let exists = module_path.exists();

        if exists {
            let existing_module =
                Self::parse_from_file_if_valid(&module_path).with_context(|| {
                    format!(
                        "Could not read existing module at {}",
                        module_path.display()
                    )
                })?;

            if let Some(existing_module) = existing_module {
                if !existing_module.code_is_unmodified() {
                    warnings.push(format!(
                        "Existing module {display_path} has been modified after it was last generated, skipping"
                    ));
                    return Ok(());
                }
                if existing_module.header.code_hash == self.header.code_hash {
                    if verbose {
                        unchanged.push(format!("No new code for module {display_path}, skipping"));
                    }
                    return Ok(());
                }
            } else {
                warnings.push(format!(
                    "An unrecognized file already exists at {}, skipping",
                    module_path.display()
                ));
                return Ok(());
            }
        }

        if let Some(module_dir) = module_path.parent() {
            fs::create_dir_all(module_dir)?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&module_path)
            .with_context(|| {
                format!(
                    "Could not create file {} for writing",
                    module_path.display()
                )
            })?;

        write!(&mut file, "{}{}", self.header, self.code)?;

        if verbose {
            generated.push(format!(
                "Generated {display_path}{}",
                if exists { " (replaced existing)" } else { "" }
            ));
        }
        Ok(())
    }

    fn code_is_unmodified(&self) -> bool {
        let actual_hash = compute_module_code_hash(&self.code).to_string();
        self.header.code_hash == actual_hash
    }

    fn parse_from_file_if_valid(module_path: &Path) -> Result<Option<Self>> {
        let reader = BufReader::new(File::open(module_path)?);
        Self::parse_if_valid(reader)
    }

    fn parse_if_valid(mut reader: impl BufRead) -> Result<Option<Self>> {
        let Some(header) = ModuleHeader::parse_if_valid(&mut reader)? else {
            return Ok(None);
        };

        let mut code = String::new();
        reader.read_to_string(&mut code)?;

        Ok(Some(Self { header, code }))
    }
}

impl ModuleHeader {
    fn new(timestamp: String, ty: &RegisteredType, module_code: &str) -> Self {
        let code_hash = compute_module_code_hash(module_code).to_string();
        let rust_type_path = ty.rust_type_path.map(ToString::to_string);
        let type_category = GeneratedTypeCategory::from_type(ty);
        let commit_sha = obtain_git_commit_sha().map(ToString::to_string);
        let commit_dirty = git_commit_dirty();
        Self {
            code_hash,
            timestamp,
            rust_type_path,
            type_category,
            commit_sha,
            commit_dirty,
        }
    }

    fn parse_if_valid(reader: &mut impl BufRead) -> Result<Option<Self>> {
        let mut hash_line = String::new();
        reader.read_line(&mut hash_line)?;
        let Some(hash_str) = Self::extract_module_hash_str(&hash_line) else {
            return Ok(None);
        };
        let code_hash = hash_str.to_string();

        let mut timestamp_line = String::new();
        reader.read_line(&mut timestamp_line)?;
        let Some(timestamp_str) = Self::extract_module_header_timestamp_str(&timestamp_line) else {
            return Ok(None);
        };
        let timestamp = timestamp_str.to_string();

        let mut rust_type_line = String::new();
        reader.read_line(&mut rust_type_line)?;
        let Some(rust_type_str) = Self::extract_module_header_rust_type_str(&rust_type_line) else {
            return Ok(None);
        };
        let rust_type_path = if rust_type_str == "-" {
            None
        } else {
            Some(rust_type_str.to_string())
        };

        let mut type_category_line = String::new();
        reader.read_line(&mut type_category_line)?;
        let Some(type_category_str) =
            Self::extract_module_header_type_category_str(&type_category_line)
        else {
            return Ok(None);
        };
        let type_category = GeneratedTypeCategory::parse(type_category_str)?;

        let mut commmit_line = String::new();
        reader.read_line(&mut commmit_line)?;
        let Some(commit_str) = Self::extract_module_header_commit_str(&commmit_line) else {
            return Ok(None);
        };
        let (commit_sha, commit_dirty) = if commit_str == "-" {
            (None, false)
        } else if let Some(commit_sha) = commit_str.strip_suffix(" (dirty)") {
            (Some(commit_sha.to_string()), true)
        } else {
            (Some(commit_str.to_string()), false)
        };

        Ok(Some(Self {
            code_hash,
            timestamp,
            rust_type_path,
            type_category,
            commit_sha,
            commit_dirty,
        }))
    }

    fn extract_module_hash_str(first_line: &str) -> Option<&str> {
        Some(first_line.strip_prefix("# Hash: ")?.trim())
    }

    fn extract_module_header_timestamp_str(second_line: &str) -> Option<&str> {
        Some(second_line.strip_prefix("# Generated: ")?.trim())
    }

    fn extract_module_header_rust_type_str(third_line: &str) -> Option<&str> {
        Some(third_line.strip_prefix("# Rust type: ")?.trim())
    }

    fn extract_module_header_type_category_str(fourth_line: &str) -> Option<&str> {
        Some(fourth_line.strip_prefix("# Type category: ")?.trim())
    }

    fn extract_module_header_commit_str(fifth_line: &str) -> Option<&str> {
        Some(fifth_line.strip_prefix("# Commit: ")?.trim())
    }
}

impl fmt::Display for ModuleHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "# Hash: {}", &self.code_hash)?;
        writeln!(f, "# Generated: {}", &self.timestamp)?;
        writeln!(
            f,
            "# Rust type: {}",
            self.rust_type_path.as_deref().unwrap_or("-")
        )?;
        writeln!(f, "# Type category: {}", self.type_category)?;
        writeln!(
            f,
            "# Commit: {sha}{dirty}",
            sha = self.commit_sha.as_deref().unwrap_or("-"),
            dirty = if self.commit_sha.is_some() && self.commit_dirty {
                " (dirty)"
            } else {
                ""
            }
        )?;
        Ok(())
    }
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
