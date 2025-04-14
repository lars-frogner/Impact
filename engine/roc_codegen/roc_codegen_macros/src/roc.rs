//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::{fmt::Write, iter};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, Field, Fields, FieldsNamed,
    FieldsUnnamed, Lit, Meta, Result,
};

// These need to match the corresponding constants in `roc_codegen::meta`.
pub const MAX_ROC_TYPE_ENUM_VARIANTS: usize = 8;
pub const MAX_ROC_TYPE_ENUM_VARIANT_FIELDS: usize = 2;
pub const MAX_ROC_TYPE_STRUCT_FIELDS: usize =
    MAX_ROC_TYPE_ENUM_VARIANTS * MAX_ROC_TYPE_ENUM_VARIANT_FIELDS;

#[cfg(feature = "enabled")]
pub(super) fn impl_roc(input: DeriveInput, crate_root: &TokenStream) -> Result<TokenStream> {
    let type_name = &input.ident;

    let roc_impl = generate_roc_impl(type_name, &input, crate_root)?;

    let mut static_assertions = Vec::new();
    let descriptor_submit = generate_roc_descriptor_submit(
        type_name,
        &input,
        crate_root,
        false,
        &mut static_assertions,
    )?;

    Ok(quote! {
        #roc_impl
        #descriptor_submit
        #(#static_assertions)*
    })
}

#[cfg(not(feature = "enabled"))]
pub(super) fn impl_roc(_input: DeriveInput, _crate_root: &TokenStream) -> Result<TokenStream> {
    Ok(quote! {})
}

#[cfg(feature = "enabled")]
pub(super) fn impl_roc_pod(input: DeriveInput, crate_root: &TokenStream) -> Result<TokenStream> {
    let type_name = &input.ident;

    let roc_impl = generate_roc_impl(type_name, &input, crate_root)?;
    let pod_roc_impl = generate_roc_pod_impl(type_name, crate_root)?;

    let mut static_assertions = Vec::new();
    let descriptor_submit = generate_roc_descriptor_submit(
        type_name,
        &input,
        crate_root,
        true,
        &mut static_assertions,
    )?;

    Ok(quote! {
        #roc_impl
        #pod_roc_impl
        #descriptor_submit
        #(#static_assertions)*
    })
}

#[cfg(not(feature = "enabled"))]
pub(super) fn impl_roc_pod(_input: DeriveInput, _crate_root: &TokenStream) -> Result<TokenStream> {
    Ok(quote! {})
}

fn generate_roc_impl(
    type_name: &Ident,
    input: &DeriveInput,
    crate_root: &TokenStream,
) -> Result<TokenStream> {
    let roc_type_id = generate_roc_type_id(type_name, crate_root);
    let size = generate_size_expr(input, crate_root)?;
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        impl #crate_root::meta::Roc for #type_name {
            const ROC_TYPE_ID: #crate_root::meta::RocTypeID = #roc_type_id;
            const SERIALIZED_SIZE: usize = #size;
        }
    })
}

fn generate_roc_pod_impl(type_name: &Ident, crate_root: &TokenStream) -> Result<TokenStream> {
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        impl #crate_root::meta::RocPod for #type_name {}
    })
}

fn generate_roc_type_id(type_name: &Ident, crate_root: &TokenStream) -> TokenStream {
    // WARNING: If changing this, make sure to change the generation of
    // component IDs in `impact_ecs_macros` accordingly, since we guarantee
    // that the Roc type ID of any component matches the component ID
    let type_path_tail = format!("::{}", type_name);
    quote!(
        #crate_root::meta::RocTypeID::hashed_from_str(concat!(
            module_path!(),
            #type_path_tail
        ))
    )
}

fn generate_roc_descriptor_submit(
    type_name: &Ident,
    input: &DeriveInput,
    crate_root: &TokenStream,
    require_pod: bool,
    static_assertions: &mut Vec<TokenStream>,
) -> Result<TokenStream> {
    let roc_name = type_name.to_string();
    let flags = generate_flags(crate_root, require_pod);
    let composition = generate_composition(input, crate_root, require_pod, static_assertions)?;
    let docstring = extract_and_process_docstring(&input.attrs);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::meta::RocTypeDescriptor {
                id: <#type_name as #crate_root::meta::Roc>::ROC_TYPE_ID,
                roc_name: #roc_name,
                serialized_size: <#type_name as #crate_root::meta::Roc>::SERIALIZED_SIZE,
                flags: #flags,
                composition: #composition,
                docstring: #docstring,
            }
        }
    })
}

fn generate_flags(crate_root: &TokenStream, require_pod: bool) -> TokenStream {
    if require_pod {
        quote! { #crate_root::meta::RocTypeFlags::IS_POD }
    } else {
        quote! { #crate_root::meta::RocTypeFlags::empty() }
    }
}

fn generate_composition(
    input: &DeriveInput,
    crate_root: &TokenStream,
    require_pod: bool,
    static_assertions: &mut Vec<TokenStream>,
) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(data) => {
            let type_name = &input.ident;

            let fields =
                generate_fields(input, &data.fields, crate_root, MAX_ROC_TYPE_STRUCT_FIELDS)?;

            if require_pod {
                static_assertions.push(quote! {
                    const _: () = {
                        const fn __assert_impl_pod<T: ::bytemuck::Pod>() {}
                        __assert_impl_pod::<#type_name>();
                    };
                });
            }

            Ok(quote! {
                #crate_root::meta::RocTypeComposition::Struct{
                    alignment: ::std::mem::align_of::<#type_name>(),
                    fields: #fields
                }
            })
        }
        Data::Enum(data) => {
            let variants =
                generate_variants(input, data, crate_root, require_pod, static_assertions)?;
            Ok(quote! {
                #crate_root::meta::RocTypeComposition::Enum(#variants)
            })
        }
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "the `Roc` trait can not be derived for unions",
        )),
    }
}

fn generate_fields(
    span: impl ToTokens,
    fields: &Fields,
    crate_root: &TokenStream,
    max_fields: usize,
) -> Result<TokenStream> {
    Ok(match fields {
        Fields::Unit => quote! {
            #crate_root::meta::RocTypeFields::None
        },
        Fields::Named(fields) => {
            if fields.named.len() > max_fields {
                return Err(Error::new_spanned(
                    span,
                    format!(
                        "too many fields to implement `Roc` ({}/{})",
                        fields.named.len(),
                        max_fields
                    ),
                ));
            }
            let fields = generate_named_field_list(fields, crate_root, max_fields);
            quote! {
                #crate_root::meta::RocTypeFields::Named(#fields)
            }
        }
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() > max_fields {
                return Err(Error::new_spanned(
                    span,
                    format!(
                        "too many fields to implement `Roc` ({}/{})",
                        fields.unnamed.len(),
                        max_fields
                    ),
                ));
            }
            let fields = generate_unnamed_field_list(fields, crate_root, max_fields);
            quote! {
                #crate_root::meta::RocTypeFields::Unnamed(#fields)
            }
        }
    })
}

fn generate_named_field_list(
    fields: &FieldsNamed,
    crate_root: &TokenStream,
    max_fields: usize,
) -> TokenStream {
    assert!(max_fields >= fields.named.len());

    let fields = fields
        .named
        .iter()
        .map(|field| {
            let docstring = extract_and_process_docstring(&field.attrs);
            let ident = field.ident.as_ref().unwrap().to_string();
            let ty = field.ty.to_token_stream();
            quote! {
                Some(#crate_root::meta::NamedRocTypeField {
                    docstring: #docstring,
                    ident: #ident,
                    type_id: <#ty as #crate_root::meta::Roc>::ROC_TYPE_ID,
                }),
            }
        })
        .chain(iter::repeat_n(
            quote! {None,},
            max_fields - fields.named.len(),
        ));

    quote! {
        #crate_root::meta::StaticList([#(#fields)*])
    }
}

fn generate_unnamed_field_list(
    fields: &FieldsUnnamed,
    crate_root: &TokenStream,
    max_fields: usize,
) -> TokenStream {
    assert!(max_fields >= fields.unnamed.len());

    let fields = fields
        .unnamed
        .iter()
        .map(|field| {
            let ty = field.ty.to_token_stream();
            quote! {
                Some(#crate_root::meta::UnnamedRocTypeField {
                    type_id: <#ty as #crate_root::meta::Roc>::ROC_TYPE_ID,
                }),
            }
        })
        .chain(iter::repeat_n(
            quote! {None,},
            max_fields - fields.unnamed.len(),
        ));

    quote! {
        #crate_root::meta::StaticList([#(#fields)*])
    }
}

fn generate_variants(
    input: &DeriveInput,
    data: &DataEnum,
    crate_root: &TokenStream,
    require_pod: bool,
    static_assertions: &mut Vec<TokenStream>,
) -> Result<TokenStream> {
    if data.variants.len() > MAX_ROC_TYPE_ENUM_VARIANTS {
        return Err(Error::new_spanned(
            input,
            format!(
                "too many variants to implement `Roc` ({}/{})",
                data.variants.len(),
                MAX_ROC_TYPE_ENUM_VARIANTS
            ),
        ));
    }

    let variant_checks_module_ident = format_ident!("__{}_variant_is_pod", &input.ident);
    let mut local_static_assertions = Vec::new();

    let variants = data
        .variants
        .iter()
        .map(|variant| {
            let attributes = if require_pod {
                quote! {
                    #[repr(C)]
                    #[derive(Clone, Copy, ::bytemuck::Zeroable, ::bytemuck::Pod)]
                }
            } else {
                quote! {}
            };
            let punct = if let Fields::Named(_) = &variant.fields {
                quote! {}
            } else {
                quote! {;}
            };
            local_static_assertions.push(quote! {
                #attributes
                pub(super) struct #variant #punct
            });

            let fields = generate_fields(
                variant,
                &variant.fields,
                crate_root,
                MAX_ROC_TYPE_ENUM_VARIANT_FIELDS,
            )?;

            let docstring = extract_and_process_docstring(&variant.attrs);
            let ident = &variant.ident;
            let ident_str = ident.to_string();

            Ok(quote! {
                Some(#crate_root::meta::RocTypeVariant {
                    docstring: #docstring,
                    ident: #ident_str,
                    size: ::std::mem::size_of::<#variant_checks_module_ident::#ident>(),
                    alignment: ::std::mem::align_of::<#variant_checks_module_ident::#ident>(),
                    fields: #fields,
                }),
            })
        })
        .chain(iter::repeat_n(
            Ok(quote! {None,}),
            MAX_ROC_TYPE_ENUM_VARIANTS - data.variants.len(),
        ))
        .collect::<Result<Vec<TokenStream>>>()?;

    static_assertions.push(quote! {
        #[allow(non_snake_case, dead_code, missing_debug_implementations)]
        pub mod #variant_checks_module_ident {
            use super::*;
            #(#local_static_assertions)*
        }
    });

    Ok(quote! {
        #crate_root::meta::RocTypeVariants(#crate_root::meta::StaticList([#(#variants)*]))
    })
}

fn generate_size_expr(input: &DeriveInput, crate_root: &TokenStream) -> Result<TokenStream> {
    match &input.data {
        Data::Struct(data) => Ok(generate_struct_size_expr(data, crate_root)),
        Data::Enum(data) => Ok(generate_enum_size_expr(data, crate_root)),
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "the `Roc` trait can not be derived for unions",
        )),
    }
}

fn generate_struct_size_expr(data: &DataStruct, crate_root: &TokenStream) -> TokenStream {
    generate_summed_field_size_expr(&data.fields, crate_root)
}

fn generate_enum_size_expr(data: &DataEnum, crate_root: &TokenStream) -> TokenStream {
    let mut variants = data.variants.iter();
    let Some(variant) = variants.next() else {
        return quote! {1};
    };
    let mut max_variant_size = generate_summed_field_size_expr(&variant.fields, crate_root);
    for variant in variants {
        let variant_size = generate_summed_field_size_expr(&variant.fields, crate_root);
        max_variant_size = quote! {
            (if #variant_size > #max_variant_size { #variant_size } else { #max_variant_size })
        };
    }
    quote! {
        1 + #max_variant_size
    }
}

fn generate_summed_field_size_expr(fields: &Fields, crate_root: &TokenStream) -> TokenStream {
    match fields {
        Fields::Unit => {
            quote! {0}
        }
        Fields::Named(fields) => {
            generate_summed_field_size_expr_from_field_iter(&fields.named, crate_root)
        }
        Fields::Unnamed(fields) => {
            generate_summed_field_size_expr_from_field_iter(&fields.unnamed, crate_root)
        }
    }
}

fn generate_summed_field_size_expr_from_field_iter<'a>(
    fields: impl IntoIterator<Item = &'a Field>,
    crate_root: &TokenStream,
) -> TokenStream {
    let mut fields = fields.into_iter();
    let Some(field) = fields.next() else {
        return quote! {0};
    };
    let ty = &field.ty;
    let mut summed_fields = quote! {
        <#ty as #crate_root::meta::Roc>::SERIALIZED_SIZE
    };
    for field in fields {
        let ty = &field.ty;
        summed_fields.extend(quote! {
            + <#ty as #crate_root::meta::Roc>::SERIALIZED_SIZE
        });
    }
    summed_fields
}

fn extract_and_process_docstring(attributes: &[Attribute]) -> String {
    process_docstrings(extract_docstrings(attributes))
}

fn extract_docstrings(attributes: &[Attribute]) -> impl Iterator<Item = String> {
    attributes.iter().filter_map(|attribute| {
        if !attribute.path().is_ident("doc") {
            return None;
        }
        let Meta::NameValue(meta) = &attribute.meta else {
            return None;
        };
        let Expr::Lit(expr_lit) = &meta.value else {
            return None;
        };
        let Lit::Str(lit_str) = &expr_lit.lit else {
            return None;
        };
        Some(lit_str.value())
    })
}

fn process_docstrings(lines: impl IntoIterator<Item = String>) -> String {
    let mut docstring = String::new();
    for line in lines {
        writeln!(&mut docstring, "##{}", line).unwrap();
    }
    docstring
}
