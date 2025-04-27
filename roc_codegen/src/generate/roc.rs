//! Generation of Roc code for working with types annotated with the
//! [`roc`](crate::roc) attribute.

use super::{RocGenerateOptions, field_type_descriptor};
use crate::meta::{
    NamedRocTypeField, RocConstructorDescriptor, RocFieldType, RocTypeComposition,
    RocTypeDescriptor, RocTypeFields, RocTypeFlags, RocTypeID, UnnamedRocTypeField,
};
use anyhow::{Result, anyhow};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
};

pub(super) fn generate_module(
    options: &RocGenerateOptions,
    type_descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    type_descriptor: &RocTypeDescriptor,
    constructor_descriptors: &[RocConstructorDescriptor],
) -> Result<Option<String>> {
    if let RocTypeComposition::Primitive(_) = type_descriptor.composition {
        return Ok(None);
    }

    let mut module = String::new();

    write_module_header(&mut module, constructor_descriptors, type_descriptor)?;
    module.push('\n');

    write_imports(options, &mut module, type_descriptors, type_descriptor)?;
    module.push('\n');

    write_type_declaration(&mut module, type_descriptors, type_descriptor)?;
    module.push('\n');

    write_constructors(
        &mut module,
        type_descriptors,
        type_descriptor,
        constructor_descriptors,
    )?;

    write_component_functions(&mut module, type_descriptor)?;

    write_write_bytes_function(&mut module, type_descriptors, type_descriptor)?;
    module.push('\n');

    write_from_bytes_function(&mut module, type_descriptors, type_descriptor)?;
    module.push('\n');

    write_roundtrip_test(&mut module, type_descriptor)?;

    Ok(Some(module))
}

fn write_module_header(
    roc_code: &mut String,
    constructor_descriptors: &[RocConstructorDescriptor],
    type_descriptor: &RocTypeDescriptor,
) -> Result<()> {
    write!(
        roc_code,
        "\
        module [\n    \
            {},\n\
        ",
        type_descriptor.type_name,
    )?;

    for descriptor in constructor_descriptors {
        writeln!(roc_code, "    {},", descriptor.function_name)?;
    }

    if type_descriptor.is_component() {
        for descriptor in constructor_descriptors {
            writeln!(roc_code, "    add_{},", descriptor.function_name)?;
        }

        roc_code.push_str(
            "    \
                add_to_entity,\n    \
                add_to_entities,\n\
            ]\n\
            ",
        );
    } else {
        roc_code.push_str(
            "    \
                write_bytes,\n    \
                from_bytes,\n\
            ]\n\
            ",
        );
    }

    Ok(())
}

fn write_imports(
    options: &RocGenerateOptions,
    roc_code: &mut String,
    type_descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    type_descriptor: &RocTypeDescriptor,
) -> Result<()> {
    let mut imports = Vec::from_iter(determine_imports(
        options,
        type_descriptors,
        type_descriptor,
    ));
    imports.sort();
    for import in imports {
        writeln!(roc_code, "import {import}")?;
    }
    Ok(())
}

fn determine_imports(
    options: &RocGenerateOptions,
    type_descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    type_descriptor: &RocTypeDescriptor,
) -> HashSet<String> {
    let mut imports = HashSet::new();

    // All modules needs this import
    imports.insert(format!(
        "{core}.Builtin as Builtin",
        core = &options.core_package_name
    ));

    if type_descriptor.is_component() {
        // ECS components need this import
        imports.insert(format!(
            "{pf}.Entity as Entity",
            pf = &options.platform_package_name
        ));
    }

    match &type_descriptor.composition {
        RocTypeComposition::Primitive(_) => {}
        RocTypeComposition::Struct { fields, .. } => {
            add_imports_for_fields(options, &mut imports, type_descriptors, fields);
        }
        RocTypeComposition::Enum(variants) => {
            for variant in &variants.0 {
                add_imports_for_fields(options, &mut imports, type_descriptors, &variant.fields);
            }
        }
    }
    imports
}

fn add_imports_for_fields<const N: usize>(
    options: &RocGenerateOptions,
    imports: &mut HashSet<String>,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    fields: &RocTypeFields<N>,
) {
    match fields {
        RocTypeFields::None => {}
        RocTypeFields::Named(fields) => {
            for NamedRocTypeField { ty, .. } in fields {
                let type_id = match ty {
                    RocFieldType::Single { type_id } => type_id,
                    RocFieldType::Array { elem_type_id, .. } => elem_type_id,
                };
                if let Some(field_descriptor) = descriptors.get(type_id) {
                    imports.insert(field_descriptor.import_module(
                        &options.import_prefix,
                        &options.core_package_name,
                        &options.platform_package_name,
                    ));
                }
            }
        }
        RocTypeFields::Unnamed(fields) => {
            for UnnamedRocTypeField { ty } in fields {
                let type_id = match ty {
                    RocFieldType::Single { type_id } => type_id,
                    RocFieldType::Array { elem_type_id, .. } => elem_type_id,
                };
                if let Some(field_descriptor) = descriptors.get(type_id) {
                    imports.insert(field_descriptor.import_module(
                        &options.import_prefix,
                        &options.core_package_name,
                        &options.platform_package_name,
                    ));
                }
            }
        }
    }
}

fn write_type_declaration(
    roc_code: &mut String,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    descriptor: &RocTypeDescriptor,
) -> Result<()> {
    if !descriptor.docstring.is_empty() {
        roc_code.push_str(descriptor.docstring);
    }
    match &descriptor.composition {
        // We don't generate code for primitive types
        RocTypeComposition::Primitive(_) => Ok(()),
        RocTypeComposition::Struct { fields, .. } => {
            write!(roc_code, "{} : ", descriptor.type_name)?;
            write_fields_declaration(roc_code, descriptors, fields, 0, false, &|| {
                format!("struct type {}", descriptor.type_name)
            })?;
            roc_code.push('\n');
            Ok(())
        }
        RocTypeComposition::Enum(variants) => {
            write!(roc_code, "{} : [", descriptor.type_name)?;
            let mut variant_count = 0;
            for variant in &variants.0 {
                if !variant.docstring.is_empty() {
                    for line in variant.docstring.lines() {
                        write!(roc_code, "\n    {line}")?;
                    }
                }
                write!(roc_code, "\n    {}", variant.ident)?;
                if !matches!(variant.fields, RocTypeFields::None) {
                    roc_code.push(' ');
                    write_fields_declaration(
                        roc_code,
                        descriptors,
                        &variant.fields,
                        2, // 1 looks more right, but 2 is consistent with Roc autoformatting
                        true,
                        &|| format!("variant {} of enum {}", variant.ident, descriptor.type_name),
                    )?;
                }
                roc_code.push(',');
                variant_count += 1;
            }
            if variant_count > 0 {
                roc_code.push('\n');
            }
            roc_code.push_str("]\n");
            Ok(())
        }
    }
}

fn write_fields_declaration<const N: usize>(
    declaration: &mut String,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    fields: &RocTypeFields<N>,
    indentation_level: usize,
    undelimited_tuple: bool,
    parent_name: &impl Fn() -> String,
) -> Result<()> {
    let indentation = "    ".repeat(indentation_level);
    match fields {
        RocTypeFields::None => {
            declaration.push_str("{}");
        }
        RocTypeFields::Named(fields) => {
            declaration.push('{');
            let mut field_count = 0;
            for NamedRocTypeField {
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

                let type_name = resolved_type_name_for_field(descriptors, ty, ident, parent_name)?;
                write!(declaration, "\n{indentation}    {ident} : {type_name},")?;

                field_count += 1;
            }
            if field_count > 0 {
                declaration.push('\n');
            }
            write!(declaration, "{indentation}}}")?;
        }

        RocTypeFields::Unnamed(fields) => {
            if !undelimited_tuple && fields.len() > 1 {
                declaration.push('(');
            }
            for (field_idx, UnnamedRocTypeField { ty }) in fields.iter().enumerate() {
                let type_name =
                    resolved_type_name_for_field(descriptors, ty, field_idx, parent_name)?;
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

fn write_write_bytes_function(
    roc_code: &mut String,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    descriptor: &RocTypeDescriptor,
) -> Result<()> {
    // We don't generate code for primitive types
    if matches!(descriptor.composition, RocTypeComposition::Primitive(_)) {
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
        name = descriptor.type_name,
        underscore = if descriptor.serialized_size == 0 {
            "_"
        } else {
            ""
        }
    )?;

    match &descriptor.composition {
        RocTypeComposition::Struct { fields, .. } => {
            roc_code.push_str("    bytes\n");
            if descriptor.serialized_size > 0 {
                writeln!(
                    roc_code,
                    "    |> List.reserve({size})",
                    size = descriptor.serialized_size,
                )?;
            }
            write_calls_to_write_bytes(
                roc_code,
                descriptors,
                fields,
                1,
                |def, field| write!(def, "value.{field}"),
                |def, idx| write!(def, "value.{idx}"),
                |def| {
                    def.push_str("value");
                },
                &|| format!("struct type {}", descriptor.type_name),
            )?;
        }
        RocTypeComposition::Enum(variants) => {
            writeln!(roc_code, "    when value is")?;
            for (variant_idx, variant) in variants.0.iter().enumerate() {
                if variant_idx > 0 {
                    roc_code.push('\n');
                }
                match &variant.fields {
                    RocTypeFields::None => {
                        writeln!(roc_code, "        {} ->", variant.ident)?;
                    }
                    RocTypeFields::Named(fields) => {
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
                    RocTypeFields::Unnamed(fields) => {
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
                    |> List.reserve({})\n            \
                    |> List.append({})\n\
                    ",
                    variant.size + 1,
                    variant_idx,
                )?;
                write_calls_to_write_bytes(
                    roc_code,
                    descriptors,
                    &variant.fields,
                    3,
                    |def, field| write!(def, "{field}"),
                    |def, idx| write!(def, "x{idx}"),
                    |def| {
                        def.push_str("val");
                    },
                    &|| format!("variant {} of enum {}", variant.ident, descriptor.type_name),
                )?;
            }
        }
        RocTypeComposition::Primitive(_) => {
            unreachable!()
        }
    }
    Ok(())
}

fn write_calls_to_write_bytes<const N: usize>(
    write_bytes_definition: &mut String,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    fields: &RocTypeFields<N>,
    indentation_level: usize,
    mut write_struct_value_access: impl FnMut(&mut String, &str) -> std::fmt::Result,
    mut write_tuple_value_access: impl FnMut(&mut String, &str) -> std::fmt::Result,
    mut write_whole_value_access: impl FnMut(&mut String),
    parent_name: &impl Fn() -> String,
) -> Result<()> {
    let indentation = "    ".repeat(indentation_level);

    let write_until_value_access = |write_bytes_definition: &mut String,
                                    ty: &RocFieldType,
                                    field: &str|
     -> Result<()> {
        match ty {
            RocFieldType::Single { type_id } => {
                let field_descriptor =
                    field_type_descriptor(descriptors, type_id, field, parent_name)?;
                write!(
                    write_bytes_definition,
                    "{indentation}|> {}(",
                    field_descriptor.write_bytes_func_name(),
                )?;
            }
            RocFieldType::Array { elem_type_id, .. } => {
                let elem_field_descriptor =
                    field_type_descriptor(descriptors, elem_type_id, field, parent_name)?;
                write!(
                    write_bytes_definition,
                    "\
                    {indentation}|> (|bts, values| values |> List.walk(bts, |b, val| b |> {}(val)))(\
                ",
                    elem_field_descriptor.write_bytes_func_name(),
                )?;
            }
        }
        Ok(())
    };

    match fields {
        RocTypeFields::None => {}
        RocTypeFields::Named(fields) => {
            for NamedRocTypeField { ident, ty, .. } in fields {
                write_until_value_access(write_bytes_definition, ty, ident)?;
                write_struct_value_access(write_bytes_definition, ident)?;
                writeln!(write_bytes_definition, ")")?;
            }
        }
        RocTypeFields::Unnamed(fields) => {
            let is_single = fields.len() == 1;
            for (field_idx, UnnamedRocTypeField { ty }) in fields.iter().enumerate() {
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
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    descriptor: &RocTypeDescriptor,
) -> Result<()> {
    if matches!(descriptor.composition, RocTypeComposition::Primitive(_)) {
        return Ok(());
    }

    write!(
        roc_code,
        "\
        ## Deserializes a value of [{name}] from its bytes in the\n\
        ## representation used by the engine.\n\
        from_bytes : List U8 -> Result {name} Builtin.DecodeErr\n\
        from_bytes = |{underscore}bytes|\n\
        ",
        name = descriptor.type_name,
        underscore = if descriptor.serialized_size == 0 {
            "_"
        } else {
            ""
        }
    )?;

    match &descriptor.composition {
        RocTypeComposition::Struct {
            fields: RocTypeFields::None,
            ..
        } => {
            roc_code.push_str("    Ok({})\n");
        }
        RocTypeComposition::Struct { fields, .. } => {
            roc_code.push_str("    Ok(\n        ");
            write_calls_to_from_bytes(roc_code, descriptors, fields, 2, "bytes", &|| {
                format!("struct type {}", descriptor.type_name)
            })?;
            writeln!(roc_code, "    )")?;
        }
        RocTypeComposition::Enum(variants) => {
            writeln!(roc_code, "    when bytes is")?;
            for (variant_idx, variant) in variants.0.iter().enumerate() {
                match &variant.fields {
                    RocTypeFields::None => {
                        writeln!(
                            roc_code,
                            "        \
                            [{variant_idx}] -> Ok({})\n        \
                            [{variant_idx}, ..] -> Err(InvalidNumberOfBytes)\
                            ",
                            variant.ident
                        )?;
                    }
                    RocTypeFields::Named(_) => {
                        write!(
                            roc_code,
                            "        \
                            [{variant_idx}, .. as data_bytes] ->\n            \
                                Ok(\n                \
                                    {} \
                            ",
                            variant.ident
                        )?;
                        write_calls_to_from_bytes(
                            roc_code,
                            descriptors,
                            &variant.fields,
                            4,
                            "data_bytes",
                            &|| {
                                format!(
                                    "variant {} of enum {}",
                                    variant.ident, descriptor.type_name
                                )
                            },
                        )?;
                        roc_code.push_str(
                            "            \
                                )\n\n\
                            ",
                        );
                        if variant_idx > 0 {
                            roc_code.push('\n');
                        }
                    }
                    RocTypeFields::Unnamed(_) => {
                        write!(
                            roc_code,
                            "        \
                            [{variant_idx}, .. as data_bytes] ->\n            \
                                Ok(\n                \
                                    {}\
                            ",
                            variant.ident
                        )?;
                        write_calls_to_from_bytes(
                            roc_code,
                            descriptors,
                            &variant.fields,
                            4,
                            "data_bytes",
                            &|| {
                                format!(
                                    "variant {} of enum {}",
                                    variant.ident, descriptor.type_name
                                )
                            },
                        )?;
                        roc_code.push_str(
                            "            \
                                )\n\n\
                            ",
                        );
                    }
                }
            }
            roc_code.push_str(
                "        \
                [] -> Err(MissingDiscriminant)\n        \
                _ -> Err(InvalidDiscriminant)\n\
                ",
            );
        }
        RocTypeComposition::Primitive(_) => {
            unreachable!()
        }
    }
    Ok(())
}

fn write_calls_to_from_bytes<const N: usize>(
    from_bytes_definition: &mut String,
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    fields: &RocTypeFields<N>,
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
        let field_descriptor = field_type_descriptor(descriptors, type_id, field, parent_name)?;
        write!(from_bytes_definition, "{indentation}    ")?;
        if write_ident {
            write!(from_bytes_definition, "{field}: ")?;
        }
        writeln!(
            from_bytes_definition,
            "{bytes_name} |> List.sublist({{ start: {byte_offset}, len: {size} }}) |> {from_bytes}?,",
            size = field_descriptor.serialized_size,
            from_bytes = field_descriptor.from_bytes_func_name(),
        )?;
        *byte_offset += field_descriptor.serialized_size;
        Ok(())
    };

    let write_array = |from_bytes_definition: &mut String,
                       elem_type_id: &RocTypeID,
                       len: usize,
                       field: &str,
                       write_ident: bool,
                       byte_offset: &mut usize|
     -> Result<()> {
        let elem_field_descriptor =
            field_type_descriptor(descriptors, elem_type_id, field, parent_name)?;
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
            elem_size = elem_field_descriptor.serialized_size,
            array_size = elem_field_descriptor.serialized_size * len,
            from_bytes = elem_field_descriptor.from_bytes_func_name(),
        )?;
        *byte_offset += elem_field_descriptor.serialized_size * len;
        Ok(())
    };

    let mut byte_offset = 0;
    match fields {
        RocTypeFields::None => {}
        RocTypeFields::Named(fields) => {
            from_bytes_definition.push_str("{\n");
            for NamedRocTypeField { ident, ty, .. } in fields {
                match ty {
                    RocFieldType::Single { type_id } => {
                        write_single(
                            from_bytes_definition,
                            type_id,
                            ident,
                            true,
                            &mut byte_offset,
                        )?;
                    }
                    RocFieldType::Array { elem_type_id, len } => {
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
        RocTypeFields::Unnamed(fields) => {
            from_bytes_definition.push_str("(\n");
            for (field_idx, UnnamedRocTypeField { ty }) in fields.iter().enumerate() {
                match ty {
                    RocFieldType::Single { type_id } => {
                        write_single(
                            from_bytes_definition,
                            type_id,
                            &field_idx.to_string(),
                            false,
                            &mut byte_offset,
                        )?;
                    }
                    RocFieldType::Array { elem_type_id, len } => {
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

fn write_constructors(
    roc_code: &mut String,
    type_descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    type_descriptor: &RocTypeDescriptor,
    constructor_descriptors: &[RocConstructorDescriptor],
) -> Result<()> {
    for constructor_descriptor in constructor_descriptors {
        write_constructor(
            roc_code,
            type_descriptors,
            type_descriptor,
            constructor_descriptor,
        )?;
        roc_code.push('\n');
    }
    Ok(())
}

pub(super) fn write_constructor(
    roc_code: &mut String,
    type_descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    type_descriptor: &RocTypeDescriptor,
    constructor_descriptor: &RocConstructorDescriptor,
) -> Result<()> {
    let docstring = if constructor_descriptor.docstring.is_empty() {
        ""
    } else {
        constructor_descriptor.docstring
    };

    let arg_types = constructor_descriptor
        .arguments
        .0
        .iter()
        .map(|arg| {
            type_descriptors
                .get(&arg.type_id)
                .ok_or_else(|| {
                    anyhow!(
                        "Missing type descriptor for argument {} to constructor {}",
                        arg.ident,
                        constructor_descriptor.function_name
                    )
                })
                .map(|desc| desc.resolved_type_name(false))
        })
        .collect::<Result<Vec<_>>>()?
        .join(", ");

    let non_empty_arg_types = if arg_types.is_empty() {
        "{}"
    } else {
        &arg_types
    };

    let arg_names = constructor_descriptor
        .arguments
        .0
        .iter()
        .map(|arg| arg.ident)
        .collect::<Vec<_>>()
        .join(", ");

    let non_empty_arg_names = if arg_names.is_empty() {
        "{}"
    } else {
        &arg_names
    };

    let body = constructor_descriptor.roc_body.replace("\n", "\n    ");

    writeln!(
        roc_code,
        "\
        {docstring}\
        {name} : {non_empty_arg_types} -> {type_name}\n\
        {name} = |{non_empty_arg_names}|\n    \
            {body}\
        ",
        name = constructor_descriptor.function_name,
        type_name = type_descriptor.type_name,
    )?;

    if type_descriptor.is_component() {
        writeln!(
            roc_code,
            "\n\
            {docstring}## Adds the component to the given entity's data.\n\
            add_{name} : Entity.Data{arg_types} -> Entity.Data\n\
            add_{name} = |data{arg_names}|\n    \
                add_to_entity(data, {name}({non_empty_arg_names}))\
            ",
            arg_types = if arg_types.is_empty() {
                String::new()
            } else {
                format!(", {arg_types}")
            },
            arg_names = if arg_names.is_empty() {
                String::new()
            } else {
                format!(", {arg_names}")
            },
            name = constructor_descriptor.function_name,
        )?;
    }

    Ok(())
}

fn write_component_functions(roc_code: &mut String, descriptor: &RocTypeDescriptor) -> Result<()> {
    if !descriptor.flags.contains(RocTypeFlags::IS_COMPONENT) {
        return Ok(());
    }

    let alignment = descriptor.alignment_as_pod_struct().ok_or_else(|| {
        anyhow!(
            "\
            Component type {} is not registered as POD: \
            make sure to derive the `RocPod` trait rather \
            than the `Roc` trait for component types\
            ",
            descriptor.type_name,
        )
    })?;

    writeln!(
        roc_code,
        "\
        ## Adds a value of the [{name}] component to an entity's data.\n\
        ## Note that an entity never should have more than a single value of\n\
        ## the same component type.\n\
        add_to_entity : Entity.Data, {name} -> Entity.Data\n\
        add_to_entity = |data, value|\n    \
            data |> Entity.append_component(write_packet, value)\n\
        \n\
        ## Adds multiple values of the [{name}] component to the data of\n\
        ## a set of entities of the same archetype's data.\n\
        ## Note that the number of values should match the number of entities\n\
        ## in the set and that an entity never should have more than a single\n\
        ## value of the same component type.\n\
        add_to_entities : Entity.MultiData, List {name} -> Entity.MultiData\n\
        add_to_entities = |data, values|\n    \
            data |> Entity.append_components(write_multi_packet, values)\n\
        \n\
        write_packet : List U8, {name} -> List U8\n\
        write_packet = |bytes, value|\n    \
            type_id = {type_id}\n    \
            size = {size}\n    \
            alignment = {alignment}\n    \
            bytes\n    \
            |> List.reserve(24 + size)\n    \
            |> Builtin.write_bytes_u64(type_id)\n    \
            |> Builtin.write_bytes_u64(size)\n    \
            |> Builtin.write_bytes_u64(alignment)\n    \
            |> write_bytes(value)\n\
        \n\
        write_multi_packet : List U8, List {name} -> List U8\n\
        write_multi_packet = |bytes, values|\n    \
            type_id = {type_id}\n    \
            size = {size}\n    \
            alignment = {alignment}\n    \
            count = List.len(values)\n    \
            bytes_with_header =\n        \
                bytes\n        \
                |> List.reserve(32 + size * count)\n        \
                |> Builtin.write_bytes_u64(type_id)\n        \
                |> Builtin.write_bytes_u64(size)\n        \
                |> Builtin.write_bytes_u64(alignment)\n        \
                |> Builtin.write_bytes_u64(count)\n    \
            values\n    \
            |> List.walk(\n        \
                bytes_with_header,\n        \
                |bts, value| bts |> write_bytes(value),\n    \
            )\n\
        ",
        type_id = descriptor.id.as_u64(),
        size = descriptor.serialized_size,
        name = descriptor.type_name,
    )?;

    Ok(())
}

fn write_roundtrip_test(roc_code: &mut String, descriptor: &RocTypeDescriptor) -> Result<()> {
    writeln!(
        roc_code,
        "\
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
        descriptor.serialized_size,
    )?;
    Ok(())
}

fn resolved_type_name_for_field(
    descriptors: &HashMap<RocTypeID, RocTypeDescriptor>,
    ty: &RocFieldType,
    field: impl Display,
    parent_name: &impl Fn() -> String,
) -> Result<Cow<'static, str>> {
    Ok(match ty {
        RocFieldType::Single { type_id } => {
            field_type_descriptor(descriptors, type_id, field, parent_name)?
                .resolved_type_name(false)
        }
        RocFieldType::Array { elem_type_id, .. } => {
            let mut type_name = String::from("List ");
            let elem_type_name =
                field_type_descriptor(descriptors, elem_type_id, field, parent_name)?
                    .resolved_type_name(true);
            write!(&mut type_name, "{}", elem_type_name)?;
            Cow::Owned(type_name)
        }
    })
}
