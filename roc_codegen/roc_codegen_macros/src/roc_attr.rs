//! Attribute macro for Roc code generation.

use crate::{RocAttributeArgs, RocTypeCategory};
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::{fmt::Write, iter};
use syn::parse::Parser;

#[derive(Clone, Debug)]
struct ResolvedAttributeArgs {
    type_category: RocTypeCategory,
    package_name: String,
    module_name: String,
    type_name: String,
    function_postfix: String,
}

// These need to match the corresponding constants in `roc_codegen::meta`.
pub const MAX_ROC_TYPE_ENUM_VARIANTS: usize = 8;
pub const MAX_ROC_TYPE_ENUM_VARIANT_FIELDS: usize = 2;
pub const MAX_ROC_TYPE_STRUCT_FIELDS: usize =
    MAX_ROC_TYPE_ENUM_VARIANTS * MAX_ROC_TYPE_ENUM_VARIANT_FIELDS;

pub(super) fn apply_roc_attribute(
    args: Option<RocAttributeArgs>,
    input: syn::DeriveInput,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "the `roc` attribute does not support generic types",
        ));
    }

    let args = resolve_attribute_args(args, &input);

    let rust_type_name = &input.ident;

    let roc_impl = generate_roc_impl(rust_type_name, &input, crate_root, args.type_category)?;
    let roc_pod_impl = generate_roc_pod_impl(rust_type_name, crate_root, args.type_category);

    let mut static_assertions = Vec::new();
    let descriptor_submit = generate_roc_descriptor_submit(
        &args,
        rust_type_name,
        &input,
        crate_root,
        &mut static_assertions,
    )?;

    Ok(quote! {
        #input
        #roc_impl
        #roc_pod_impl
        #descriptor_submit
        #(#static_assertions)*
    })
}

fn resolve_attribute_args(
    args: Option<RocAttributeArgs>,
    input: &syn::DeriveInput,
) -> ResolvedAttributeArgs {
    match args {
        None => {
            let category = if derives_trait(input, "Pod") {
                RocTypeCategory::Pod
            } else {
                RocTypeCategory::Inline
            };
            let type_name = input.ident.to_string();
            let module_name = type_name.clone();
            let package_name = String::new();
            let function_postfix = String::new();
            ResolvedAttributeArgs {
                type_category: category,
                package_name,
                module_name,
                type_name,
                function_postfix,
            }
        }
        Some(RocAttributeArgs {
            category,
            package_name,
            module_name,
            type_name,
            function_postfix,
        }) => {
            let type_name = type_name.unwrap_or_else(|| input.ident.to_string());
            let module_name = module_name.unwrap_or_else(|| type_name.clone());
            let package_name = if category == RocTypeCategory::Primitive {
                package_name.unwrap_or_else(|| String::from("core"))
            } else {
                package_name.unwrap_or_default()
            };
            let function_postfix = function_postfix.unwrap_or_default();
            ResolvedAttributeArgs {
                type_category: category,
                package_name,
                module_name,
                type_name,
                function_postfix,
            }
        }
    }
}

fn derives_trait(input: &syn::DeriveInput, trait_name: &str) -> bool {
    for attribute in &input.attrs {
        if !attribute.path().is_ident("derive") {
            continue;
        }
        let Ok(derives) = attribute.meta.require_list() else {
            continue;
        };
        let Ok(paths) = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
            .parse2(derives.tokens.clone())
        else {
            continue;
        };
        for path in paths {
            let Some(last) = path.segments.last() else {
                continue;
            };
            if last.ident == trait_name {
                return true;
            }
        }
    }
    false
}

fn generate_roc_impl(
    rust_type_name: &Ident,
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: RocTypeCategory,
) -> syn::Result<TokenStream> {
    let roc_type_id = generate_roc_type_id(rust_type_name, crate_root);
    let size = generate_size_expr(input, crate_root, type_category)?;
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        impl #crate_root::meta::Roc for #rust_type_name {
            const ROC_TYPE_ID: #crate_root::meta::RocTypeID = #roc_type_id;
            const SERIALIZED_SIZE: usize = #size;
        }
    })
}

fn generate_roc_pod_impl(
    rust_type_name: &Ident,
    crate_root: &TokenStream,
    type_category: RocTypeCategory,
) -> TokenStream {
    if matches!(
        type_category,
        RocTypeCategory::Primitive | RocTypeCategory::Pod
    ) {
        // This impl ensures that we get an error if the type doesn't implement `Pod`
        quote! {
            #[cfg(feature = "roc_codegen")]
            impl #crate_root::meta::RocPod for #rust_type_name {}
        }
    } else {
        quote! {}
    }
}

fn generate_roc_type_id(rust_type_name: &Ident, crate_root: &TokenStream) -> TokenStream {
    // WARNING: If changing this, make sure to change the generation of
    // component IDs in `impact_ecs_macros` accordingly, since we guarantee
    // that the Roc type ID of any component matches the component ID
    let type_path_tail = format!("::{}", rust_type_name);
    quote!(
        #crate_root::meta::RocTypeID::hashed_from_str(concat!(
            module_path!(),
            #type_path_tail
        ))
    )
}

fn generate_roc_descriptor_submit(
    args: &ResolvedAttributeArgs,
    rust_type_name: &Ident,
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    let package_name = &args.package_name;
    let module_name = &args.module_name;
    let type_name = &args.type_name;
    let function_postfix = &args.function_postfix;
    let flags = generate_flags(crate_root, args.type_category);
    let composition =
        generate_composition(input, crate_root, args.type_category, static_assertions)?;
    let docstring = extract_and_process_docstring(&input.attrs);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::meta::RocTypeDescriptor {
                id: <#rust_type_name as #crate_root::meta::Roc>::ROC_TYPE_ID,
                package_name: #package_name,
                module_name: #module_name,
                type_name: #type_name,
                function_postfix: #function_postfix,
                serialized_size: <#rust_type_name as #crate_root::meta::Roc>::SERIALIZED_SIZE,
                flags: #flags,
                composition: #composition,
                docstring: #docstring,
            }
        }
    })
}

fn generate_flags(crate_root: &TokenStream, type_category: RocTypeCategory) -> TokenStream {
    match type_category {
        // All primitives are required to be POD
        RocTypeCategory::Primitive | RocTypeCategory::Pod => {
            quote! { #crate_root::meta::RocTypeFlags::IS_POD }
        }
        RocTypeCategory::Inline => {
            quote! { #crate_root::meta::RocTypeFlags::empty() }
        }
    }
}

fn generate_composition(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: RocTypeCategory,
    static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    if type_category == RocTypeCategory::Primitive {
        return Ok(quote! {
            #crate_root::meta::RocTypeComposition::Primitive(
                #crate_root::meta::RocPrimitiveKind::LibraryProvided{
                    precision: #crate_root::meta::RocPrimitivePrecision::PrecisionIrrelevant,
               }
            )
        });
    }
    match &input.data {
        syn::Data::Struct(data) => {
            let type_name = &input.ident;

            let fields =
                generate_fields(input, &data.fields, crate_root, MAX_ROC_TYPE_STRUCT_FIELDS)?;

            Ok(quote! {
                #crate_root::meta::RocTypeComposition::Struct{
                    alignment: ::std::mem::align_of::<#type_name>(),
                    fields: #fields
                }
            })
        }
        syn::Data::Enum(data) => {
            let variants = generate_variants(input, data, crate_root, false, static_assertions)?;
            Ok(quote! {
                #crate_root::meta::RocTypeComposition::Enum(#variants)
            })
        }
        syn::Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "the `roc` attribute does not support unions",
        )),
    }
}

fn generate_fields(
    span: impl ToTokens,
    fields: &syn::Fields,
    crate_root: &TokenStream,
    max_fields: usize,
) -> syn::Result<TokenStream> {
    Ok(match fields {
        syn::Fields::Unit => quote! {
            #crate_root::meta::RocTypeFields::None
        },
        syn::Fields::Named(fields) => {
            if fields.named.len() > max_fields {
                return Err(syn::Error::new_spanned(
                    span,
                    format!(
                        "the `roc` attribute does not support this many fields ({}/{})",
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
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() > max_fields {
                return Err(syn::Error::new_spanned(
                    span,
                    format!(
                        "the `roc` attribute does not support this many fields ({}/{})",
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
    fields: &syn::FieldsNamed,
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
            let ty = generate_field_type(field, crate_root);
            quote! {
                Some(#crate_root::meta::NamedRocTypeField {
                    docstring: #docstring,
                    ident: #ident,
                    ty: #ty,
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
    fields: &syn::FieldsUnnamed,
    crate_root: &TokenStream,
    max_fields: usize,
) -> TokenStream {
    assert!(max_fields >= fields.unnamed.len());

    let fields = fields
        .unnamed
        .iter()
        .map(|field| {
            let ty = generate_field_type(field, crate_root);
            quote! {
                Some(#crate_root::meta::UnnamedRocTypeField {
                    ty: #ty,
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

fn generate_field_type(field: &syn::Field, crate_root: &TokenStream) -> TokenStream {
    if let syn::Type::Array(array) = &field.ty {
        let elem_ty = &array.elem;
        let len = &array.len;
        quote! {
            #crate_root::meta::RocFieldType::Array {
                elem_type_id: <#elem_ty as #crate_root::meta::Roc>::ROC_TYPE_ID,
                len: #len,
            }
        }
    } else {
        let ty = &field.ty;
        quote! {
            #crate_root::meta::RocFieldType::Single {
                type_id: <#ty as #crate_root::meta::Roc>::ROC_TYPE_ID,
            }
        }
    }
}

fn generate_variants(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    crate_root: &TokenStream,
    require_pod: bool,
    static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            input,
            "the `roc` attribute does not support zero-sized enums",
        ));
    }

    if data.variants.len() > MAX_ROC_TYPE_ENUM_VARIANTS {
        return Err(syn::Error::new_spanned(
            input,
            format!(
                "the `roc` attribute does not support this many variants ({}/{})",
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
            let punct = if let syn::Fields::Named(_) = &variant.fields {
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
        .collect::<syn::Result<Vec<TokenStream>>>()?;

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

fn generate_size_expr(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: RocTypeCategory,
) -> syn::Result<TokenStream> {
    if type_category == RocTypeCategory::Primitive {
        // Since primitives are always POD, their serialized size
        // will always match their in-memory size
        let type_name = &input.ident;
        return Ok(quote! {
            ::std::mem::size_of::<#type_name>()
        });
    }
    match &input.data {
        syn::Data::Struct(data) => Ok(generate_struct_size_expr(data, crate_root)),
        syn::Data::Enum(data) => Ok(generate_enum_size_expr(data, crate_root)),
        syn::Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "the `roc` attribute does not support unions",
        )),
    }
}

fn generate_struct_size_expr(data: &syn::DataStruct, crate_root: &TokenStream) -> TokenStream {
    generate_summed_field_size_expr(&data.fields, crate_root)
}

fn generate_enum_size_expr(data: &syn::DataEnum, crate_root: &TokenStream) -> TokenStream {
    let mut variants = data.variants.iter();
    let Some(variant) = variants.next() else {
        return quote! {1}; // 1 byte for the discriminant
    };
    let mut max_variant_size = generate_summed_field_size_expr(&variant.fields, crate_root);
    for variant in variants {
        let variant_size = generate_summed_field_size_expr(&variant.fields, crate_root);
        max_variant_size = quote! {
            (if #variant_size > #max_variant_size { #variant_size } else { #max_variant_size })
        };
    }
    quote! {
        1 + #max_variant_size // 1 extra byte for the discriminant
    }
}

fn generate_summed_field_size_expr(fields: &syn::Fields, crate_root: &TokenStream) -> TokenStream {
    match fields {
        syn::Fields::Unit => {
            quote! {0}
        }
        syn::Fields::Named(fields) => {
            generate_summed_field_size_expr_from_field_iter(&fields.named, crate_root)
        }
        syn::Fields::Unnamed(fields) => {
            generate_summed_field_size_expr_from_field_iter(&fields.unnamed, crate_root)
        }
    }
}

fn generate_summed_field_size_expr_from_field_iter<'a>(
    fields: impl IntoIterator<Item = &'a syn::Field>,
    crate_root: &TokenStream,
) -> TokenStream {
    let mut fields = fields.into_iter();
    let Some(field) = fields.next() else {
        return quote! {0};
    };
    let ty = &field.ty;
    let mut summed_fields = serialized_size_of_type(ty, crate_root);
    for field in fields {
        let size = serialized_size_of_type(&field.ty, crate_root);
        summed_fields.extend(quote! {
            + #size
        });
    }
    summed_fields
}

fn serialized_size_of_type(ty: &syn::Type, crate_root: &TokenStream) -> TokenStream {
    if let syn::Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len = &array.len;
        quote! {
            (<#elem_ty as #crate_root::meta::Roc>::SERIALIZED_SIZE * #len)
        }
    } else {
        quote! {
            <#ty as #crate_root::meta::Roc>::SERIALIZED_SIZE
        }
    }
}

fn extract_and_process_docstring(attributes: &[syn::Attribute]) -> String {
    process_docstrings(extract_docstrings(attributes))
}

fn extract_docstrings(attributes: &[syn::Attribute]) -> impl Iterator<Item = String> {
    attributes.iter().filter_map(|attribute| {
        if !attribute.path().is_ident("doc") {
            return None;
        }
        let syn::Meta::NameValue(meta) = &attribute.meta else {
            return None;
        };
        let syn::Expr::Lit(expr_lit) = &meta.value else {
            return None;
        };
        let syn::Lit::Str(lit_str) = &expr_lit.lit else {
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
