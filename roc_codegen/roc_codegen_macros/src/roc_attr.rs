//! Attribute macro for Roc code generation.

use crate::{
    MAX_ROC_DEPENDENCIES, MAX_ROC_FUNCTION_ARGS, MAX_ROC_TYPE_ENUM_VARIANT_FIELDS,
    MAX_ROC_TYPE_ENUM_VARIANTS, MAX_ROC_TYPE_STRUCT_FIELDS, RocImplAttributeArgs,
    RocTypeAttributeArgs, RocTypeCategory,
};
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

pub(super) fn apply_roc_type_attribute(
    args: Option<RocTypeAttributeArgs>,
    input: syn::DeriveInput,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "the `roc` attribute does not support generic types",
        ));
    }

    let args = resolve_type_attribute_args(args, &input);

    let rust_type_name = &input.ident;

    let roc_impl = generate_roc_impl(rust_type_name, &input, crate_root, args.type_category)?;
    let roc_pod_impl = generate_roc_pod_impl(rust_type_name, crate_root, args.type_category);

    let mut static_assertions = Vec::new();
    let type_submit = generate_roc_type_submit(
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
        #type_submit
        #(#static_assertions)*
    })
}

pub(super) fn apply_roc_impl_attribute(
    args: Option<RocImplAttributeArgs>,
    block: syn::ItemImpl,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    if !block.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &block.generics,
            "the `roc` attribute does not support generic impl blocks",
        ));
    }
    if let Some((_, trait_, _)) = block.trait_.as_ref() {
        return Err(syn::Error::new_spanned(
            trait_,
            "the `roc` attribute does not support trait impl blocks",
        ));
    }

    let for_type = &block.self_ty;

    let mut method_submits = Vec::with_capacity(block.items.len());
    for item in &block.items {
        if let syn::ImplItem::Fn(func) = item {
            let sequence_number = method_submits.len();

            let Some(roc_body) = extract_roc_body(func) else {
                continue;
            };

            method_submits.push(generate_roc_method_submit(
                sequence_number,
                for_type,
                func,
                crate_root,
                roc_body,
            )?);
        }
    }

    let dependencies_submit = if let Some(args) = args {
        generate_roc_dependencies_submit(for_type, &args.dependency_types, crate_root)?
    } else {
        quote! {}
    };

    Ok(quote! {
        #block
        #(#method_submits)*
        #dependencies_submit
    })
}

pub(super) fn apply_roc_body_attribute(
    body: syn::Expr,
    func: syn::ImplItemFn,
) -> syn::Result<TokenStream> {
    if !matches!(
        body,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    ) {
        return Err(syn::Error::new_spanned(body, "expected a string literal"));
    }

    Ok(quote! {
        #func
    })
}

fn resolve_type_attribute_args(
    args: Option<RocTypeAttributeArgs>,
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
        Some(RocTypeAttributeArgs {
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

fn generate_roc_type_submit(
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
    let flags = generate_type_flags(crate_root, args.type_category);
    let composition =
        generate_type_composition(input, crate_root, args.type_category, static_assertions)?;
    let docstring = extract_and_process_docstring(&input.attrs);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::meta::RocType {
                id: <#rust_type_name as #crate_root::meta::Roc>::ROC_TYPE_ID,
                package_name: #package_name,
                module_name: #module_name,
                name: #type_name,
                function_postfix: #function_postfix,
                serialized_size: <#rust_type_name as #crate_root::meta::Roc>::SERIALIZED_SIZE,
                flags: #flags,
                composition: #composition,
                docstring: #docstring,
            }
        }
    })
}

fn generate_roc_method_submit(
    sequence_number: usize,
    for_type: &syn::Type,
    func: &syn::ImplItemFn,
    crate_root: &TokenStream,
    roc_body: String,
) -> syn::Result<TokenStream> {
    let name = func.sig.ident.to_string();
    let arguments = generate_function_arguments::<MAX_ROC_FUNCTION_ARGS>(&func.sig, crate_root)?;
    let return_type = generate_method_return_type(&func.sig.output, crate_root)?;
    let docstring = extract_and_process_docstring(&func.attrs);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::meta::RocMethod {
                sequence_number: #sequence_number,
                for_type_id: <#for_type as #crate_root::meta::Roc>::ROC_TYPE_ID,
                name: #name,
                arguments: #arguments,
                return_type: #return_type,
                roc_body: #roc_body,
                docstring: #docstring,
            }
        }
    })
}

fn generate_roc_dependencies_submit(
    for_type: &syn::Type,
    dependency_types: &[syn::Type],
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    let dependencies = generate_type_id_list(dependency_types, crate_root, MAX_ROC_DEPENDENCIES);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::meta::RocDependencies {
                for_type_id: <#for_type as #crate_root::meta::Roc>::ROC_TYPE_ID,
                dependencies: #dependencies,
            }
        }
    })
}

fn generate_type_flags(crate_root: &TokenStream, type_category: RocTypeCategory) -> TokenStream {
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

fn generate_type_composition(
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
            {
                let syn::Variant { ident, fields, .. } = variant;

                let attributes = if require_pod {
                    quote! {
                        #[repr(C)]
                        #[derive(Clone, Copy, ::bytemuck::Zeroable, ::bytemuck::Pod)]
                    }
                } else {
                    quote! {}
                };
                let punct = if let syn::Fields::Named(_) = fields {
                    quote! {}
                } else {
                    quote! {;}
                };
                local_static_assertions.push(quote! {
                    #attributes
                    pub(super) struct #ident #fields #punct
                });
            }

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

fn generate_function_arguments<const MAX_ARGS: usize>(
    sig: &syn::Signature,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "the `roc` attribute does not support generic functions",
        ));
    }
    if sig.inputs.len() > MAX_ARGS {
        return Err(syn::Error::new_spanned(
            &sig.inputs,
            format!(
                "the `roc` attribute does not support this many arguments ({}/{})",
                sig.inputs.len(),
                MAX_ARGS
            ),
        ));
    }

    let args = sig
        .inputs
        .iter()
        .map(|input| match input {
            syn::FnArg::Receiver(syn::Receiver {
                reference,
                mutability,
                colon_token,
                ..
            }) => {
                if colon_token.is_some() {
                    return Err(syn::Error::new_spanned(
                        colon_token,
                        "the `roc` attribute does not support receivers with explicit types",
                    ));
                }
                if mutability.is_some() {
                    return Err(syn::Error::new_spanned(
                        mutability,
                        "the `roc` attribute does not support mutable methods",
                    ));
                }
                let receiver = if reference.is_some() {
                    quote! { #crate_root::meta::RocMethodReceiver::RefSelf }
                } else {
                    quote! { #crate_root::meta::RocMethodReceiver::OwnedSelf }
                };
                Ok(quote! {
                    Some(#crate_root::meta::RocFunctionArgument::Receiver(#receiver)),
                })
            }
            syn::FnArg::Typed(arg) => {
                let ident_str = match arg.pat.as_ref() {
                    syn::Pat::Ident(ident) => ident.ident.to_string(),
                    pat => {
                        return Err(syn::Error::new_spanned(
                            pat,
                            "the `roc` attribute does not support this argument pattern",
                        ));
                    }
                };
                let ty = generate_function_signature_type(arg.ty.clone(), crate_root)?;
                Ok(quote! {
                    Some(#crate_root::meta::RocFunctionArgument::Typed(
                        #crate_root::meta::TypedRocFunctionArgument {
                            ident: #ident_str,
                            ty: #ty,
                        }
                    )),
                })
            }
        })
        .chain(iter::repeat_n(
            Ok(quote! {None,}),
            MAX_ARGS - sig.inputs.len(),
        ))
        .collect::<syn::Result<Vec<TokenStream>>>()?;

    Ok(quote! {
        #crate_root::meta::RocFunctionArguments(#crate_root::meta::StaticList([#(#args)*]))
    })
}

fn generate_method_return_type(
    return_type: &syn::ReturnType,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    match return_type {
        syn::ReturnType::Type(_, ty) => match ty.as_ref() {
            syn::Type::Path(type_path)
                if type_path.qself.is_none()
                    && type_path.path.segments.len() == 1
                    && type_path.path.segments[0].ident == "Self" =>
            {
                Ok(quote! {
                    #crate_root::meta::RocMethodReturnType::SelfType
                })
            }
            _ => {
                let ty = generate_function_signature_type(ty.clone(), crate_root)?;
                Ok(quote! {
                    #crate_root::meta::RocMethodReturnType::Specific(#ty)
                })
            }
        },
        syn::ReturnType::Default => Err(syn::Error::new_spanned(
            return_type,
            "the `roc` attribute does not support functions returning nothing",
        )),
    }
}

fn generate_function_signature_type(
    mut arg_ty: Box<syn::Type>,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    arg_ty = unwrap_function_signature_references(arg_ty)?;
    match arg_ty.as_ref() {
        syn::Type::Array(syn::TypeArray { elem, .. })
        | syn::Type::Slice(syn::TypeSlice { elem, .. }) => {
            let elem_ty = unwrap_function_signature_references(elem.clone())?;
            let ty = generate_maybe_unregistered_type(&elem_ty, crate_root);
            Ok(quote! {
                #crate_root::meta::RocFunctionSignatureType::List(#ty)
            })
        }
        arg_ty => {
            let ty = generate_maybe_unregistered_type(arg_ty, crate_root);
            Ok(quote! {
                #crate_root::meta::RocFunctionSignatureType::Single(#ty)
            })
        }
    }
}

fn unwrap_function_signature_references(mut ty: Box<syn::Type>) -> syn::Result<Box<syn::Type>> {
    while let syn::Type::Reference(syn::TypeReference {
        elem, mutability, ..
    }) = *ty
    {
        if mutability.is_some() {
            return Err(syn::Error::new_spanned(
                elem,
                "the `roc` attribute does not support function signatures with mutable references",
            ));
        }
        ty = elem;
    }
    Ok(ty)
}

fn generate_maybe_unregistered_type(ty: &syn::Type, crate_root: &TokenStream) -> TokenStream {
    if type_is_string(ty) {
        quote! {
            #crate_root::meta::MaybeUnregisteredRocType::String
        }
    } else {
        quote! {
            #crate_root::meta::MaybeUnregisteredRocType::Registered(
                <#ty as #crate_root::meta::Roc>::ROC_TYPE_ID
            )
        }
    }
}

fn type_is_string(ty: &syn::Type) -> bool {
    matches!(
        ty,
        syn::Type::Path(syn::TypePath { path, .. })
            if path.segments.last().is_some_and(|segment| {
                matches!(segment.arguments, syn::PathArguments::None)
                    && (
                        segment.ident == "String"
                        || (path.segments.len() == 1 && segment.ident == "str")
                    )
            })
    )
}

fn generate_type_id_list(
    types: &[syn::Type],
    crate_root: &TokenStream,
    max_types: usize,
) -> TokenStream {
    assert!(max_types >= types.len());

    let ids = types
        .iter()
        .map(|ty| {
            quote! {
                Some(<#ty as #crate_root::meta::Roc>::ROC_TYPE_ID),
            }
        })
        .chain(iter::repeat_n(quote! {None,}, max_types - types.len()));

    quote! {
        #crate_root::meta::StaticList([#(#ids)*])
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

fn extract_roc_body(func: &syn::ImplItemFn) -> Option<String> {
    for attribute in &func.attrs {
        if let syn::Meta::List(syn::MetaList { path, tokens, .. }) = &attribute.meta {
            let Some(last) = path.segments.last() else {
                continue;
            };
            if last.ident != "roc_body" {
                continue;
            }
            return extract_roc_body_string(tokens.clone()).ok();
        }
    }
    None
}

fn extract_roc_body_string(attr: TokenStream) -> syn::Result<String> {
    let expr: syn::Expr = syn::parse2(attr)?;

    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit_str),
        ..
    }) = expr
    {
        Ok(lit_str.value())
    } else {
        Err(syn::Error::new_spanned(expr, "expected a string literal"))
    }
}
