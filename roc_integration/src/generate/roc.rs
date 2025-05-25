//! Generation of Roc code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

use super::get_field_type;
use crate::{RegisteredType, RocTypeID, ir};
use anyhow::{Context, Result, anyhow};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
};

pub(super) fn generate_module(
    package_name: &str,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
    associated_dependencies: &[ir::AssociatedDependencies],
    associated_constants: &[ir::AssociatedConstant],
    associated_functions: &[ir::AssociatedFunction],
) -> Result<Option<String>> {
    if ty.is_primitive()
        || ty
            .package_name
            .is_some_and(|type_package| type_package != package_name)
    {
        return Ok(None);
    }

    let mut module_body = String::new();
    let mut optional_exports = OptionalExports::new();
    let mut optional_imports = OptionalImports::new(package_name);

    write_type_declaration(&mut module_body, type_map, &ty.ty)?;

    write_associated_constants(
        &mut module_body,
        &mut optional_exports,
        type_map,
        ty,
        associated_constants,
    )?;

    write_associated_functions(
        &mut module_body,
        &mut optional_exports,
        type_map,
        ty,
        associated_functions,
    )?;

    write_component_functions(
        &mut module_body,
        &mut optional_exports,
        &mut optional_imports,
        ty,
    )?;

    write_write_bytes_function(&mut module_body, type_map, ty)?;

    write_from_bytes_function(&mut module_body, type_map, ty)?;

    write_roundtrip_test(&mut module_body, ty)?;

    let mut module = String::with_capacity(module_body.len());

    write_module_header(
        &mut module,
        optional_exports,
        associated_constants,
        associated_functions,
        ty,
    )?;

    write_imports(
        &mut module,
        package_name,
        optional_imports,
        type_map,
        associated_dependencies,
        ty,
    )?;

    module.push_str(&module_body);

    Ok(Some(module))
}

#[derive(Clone, Debug)]
pub struct OptionalExports {
    export_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct OptionalImports {
    current_package_name: String,
    import_paths: HashSet<String>,
}

impl OptionalExports {
    pub fn new() -> Self {
        Self {
            export_names: Vec::new(),
        }
    }

    pub fn add(&mut self, name: impl ToString) {
        self.export_names.push(name.to_string());
    }
}

impl OptionalImports {
    pub fn new(current_package_name: impl ToString) -> Self {
        Self {
            current_package_name: current_package_name.to_string(),
            import_paths: HashSet::new(),
        }
    }

    pub fn add(&mut self, package_name: impl AsRef<str>, module_path: impl AsRef<str>) {
        let package_name = package_name.as_ref();
        let module_path = module_path.as_ref();
        if package_name == self.current_package_name {
            self.import_paths.insert(module_path.to_string());
        } else {
            self.import_paths
                .insert(format!("{package_name}.{module_path}"));
        }
    }
}

fn write_module_header(
    roc_code: &mut String,
    optional_exports: OptionalExports,
    associated_constants: &[ir::AssociatedConstant],
    associated_functions: &[ir::AssociatedFunction],
    ty: &RegisteredType,
) -> Result<()> {
    write!(
        roc_code,
        "\
        module [\n    \
            {},\n\
        ",
        ty.ty.name,
    )?;

    for constant in associated_constants {
        writeln!(roc_code, "    {},", constant.name)?;
    }

    for function in associated_functions {
        writeln!(roc_code, "    {},", function.name)?;
    }

    for export_name in optional_exports.export_names {
        writeln!(roc_code, "    {},", export_name)?;
    }

    roc_code.push_str(
        "    \
            write_bytes,\n    \
            from_bytes,\n\
        ]\n\
        \n",
    );

    Ok(())
}

fn write_imports(
    roc_code: &mut String,
    package_name: &str,
    optional_imports: OptionalImports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    associated_dependencies: &[ir::AssociatedDependencies],
    ty: &RegisteredType,
) -> Result<()> {
    let mut imports = Vec::from_iter(determine_imports(
        package_name,
        optional_imports,
        type_map,
        associated_dependencies,
        ty,
    ));
    imports.sort();
    for import in &imports {
        writeln!(roc_code, "import {import}")?;
    }
    if !imports.is_empty() {
        roc_code.push('\n');
    }
    Ok(())
}

fn determine_imports(
    package_name: &str,
    optional_imports: OptionalImports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    associated_dependencies: &[ir::AssociatedDependencies],
    ty: &RegisteredType,
) -> HashSet<String> {
    let mut import_paths = optional_imports.import_paths;

    for associated_dependencies in associated_dependencies {
        add_imports_for_associated_dependencies(
            &mut import_paths,
            package_name,
            type_map,
            associated_dependencies,
        );
    }

    match &ty.ty.composition {
        ir::TypeComposition::Primitive(_) => {}
        ir::TypeComposition::Struct { fields, .. } => {
            add_imports_for_fields(&mut import_paths, package_name, type_map, fields);
        }
        ir::TypeComposition::Enum(variants) => {
            for variant in &variants.0 {
                add_imports_for_fields(&mut import_paths, package_name, type_map, &variant.fields);
            }
        }
    }
    import_paths
}

fn add_imports_for_associated_dependencies(
    import_paths: &mut HashSet<String>,
    package_name: &str,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    associated_dependencies: &ir::AssociatedDependencies,
) {
    for dependency_id in &associated_dependencies.dependencies {
        if let Some(dependency) = type_map.get(dependency_id) {
            import_paths.insert(dependency.import_path(package_name));
        }
    }
}

fn add_imports_for_fields<const N: usize>(
    import_paths: &mut HashSet<String>,
    package_name: &str,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    fields: &ir::TypeFields<N>,
) {
    match fields {
        ir::TypeFields::None => {}
        ir::TypeFields::Named(fields) => {
            for ir::NamedTypeField { ty, .. } in fields {
                let type_id = match ty {
                    ir::FieldType::Single { type_id } => type_id,
                    ir::FieldType::Array { elem_type_id, .. } => elem_type_id,
                };
                if let Some(field_ty) = type_map.get(type_id) {
                    import_paths.insert(field_ty.import_path(package_name));
                }
            }
        }
        ir::TypeFields::Unnamed(fields) => {
            for ir::UnnamedTypeField { ty } in fields {
                let type_id = match ty {
                    ir::FieldType::Single { type_id } => type_id,
                    ir::FieldType::Array { elem_type_id, .. } => elem_type_id,
                };
                if let Some(field_ty) = type_map.get(type_id) {
                    import_paths.insert(field_ty.import_path(package_name));
                }
            }
        }
    }
}

fn write_type_declaration(
    roc_code: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &ir::Type,
) -> Result<()> {
    if !ty.docstring.is_empty() {
        roc_code.push_str(ty.docstring);
    }
    match &ty.composition {
        // We don't generate code for primitive types
        ir::TypeComposition::Primitive(_) => Ok(()),
        ir::TypeComposition::Struct { fields, .. } => {
            write!(roc_code, "{} : ", ty.name)?;
            write_fields_declaration(roc_code, type_map, fields, 0, false, &|| {
                format!("struct type {}", ty.name)
            })?;
            roc_code.push_str("\n\n");
            Ok(())
        }
        ir::TypeComposition::Enum(variants) => {
            write!(roc_code, "{} : [", ty.name)?;
            let mut variant_count = 0;
            for variant in &variants.0 {
                if !variant.docstring.is_empty() {
                    for line in variant.docstring.lines() {
                        write!(roc_code, "\n    {line}")?;
                    }
                }
                write!(roc_code, "\n    {}", variant.ident)?;
                if !matches!(variant.fields, ir::TypeFields::None) {
                    roc_code.push(' ');
                    write_fields_declaration(
                        roc_code,
                        type_map,
                        &variant.fields,
                        2, // 1 looks more right, but 2 is consistent with Roc autoformatting
                        true,
                        &|| format!("variant {} of enum {}", variant.ident, ty.name),
                    )?;
                }
                roc_code.push(',');
                variant_count += 1;
            }
            if variant_count > 0 {
                roc_code.push('\n');
            }
            roc_code.push_str("]\n\n");
            Ok(())
        }
    }
}

fn write_fields_declaration<const N: usize>(
    declaration: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    fields: &ir::TypeFields<N>,
    indentation_level: usize,
    undelimited_tuple: bool,
    parent_name: &impl Fn() -> String,
) -> Result<()> {
    let indentation = "    ".repeat(indentation_level);
    match fields {
        ir::TypeFields::None => {
            declaration.push_str("{}");
        }
        ir::TypeFields::Named(fields) => {
            declaration.push('{');
            let mut field_count = 0;
            for ir::NamedTypeField {
                docstring,
                ident,
                ty,
            } in fields
            {
                if !docstring.is_empty() {
                    for line in docstring.lines() {
                        write!(declaration, "\n{indentation}    {line}")?;
                    }
                }

                let type_name = qualified_type_name_for_field(type_map, ty, ident, parent_name)?;
                write!(declaration, "\n{indentation}    {ident} : {type_name},")?;

                field_count += 1;
            }
            if field_count > 0 {
                declaration.push('\n');
            }
            write!(declaration, "{indentation}}}")?;
        }

        ir::TypeFields::Unnamed(fields) => {
            if !undelimited_tuple && fields.len() > 1 {
                declaration.push('(');
            }
            for (field_idx, ir::UnnamedTypeField { ty }) in fields.iter().enumerate() {
                let type_name =
                    qualified_type_name_for_field(type_map, ty, field_idx, parent_name)?;
                if field_idx > 0 {
                    if !undelimited_tuple {
                        declaration.push(',');
                    }
                    declaration.push(' ');
                }
                declaration.push_str(&type_name);
            }
            if !undelimited_tuple && fields.len() > 1 {
                declaration.push(')');
            }
        }
    }
    Ok(())
}

fn qualified_type_name_for_field(
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &ir::FieldType,
    field: impl Display,
    parent_name: &impl Fn() -> String,
) -> Result<Cow<'static, str>> {
    Ok(match ty {
        ir::FieldType::Single { type_id } => get_field_type(type_map, type_id, field, parent_name)?
            .qualified_type_name(ir::TypeUsage::Concrete),
        ir::FieldType::Array { elem_type_id, .. } => {
            let mut type_name = String::from("List ");
            let elem_type_name = get_field_type(type_map, elem_type_id, field, parent_name)?
                .qualified_type_name(ir::TypeUsage::TypeParameter);
            write!(&mut type_name, "{}", elem_type_name)?;
            Cow::Owned(type_name)
        }
    })
}

fn write_associated_constants(
    roc_code: &mut String,
    optional_exports: &mut OptionalExports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
    associated_constants: &[ir::AssociatedConstant],
) -> Result<()> {
    for associated_constant in associated_constants {
        write_associated_constant(
            roc_code,
            optional_exports,
            type_map,
            ty,
            associated_constant,
        )?;
        roc_code.push('\n');
    }
    Ok(())
}

pub(super) fn write_associated_constant(
    roc_code: &mut String,
    optional_exports: &mut OptionalExports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
    associated_constant: &ir::AssociatedConstant,
) -> Result<()> {
    let type_name = qualified_type_name_for_containable_type(
        |type_map, contained_ty, in_container| {
            qualified_type_name_for_inferrable_type(
                qualified_type_name_for_translatable_type,
                type_map,
                contained_ty,
                Cow::Borrowed(ty.ty.name),
                in_container,
            )
        },
        type_map,
        &associated_constant.ty,
    )
    .with_context(|| {
        format!(
            "Invalid type for associated constant {}",
            associated_constant.name
        )
    })?;

    let docstring = if associated_constant.docstring.is_empty() {
        ""
    } else {
        associated_constant.docstring
    };

    writeln!(
        roc_code,
        "\
        {docstring}\
        {name} : {type_name}\n\
        {name} = {expr}\
        ",
        name = associated_constant.name,
        expr = associated_constant.expr.trim(),
    )?;

    if ty.is_component()
        && matches!(
            associated_constant.ty,
            ir::Containable::Single(ir::Inferrable::SelfType)
        )
    {
        let name = associated_constant.name;

        optional_exports.add(format!("add_{name}"));
        optional_exports.add(format!("add_multiple_{name}"));

        let (docstring, docstring_multi) = if docstring.is_empty() {
            (String::new(), String::new())
        } else {
            (
                format!("{docstring}## Adds the component to the given entity's data.\n"),
                format!(
                    "{docstring}\
                     ## Adds multiple values of the component to the data of\n\
                     ## a set of entities of the same archetype's data.\n"
                ),
            )
        };
        if ty.serialized_size == 0 {
            writeln!(
                roc_code,
                "\n\
                {docstring}\
                add_{name} : Entity.Data -> Entity.Data\n\
                add_{name} = |entity_data|\n    \
                    add(entity_data)\n\
                \n\
                {docstring_multi}\
                add_multiple_{name} : Entity.MultiData -> Entity.MultiData\n\
                add_multiple_{name} = |entity_data|\n    \
                    add_multiple(entity_data)\n\
                "
            )?;
        } else {
            writeln!(
                roc_code,
                "\n\
                {docstring}\
                add_{name} : Entity.Data -> Entity.Data\n\
                add_{name} = |entity_data|\n    \
                    add(entity_data, {name})\n\
                \n\
                {docstring_multi}\
                add_multiple_{name} : Entity.MultiData -> Entity.MultiData\n\
                add_multiple_{name} = |entity_data|\n    \
                    res = add_multiple(\n        \
                        entity_data,\n        \
                        Same({name})\n    \
                    )\n    \
                    when res is\n        \
                        Ok(res_data) -> res_data\n        \
                        Err(err) -> crash \"unexpected error in {ty_name}.add_multiple_{name}: ${{Inspect.to_str(err)}}\"\n\
                ",
                ty_name = ty.ty.name,
            )?;
        }
    }

    Ok(())
}

fn write_associated_functions(
    roc_code: &mut String,
    optional_exports: &mut OptionalExports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
    associated_functions: &[ir::AssociatedFunction],
) -> Result<()> {
    for associated_function in associated_functions {
        write_associated_function(
            roc_code,
            optional_exports,
            type_map,
            ty,
            associated_function,
        )?;
        roc_code.push('\n');
    }
    Ok(())
}

pub(super) fn write_associated_function(
    roc_code: &mut String,
    optional_exports: &mut OptionalExports,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
    associated_function: &ir::AssociatedFunction,
) -> Result<()> {
    let mut arg_name_list = Vec::with_capacity(associated_function.arguments.0.len());
    let mut arg_type_list = Vec::with_capacity(arg_name_list.capacity());

    for arg in &associated_function.arguments.0 {
        match arg {
            ir::FunctionArgument::Receiver(
                ir::MethodReceiver::RefSelf | ir::MethodReceiver::OwnedSelf,
            ) => {
                arg_name_list.push("self");
                arg_type_list.push(Cow::Borrowed(ty.ty.name));
            }
            ir::FunctionArgument::Typed(arg) => {
                arg_name_list.push(arg.ident);
                arg_type_list.push(
                    qualified_type_name_for_containable_type(
                        |type_map, contained_ty, in_container| {
                            qualified_type_name_for_inferrable_type(
                                qualified_type_name_for_translatable_type,
                                type_map,
                                contained_ty,
                                Cow::Borrowed(ty.ty.name),
                                in_container,
                            )
                        },
                        type_map,
                        &arg.ty,
                    )
                    .with_context(|| {
                        format!(
                            "Invalid type for argument {} of associated function {}",
                            arg.ident, associated_function.name
                        )
                    })?,
                );
            }
        }
    }

    let arg_names = arg_name_list.join(", ");
    let arg_types = arg_type_list.join(", ");

    let (non_empty_arg_names, non_empty_arg_types) = if arg_names.is_empty() {
        ("{}", "{}")
    } else {
        (arg_names.as_str(), arg_types.as_str())
    };

    let return_type = qualified_type_name_for_containable_type(
        |type_map, contained_ty, in_container| {
            qualified_type_name_for_inferrable_type(
                qualified_type_name_for_translatable_type,
                type_map,
                contained_ty,
                Cow::Borrowed(ty.ty.name),
                in_container,
            )
        },
        type_map,
        &associated_function.return_type,
    )
    .with_context(|| {
        format!(
            "Invalid return type for associated function {}",
            associated_function.name
        )
    })?;

    let docstring = if associated_function.docstring.is_empty() {
        ""
    } else {
        associated_function.docstring
    };

    writeln!(
        roc_code,
        "\
        {docstring}\
        {name} : {non_empty_arg_types} -> {return_type}\n\
        {name} = |{non_empty_arg_names}|\n    \
            {body}\
        ",
        name = associated_function.name,
        body = associated_function.body.trim(),
    )?;

    if ty.is_component()
        && matches!(
            associated_function.return_type,
            ir::Containable::Single(ir::Inferrable::SelfType)
        )
    {
        let name = associated_function.name;

        optional_exports.add(format!("add_{name}"));

        let (docstring, docstring_multi) = if docstring.is_empty() {
            (String::new(), String::new())
        } else {
            (
                format!("{docstring}## Adds the component to the given entity's data.\n"),
                format!(
                    "{docstring}\
                     ## Adds multiple values of the component to the data of\n\
                     ## a set of entities of the same archetype's data.\n"
                ),
            )
        };

        if arg_type_list.is_empty() {
            optional_exports.add(format!("add_multiple_{name}"));

            if ty.serialized_size == 0 {
                writeln!(
                    roc_code,
                    "\n\
                    {docstring}\
                    add_{name} : Entity.Data -> Entity.Data\n\
                    add_{name} = |entity_data|\n    \
                        add(entity_data)\n\
                    \n\
                    {docstring_multi}\
                    add_multiple_{name} : Entity.MultiData -> Entity.MultiData\n\
                    add_multiple_{name} = |entity_data|\n    \
                        add_multiple(entity_data)\
                    "
                )?;
            } else {
                writeln!(
                    roc_code,
                    "\n\
                    {docstring}\
                    add_{name} : Entity.Data -> Entity.Data\n\
                    add_{name} = |entity_data|\n    \
                        add(entity_data, {name}({{}}))\n\
                    \n\
                    {docstring_multi}\
                    add_multiple_{name} : Entity.MultiData -> Entity.MultiData\n\
                    add_multiple_{name} = |entity_data|\n    \
                        res = add_multiple(\n        \
                            entity_data,\n        \
                            Same({name}({{}}))\n    \
                        )\n    \
                        when res is\n        \
                            Ok(res_data) -> res_data\n        \
                            Err(err) -> crash \"unexpected error in {ty_name}.add_multiple_{name}: ${{Inspect.to_str(err)}}\"\
                    ",
                    ty_name = ty.ty.name,
                )?;
            }
        } else {
            writeln!(
                roc_code,
                "\n\
                {docstring}\
                add_{name} : Entity.Data, {arg_types} -> Entity.Data\n\
                add_{name} = |entity_data, {arg_names}|\n    \
                    add(entity_data, {name}({arg_names}))\
                "
            )?;

            // This only works with up to four arguments until we have map5
            if arg_type_list.len() <= 4 {
                optional_exports.add(format!("add_multiple_{name}"));

                writeln!(
                    roc_code,
                    "\n\
                    {docstring_multi}\
                    add_multiple_{name} : Entity.MultiData, {wrapped_arg_types} -> Result Entity.MultiData Str\n\
                    add_multiple_{name} = |entity_data, {arg_names}|\n    \
                        add_multiple(\n        \
                            entity_data,\n        \
                            All(Entity.Arg.broadcasted_map{arg_count}(\n            \
                                {arg_names},\n            \
                                Entity.multi_count(entity_data),\n            \
                                {name}\n        \
                            ))\n    \
                        )\
                    ",
                    wrapped_arg_types = arg_type_list
                        .iter()
                        .map(|arg_type| format!("Entity.Arg.Broadcasted ({arg_type})"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    arg_count = arg_type_list.len(),
                )?;
            }
        }
    }

    Ok(())
}

fn qualified_type_name_for_containable_type<T, R>(
    type_name_for_contained_type: R,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &ir::Containable<T>,
) -> Result<Cow<'static, str>>
where
    R: Fn(&HashMap<RocTypeID, RegisteredType>, &T, ir::TypeUsage) -> Result<Cow<'static, str>>,
{
    match ty {
        ir::Containable::Single(ty) => {
            type_name_for_contained_type(type_map, ty, ir::TypeUsage::Concrete)
        }
        ir::Containable::List(ty) => {
            type_name_for_contained_type(type_map, ty, ir::TypeUsage::TypeParameter)
                .map(|type_name| Cow::Owned(format!("List {type_name}")))
                .context("Invalid list element type")
        }
        ir::Containable::Tuple2(ty0, ty1) => {
            let type_name_0 = type_name_for_contained_type(type_map, ty0, ir::TypeUsage::Concrete)
                .context("Invalid type for tuple element 0")?;
            let type_name_1 = type_name_for_contained_type(type_map, ty1, ir::TypeUsage::Concrete)
                .context("Invalid type for tuple element 1")?;
            Ok(Cow::Owned(format!("({type_name_0}, {type_name_1})")))
        }
        ir::Containable::Tuple3(ty0, ty1, ty2) => {
            let type_name_0 = type_name_for_contained_type(type_map, ty0, ir::TypeUsage::Concrete)
                .context("Invalid type for tuple element 0")?;
            let type_name_1 = type_name_for_contained_type(type_map, ty1, ir::TypeUsage::Concrete)
                .context("Invalid type for tuple element 1")?;
            let type_name_2 = type_name_for_contained_type(type_map, ty2, ir::TypeUsage::Concrete)
                .context("Invalid type for tuple element 2")?;
            Ok(Cow::Owned(format!(
                "({type_name_0}, {type_name_1}, {type_name_2})"
            )))
        }
        ir::Containable::Result(ty) => {
            type_name_for_contained_type(type_map, ty, ir::TypeUsage::TypeParameter)
                .map(|type_name| Cow::Owned(format!("Result {type_name} Str")))
                .context("Invalid Result Ok type")
        }
    }
}

fn qualified_type_name_for_inferrable_type<T, R>(
    type_name_for_specific_type: R,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &ir::Inferrable<T>,
    self_ty_name: Cow<'static, str>,
    usage: ir::TypeUsage,
) -> Result<Cow<'static, str>>
where
    R: Fn(&HashMap<RocTypeID, RegisteredType>, &T, ir::TypeUsage) -> Result<Cow<'static, str>>,
{
    match ty {
        ir::Inferrable::SelfType => Ok(self_ty_name),
        ir::Inferrable::Specific(specific_ty) => {
            type_name_for_specific_type(type_map, specific_ty, usage)
        }
    }
}

fn qualified_type_name_for_translatable_type(
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &ir::TranslatableType,
    usage: ir::TypeUsage,
) -> Result<Cow<'static, str>> {
    match ty {
        ir::TranslatableType::Registered(type_id) => type_map
            .get(type_id)
            .ok_or_else(|| anyhow!("Type not registered"))
            .map(|ty| ty.qualified_type_name(usage)),
        ir::TranslatableType::Special(ty) => Ok(Cow::Borrowed(type_name_for_special_type(ty))),
    }
}

fn type_name_for_special_type(ty: &ir::SpecialType) -> &'static str {
    match ty {
        ir::SpecialType::String => "Str",
    }
}

fn write_component_functions(
    roc_code: &mut String,
    optional_exports: &mut OptionalExports,
    optional_imports: &mut OptionalImports,
    ty: &RegisteredType,
) -> Result<()> {
    if !ty.is_component() {
        return Ok(());
    }

    optional_exports.add("add");
    optional_exports.add("add_multiple");

    optional_imports.add("pf", "Entity");
    optional_imports.add("pf", "Entity.Arg");
    optional_imports.add("core", "Builtin");

    let alignment = ty.alignment_as_pod_struct().ok_or_else(|| {
        anyhow!(
            "\
            Component type {} is not registered as POD: \
            make sure to derive the `RocPod` trait rather \
            than the `Roc` trait for component types\
            ",
            ty.ty.name,
        )
    })?;

    if ty.serialized_size == 0 {
        writeln!(
            roc_code,
            "\
            ## Adds the [{name}] component to an entity's data.\n\
            add : Entity.Data -> Entity.Data\n\
            add = |entity_data|\n    \
                entity_data |> Entity.append_component(write_packet, {{}})\n\
            \n\
            ## Adds the [{name}] component to each entity's data.\n\
            add_multiple : Entity.MultiData -> Entity.MultiData\n\
            add_multiple = |entity_data|\n    \
                res = entity_data\n        \
                    |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(Same({{}}), Entity.multi_count(entity_data)))\n    \
                when res is\n        \
                    Ok(res_data) -> res_data\n        \
                    Err(err) -> crash \"unexpected error in {name}.add_multiple: ${{Inspect.to_str(err)}}\"\n\
            ",
            name = ty.ty.name,
        )?;
    } else {
        writeln!(
            roc_code,
            "\
            ## Adds a value of the [{name}] component to an entity's data.\n\
            ## Note that an entity never should have more than a single value of\n\
            ## the same component type.\n\
            add : Entity.Data, {name} -> Entity.Data\n\
            add = |entity_data, comp_value|\n    \
                entity_data |> Entity.append_component(write_packet, comp_value)\n\
            \n\
            ## Adds multiple values of the [{name}] component to the data of\n\
            ## a set of entities of the same archetype's data.\n\
            ## Note that the number of values should match the number of entities\n\
            ## in the set and that an entity never should have more than a single\n\
            ## value of the same component type.\n\
            add_multiple : Entity.MultiData, Entity.Arg.Broadcasted ({name}) -> Result Entity.MultiData Str\n\
            add_multiple = |entity_data, comp_values|\n    \
                entity_data\n    \
                |> Entity.append_components(write_multi_packet, Entity.Arg.broadcast(comp_values, Entity.multi_count(entity_data)))\n    \
                |> Result.map_err(\n        \
                    |CountMismatch(new_count, orig_count)|\n            \
                        \"Got ${{Inspect.to_str(new_count)}} values in {name}.add_multiple, expected ${{Inspect.to_str(orig_count)}}\",\n    \
                )\n\
            ",
            name = ty.ty.name,
        )?;
    }

    writeln!(
        roc_code,
        "\
        write_packet : List U8, {name} -> List U8\n\
        write_packet = |bytes, val|\n    \
            type_id = {type_id}\n    \
            size = {size}\n    \
            alignment = {alignment}\n    \
            bytes\n    \
            |> List.reserve(24 + size)\n    \
            |> Builtin.write_bytes_u64(type_id)\n    \
            |> Builtin.write_bytes_u64(size)\n    \
            |> Builtin.write_bytes_u64(alignment)\n    \
            |> write_bytes(val)\n\
        \n\
        write_multi_packet : List U8, List {name} -> List U8\n\
        write_multi_packet = |bytes, vals|\n    \
            type_id = {type_id}\n    \
            size = {size}\n    \
            alignment = {alignment}\n    \
            count = List.len(vals)\n    \
            bytes_with_header =\n        \
                bytes\n        \
                |> List.reserve(32 + size * count)\n        \
                |> Builtin.write_bytes_u64(type_id)\n        \
                |> Builtin.write_bytes_u64(size)\n        \
                |> Builtin.write_bytes_u64(alignment)\n        \
                |> Builtin.write_bytes_u64(count)\n    \
            vals\n    \
            |> List.walk(\n        \
                bytes_with_header,\n        \
                |bts, value| bts |> write_bytes(value),\n    \
            )\n\
        ",
        type_id = ty.ty.id.as_u64(),
        size = ty.serialized_size,
        name = ty.ty.name,
    )?;

    Ok(())
}

fn write_write_bytes_function(
    roc_code: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
) -> Result<()> {
    // We don't generate code for primitive types
    if ty.is_primitive() {
        return Ok(());
    }

    write!(
        roc_code,
        "\
        ## Serializes a value of [{name}] into the binary representation\n\
        ## expected by the engine and appends the bytes to the list.\n\
        write_bytes : List U8, {name} -> List U8\n\
        write_bytes = |bytes, {underscore}value|\n\
        ",
        name = ty.ty.name,
        underscore = if ty.serialized_size == 0 { "_" } else { "" }
    )?;

    match &ty.ty.composition {
        ir::TypeComposition::Struct { fields, .. } => {
            roc_code.push_str("    bytes\n");
            if ty.serialized_size > 0 {
                writeln!(
                    roc_code,
                    "    |> List.reserve({size})",
                    size = ty.serialized_size,
                )?;
            }
            write_calls_to_write_bytes(
                roc_code,
                type_map,
                fields,
                1,
                |def, field| write!(def, "value.{field}"),
                |def, idx| write!(def, "value.{idx}"),
                |def| {
                    def.push_str("value");
                },
                &|| format!("struct type {}", ty.ty.name),
            )?;
        }
        ir::TypeComposition::Enum(variants) => {
            writeln!(roc_code, "    when value is")?;
            for (variant_idx, variant) in variants.0.iter().enumerate() {
                if variant_idx > 0 {
                    roc_code.push('\n');
                }
                match &variant.fields {
                    ir::TypeFields::None => {
                        writeln!(roc_code, "        {} ->", variant.ident)?;
                    }
                    ir::TypeFields::Named(fields) => {
                        write!(roc_code, "        {} {{", variant.ident)?;
                        let mut has_fields = false;
                        for (field_idx, field) in fields.iter().enumerate() {
                            if field_idx > 0 {
                                roc_code.push(',');
                            }
                            write!(roc_code, " {}", field.ident)?;
                            has_fields = true;
                        }
                        if has_fields {
                            roc_code.push(' ');
                        }
                        roc_code.push_str("} ->\n");
                    }
                    ir::TypeFields::Unnamed(fields) => {
                        write!(roc_code, "        {}(", variant.ident)?;
                        if fields.len() == 1 {
                            roc_code.push_str("val");
                        } else {
                            for (field_idx, _) in fields.iter().enumerate() {
                                if field_idx > 0 {
                                    roc_code.push_str(", ");
                                }
                                write!(roc_code, "x{}", field_idx)?;
                            }
                        }
                        roc_code.push_str(") ->\n");
                    }
                }
                write!(
                    roc_code,
                    "            \
                    bytes\n            \
                    |> List.reserve({size})\n            \
                    |> List.append({discriminant})\n\
                    ",
                    size = ty.serialized_size,
                    discriminant = variant_idx,
                )?;
                write_calls_to_write_bytes(
                    roc_code,
                    type_map,
                    &variant.fields,
                    3,
                    |def, field| write!(def, "{field}"),
                    |def, idx| write!(def, "x{idx}"),
                    |def| {
                        def.push_str("val");
                    },
                    &|| format!("variant {} of enum {}", variant.ident, ty.ty.name),
                )?;

                let padding_size = ty
                    .serialized_size
                    .checked_sub(variant.serialized_size + 1)
                    .unwrap();
                if padding_size > 0 {
                    writeln!(
                        roc_code,
                        "            \
                        |> List.concat(List.repeat(0, {padding_size}))\
                        ",
                    )?;
                }
            }
        }
        ir::TypeComposition::Primitive(_) => {
            unreachable!()
        }
    }
    roc_code.push('\n');
    Ok(())
}

fn write_calls_to_write_bytes<const N: usize>(
    write_bytes_definition: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    fields: &ir::TypeFields<N>,
    indentation_level: usize,
    mut write_struct_value_access: impl FnMut(&mut String, &str) -> std::fmt::Result,
    mut write_tuple_value_access: impl FnMut(&mut String, &str) -> std::fmt::Result,
    mut write_whole_value_access: impl FnMut(&mut String),
    parent_name: &impl Fn() -> String,
) -> Result<()> {
    let indentation = "    ".repeat(indentation_level);

    let write_until_value_access = |write_bytes_definition: &mut String,
                                    ty: &ir::FieldType,
                                    field: &str|
     -> Result<()> {
        match ty {
            ir::FieldType::Single { type_id } => {
                let field_ty = get_field_type(type_map, type_id, field, parent_name)?;
                write!(
                    write_bytes_definition,
                    "{indentation}|> {}(",
                    field_ty.write_bytes_func_name(),
                )?;
            }
            ir::FieldType::Array { elem_type_id, .. } => {
                let elem_field_ty = get_field_type(type_map, elem_type_id, field, parent_name)?;
                write!(
                    write_bytes_definition,
                    "\
                    {indentation}|> (|bts, values| values |> List.walk(bts, |b, val| b |> {}(val)))(\
                ",
                    elem_field_ty.write_bytes_func_name(),
                )?;
            }
        }
        Ok(())
    };

    match fields {
        ir::TypeFields::None => {}
        ir::TypeFields::Named(fields) => {
            for ir::NamedTypeField { ident, ty, .. } in fields {
                write_until_value_access(write_bytes_definition, ty, ident)?;
                write_struct_value_access(write_bytes_definition, ident)?;
                writeln!(write_bytes_definition, ")")?;
            }
        }
        ir::TypeFields::Unnamed(fields) => {
            let is_single = fields.len() == 1;
            for (field_idx, ir::UnnamedTypeField { ty }) in fields.iter().enumerate() {
                let ident = field_idx.to_string();
                write_until_value_access(write_bytes_definition, ty, &ident)?;
                if is_single {
                    write_whole_value_access(write_bytes_definition);
                } else {
                    write_tuple_value_access(write_bytes_definition, &ident)?;
                }
                writeln!(write_bytes_definition, ")")?;
            }
        }
    }
    Ok(())
}

fn write_from_bytes_function(
    roc_code: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    ty: &RegisteredType,
) -> Result<()> {
    if matches!(ty.ty.composition, ir::TypeComposition::Primitive(_)) {
        return Ok(());
    }

    write!(
        roc_code,
        "\
        ## Deserializes a value of [{name}] from its bytes in the\n\
        ## representation used by the engine.\n\
        from_bytes : List U8 -> Result {name} _\n\
        from_bytes = |{underscore}bytes|\n\
        ",
        name = ty.ty.name,
        underscore = if ty.serialized_size == 0 { "_" } else { "" }
    )?;

    match &ty.ty.composition {
        ir::TypeComposition::Struct {
            fields: ir::TypeFields::None,
            ..
        } => {
            roc_code.push_str("    Ok({})\n");
        }
        ir::TypeComposition::Struct { fields, .. } => {
            roc_code.push_str("    Ok(\n        ");
            write_calls_to_from_bytes(roc_code, type_map, fields, 2, "bytes", &|| {
                format!("struct type {}", ty.ty.name)
            })?;
            writeln!(roc_code, "    )")?;
        }
        ir::TypeComposition::Enum(variants) => {
            writeln!(
                roc_code,
                "    \
                if List.len(bytes) != {size} then\n        \
                    Err(InvalidNumberOfBytes)\n    \
                else\n        \
                    when bytes is\
                ",
                size = ty.serialized_size
            )?;
            for (variant_idx, variant) in variants.0.iter().enumerate() {
                match &variant.fields {
                    ir::TypeFields::None => {
                        writeln!(
                            roc_code,
                            "            \
                            [{variant_idx}, ..] -> Ok({})\
                            ",
                            variant.ident
                        )?;
                    }
                    ir::TypeFields::Named(_) => {
                        write!(
                            roc_code,
                            "            \
                            [{variant_idx}, .. as data_bytes] ->\n                \
                                Ok(\n                    \
                                    {}     \
                            ",
                            variant.ident
                        )?;
                        write_calls_to_from_bytes(
                            roc_code,
                            type_map,
                            &variant.fields,
                            5,
                            "data_bytes",
                            &|| format!("variant {} of enum {}", variant.ident, ty.ty.name),
                        )?;
                        roc_code.push_str(
                            "                \
                                )\n\n\
                            ",
                        );
                        if variant_idx > 0 {
                            roc_code.push('\n');
                        }
                    }
                    ir::TypeFields::Unnamed(_) => {
                        write!(
                            roc_code,
                            "            \
                            [{variant_idx}, .. as data_bytes] ->\n                \
                                Ok(\n                    \
                                    {}\
                            ",
                            variant.ident
                        )?;
                        write_calls_to_from_bytes(
                            roc_code,
                            type_map,
                            &variant.fields,
                            5,
                            "data_bytes",
                            &|| format!("variant {} of enum {}", variant.ident, ty.ty.name),
                        )?;
                        roc_code.push_str(
                            "                \
                                )\n\n\
                            ",
                        );
                    }
                }
            }
            roc_code.push_str(
                "            \
                [] -> Err(MissingDiscriminant)\n            \
                [discr, ..] -> Err(InvalidDiscriminant(discr))\n\
                ",
            );
        }
        ir::TypeComposition::Primitive(_) => {
            unreachable!()
        }
    }
    Ok(())
}

fn write_calls_to_from_bytes<const N: usize>(
    from_bytes_definition: &mut String,
    type_map: &HashMap<RocTypeID, RegisteredType>,
    fields: &ir::TypeFields<N>,
    indentation_level: usize,
    bytes_name: &str,
    parent_name: &impl Fn() -> String,
) -> Result<()> {
    let indentation = "    ".repeat(indentation_level);

    let write_single = |from_bytes_definition: &mut String,
                        type_id: &RocTypeID,
                        field: &str,
                        write_ident: bool,
                        byte_offset: &mut usize|
     -> Result<()> {
        let field_ty = get_field_type(type_map, type_id, field, parent_name)?;
        write!(from_bytes_definition, "{indentation}    ")?;
        if write_ident {
            write!(from_bytes_definition, "{field}: ")?;
        }
        writeln!(
            from_bytes_definition,
            "{bytes_name} |> List.sublist({{ start: {byte_offset}, len: {size} }}) |> {from_bytes}?,",
            size = field_ty.serialized_size,
            from_bytes = field_ty.from_bytes_func_name(),
        )?;
        *byte_offset += field_ty.serialized_size;
        Ok(())
    };

    let write_array = |from_bytes_definition: &mut String,
                       elem_type_id: &RocTypeID,
                       len: usize,
                       field: &str,
                       write_ident: bool,
                       byte_offset: &mut usize|
     -> Result<()> {
        let elem_field_ty = get_field_type(type_map, elem_type_id, field, parent_name)?;
        write!(from_bytes_definition, "{indentation}    ")?;
        if write_ident {
            write!(from_bytes_definition, "{field}: ")?;
        }
        writeln!(
            from_bytes_definition,
            "\
            {bytes_name}\n{indentation}    \
            |> List.sublist({{ start: {byte_offset}, len: {array_size} }})\n{indentation}    \
            |> List.chunks_of({elem_size})\n{indentation}    \
            |> List.map_try(|bts| {from_bytes}(bts))?,\
            ",
            elem_size = elem_field_ty.serialized_size,
            array_size = elem_field_ty.serialized_size * len,
            from_bytes = elem_field_ty.from_bytes_func_name(),
        )?;
        *byte_offset += elem_field_ty.serialized_size * len;
        Ok(())
    };

    let mut byte_offset = 0;
    match fields {
        ir::TypeFields::None => {}
        ir::TypeFields::Named(fields) => {
            from_bytes_definition.push_str("{\n");
            for ir::NamedTypeField { ident, ty, .. } in fields {
                match ty {
                    ir::FieldType::Single { type_id } => {
                        write_single(
                            from_bytes_definition,
                            type_id,
                            ident,
                            true,
                            &mut byte_offset,
                        )?;
                    }
                    ir::FieldType::Array { elem_type_id, len } => {
                        write_array(
                            from_bytes_definition,
                            elem_type_id,
                            *len,
                            ident,
                            true,
                            &mut byte_offset,
                        )?;
                    }
                }
            }
            writeln!(from_bytes_definition, "{indentation}}},")?;
        }
        ir::TypeFields::Unnamed(fields) => {
            from_bytes_definition.push_str("(\n");
            for (field_idx, ir::UnnamedTypeField { ty }) in fields.iter().enumerate() {
                match ty {
                    ir::FieldType::Single { type_id } => {
                        write_single(
                            from_bytes_definition,
                            type_id,
                            &field_idx.to_string(),
                            false,
                            &mut byte_offset,
                        )?;
                    }
                    ir::FieldType::Array { elem_type_id, len } => {
                        write_array(
                            from_bytes_definition,
                            elem_type_id,
                            *len,
                            &field_idx.to_string(),
                            false,
                            &mut byte_offset,
                        )?;
                    }
                }
            }
            writeln!(from_bytes_definition, "{indentation}),")?;
        }
    }
    Ok(())
}

fn write_roundtrip_test(roc_code: &mut String, ty: &RegisteredType) -> Result<()> {
    match &ty.ty.composition {
        ir::TypeComposition::Primitive(_) => Ok(()),
        ir::TypeComposition::Struct { .. } => write_roundtrip_test_for_struct(roc_code, ty),
        ir::TypeComposition::Enum(_) => {
            // Creating valid roundtrip tests for enums is not trivial since
            // there could be illegal values at any byte position in the input
            // if the enum has enum payloads. We rely on fuzzing to test enums
            // instead.
            Ok(())
        }
    }
}

fn write_roundtrip_test_for_struct(roc_code: &mut String, ty: &RegisteredType) -> Result<()> {
    writeln!(
        roc_code,
        "\n\
        test_roundtrip : {{}} -> Result {{}} _\n\
        test_roundtrip = |{{}}|\n    \
            bytes = List.range({{ start: At 0, end: Length {} }}) |> List.map(|b| Num.to_u8(b))\n    \
            decoded = from_bytes(bytes)?\n    \
            encoded = write_bytes([], decoded)\n    \
            if List.len(bytes) == List.len(encoded) and List.map2(bytes, encoded, |a, b| a == b) |> List.all(|eq| eq) then\n        \
                Ok({{}})\n    \
            else\n        \
                Err(NotEqual(encoded, bytes))\n\
        \n\
        expect\n    \
            result = test_roundtrip({{}})\n    \
            result |> Result.is_ok\
        ",
        ty.serialized_size,
    )?;
    Ok(())
}
