//! Attribute macro for Roc code generation.

use crate::{
    AssociatedConstantAttributeArgs, AssociatedFunctionAttributeArgs, ImplAttributeArgs,
    MAX_DEPENDENCIES, MAX_ENUM_VARIANT_FIELDS, MAX_ENUM_VARIANTS, MAX_FUNCTION_ARGS,
    MAX_STRUCT_FIELDS, TypeAttributeArgs, TypeCategory,
};
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use std::{fmt::Write, iter};
use syn::parse::Parser;

#[derive(Clone, Debug)]
struct ResolvedAttributeArgs {
    type_category: TypeCategory,
    package_name: Option<String>,
    parent_modules: Option<String>,
    module_name: String,
    type_name: String,
    function_postfix: Option<String>,
}

pub(super) fn apply_type_attribute(
    args: TypeAttributeArgs,
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
    let type_submit = generate_registered_type_submit(
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

#[cfg(feature = "enabled")]
pub(super) fn apply_impl_attribute(
    args: ImplAttributeArgs,
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

    let associated_dependencies_submit = if !args.dependency_types.is_empty() {
        generate_associated_dependencies_submit(for_type, &args.dependency_types, crate_root)?
    } else {
        quote! {}
    };

    let mut associated_constant_submits = Vec::with_capacity(block.items.len());
    let mut associated_function_submits = Vec::with_capacity(block.items.len());

    for item in &block.items {
        if let syn::ImplItem::Const(constant) = item {
            let sequence_number = associated_constant_submits.len();

            let Some(AssociatedConstantAttributeArgs { expr, name }) =
                extract_associated_constant_attribute_args(constant)
            else {
                continue;
            };

            associated_constant_submits.push(generate_associated_constant_submit(
                for_type,
                sequence_number,
                name,
                constant,
                expr,
                crate_root,
            )?);
        } else if let syn::ImplItem::Fn(function) = item {
            let sequence_number = associated_function_submits.len();

            let Some(AssociatedFunctionAttributeArgs { body, name }) =
                extract_associated_function_attribute_args(function)
            else {
                continue;
            };

            associated_function_submits.push(generate_associated_function_submit(
                for_type,
                sequence_number,
                name,
                function,
                body,
                crate_root,
            )?);
        }
    }

    Ok(quote! {
        #block
        #associated_dependencies_submit
        #(#associated_constant_submits)*
        #(#associated_function_submits)*
    })
}

#[cfg(not(feature = "enabled"))]
pub(super) fn apply_impl_attribute(
    _args: ImplAttributeArgs,
    block: syn::ItemImpl,
    _crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    Ok(quote! {
        #block
    })
}

pub(super) fn apply_associated_constant_attribute(
    _args: AssociatedConstantAttributeArgs,
    constant: syn::ImplItemConst,
) -> syn::Result<TokenStream> {
    // When the `roc` macro is applied to an associated constant, it doesn't
    // actually do anything other than validating the macro arguments. It is
    // the `roc` macro applied to the surrounding `impl` block that actually
    // uses the arguments for code generation.
    Ok(quote! {
        #constant
    })
}

pub(super) fn apply_associated_function_attribute(
    _args: AssociatedFunctionAttributeArgs,
    func: syn::ImplItemFn,
) -> syn::Result<TokenStream> {
    // When the `roc` macro is applied to an associated function, it doesn't
    // actually do anything other than validating the macro arguments. It is
    // the `roc` macro applied to the surrounding `impl` block that actually
    // uses the arguments for code generation.
    Ok(quote! {
        #func
    })
}

fn resolve_type_attribute_args(
    TypeAttributeArgs {
        category,
        package_name,
        parent_modules,
        module_name,
        type_name,
        function_postfix,
    }: TypeAttributeArgs,
    input: &syn::DeriveInput,
) -> ResolvedAttributeArgs {
    let category = category.unwrap_or_else(|| {
        if derives_trait(input, "Pod") {
            TypeCategory::Pod
        } else {
            TypeCategory::Inline
        }
    });
    let type_name = type_name.unwrap_or_else(|| input.ident.to_string());
    let module_name = module_name.unwrap_or_else(|| type_name.clone());
    ResolvedAttributeArgs {
        type_category: category,
        package_name,
        parent_modules,
        module_name,
        type_name,
        function_postfix,
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
    type_category: TypeCategory,
) -> syn::Result<TokenStream> {
    let roc_type_id = generate_roc_type_id(rust_type_name, crate_root);
    let size = generate_size_expr(input, crate_root, type_category)?;
    let trait_method_impls = generate_roc_trait_method_impls(input, crate_root, type_category)?;
    Ok(quote! {
        impl #crate_root::Roc for #rust_type_name {
            const ROC_TYPE_ID: #crate_root::RocTypeID = #roc_type_id;
            const SERIALIZED_SIZE: usize = #size;

            #trait_method_impls
        }
    })
}

fn generate_roc_pod_impl(
    rust_type_name: &Ident,
    crate_root: &TokenStream,
    type_category: TypeCategory,
) -> TokenStream {
    if matches!(type_category, TypeCategory::Primitive | TypeCategory::Pod) {
        // This impl ensures that we get an error if the type doesn't implement `Pod`
        quote! {
            impl #crate_root::RocPod for #rust_type_name {}
        }
    } else {
        quote! {}
    }
}

fn generate_roc_type_id(rust_type_name: &Ident, crate_root: &TokenStream) -> TokenStream {
    // WARNING: If changing this, make sure to change the generation of
    // component IDs in `impact_ecs_macros` accordingly, since we guarantee
    // that the Roc type ID of any component matches the component ID
    let type_path_str = generate_qualified_type_path_str(rust_type_name);
    quote!(
        #crate_root::RocTypeID::hashed_from_str(#type_path_str)
    )
}

fn generate_qualified_type_path_str(rust_type_name: &Ident) -> TokenStream {
    let type_path_tail = format!("::{}", rust_type_name);
    quote! {
        concat!(module_path!(), #type_path_tail)
    }
}

fn generate_size_expr(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: TypeCategory,
) -> syn::Result<TokenStream> {
    if matches!(type_category, TypeCategory::Primitive | TypeCategory::Pod) {
        // Their serialized size of POD types will always match their
        // in-memory size
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
    generate_summed_field_size_expr(&data.fields, crate_root).unwrap_or_else(|| quote! {0})
}

fn generate_enum_size_expr(data: &syn::DataEnum, crate_root: &TokenStream) -> TokenStream {
    const _: () = assert!(
        MAX_ENUM_VARIANTS <= 256,
        "Enum discriminant is assumed to fit in one byte"
    );

    let variant_sizes: Vec<_> = data
        .variants
        .iter()
        .filter_map(|variant| generate_summed_field_size_expr(&variant.fields, crate_root))
        .collect();

    if variant_sizes.is_empty() {
        return quote! {1}; // 1 byte for the discriminant
    }

    let n_variant_sizes = variant_sizes.len();

    quote! {
        {
            const SIZES: [usize; #n_variant_sizes] = [#(#variant_sizes),*];
            let mut max = SIZES[0];
            let mut i = 1;
            while i < #n_variant_sizes {
                if SIZES[i] > max {
                    max = SIZES[i];
                }
                i += 1;
            }
            max + 1 // 1 extra byte for the discriminant
        }
    }
}

fn generate_summed_field_size_expr(
    fields: &syn::Fields,
    crate_root: &TokenStream,
) -> Option<TokenStream> {
    match fields {
        syn::Fields::Unit => None,
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
) -> Option<TokenStream> {
    let mut fields = fields.into_iter();
    let field = fields.next()?;
    let ty = &field.ty;
    let mut summed_fields = serialized_size_of_type(ty, crate_root);
    for field in fields {
        let size = serialized_size_of_type(&field.ty, crate_root);
        summed_fields.extend(quote! {
            + #size
        });
    }
    Some(summed_fields)
}

fn serialized_size_of_type(ty: &syn::Type, crate_root: &TokenStream) -> TokenStream {
    if let syn::Type::Array(array) = ty {
        let elem_ty = &array.elem;
        let len = &array.len;
        quote! {
            (<#elem_ty as #crate_root::Roc>::SERIALIZED_SIZE * #len)
        }
    } else {
        quote! {
            <#ty as #crate_root::Roc>::SERIALIZED_SIZE
        }
    }
}

fn generate_roc_trait_method_impls(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: TypeCategory,
) -> syn::Result<TokenStream> {
    let type_name = &input.ident;
    if let TypeCategory::Primitive | TypeCategory::Pod = type_category {
        Ok(generate_roc_trait_method_impls_for_pod_type(
            type_name, crate_root,
        ))
    } else {
        generate_roc_trait_method_impls_for_non_pod_type(input, crate_root)
    }
}

fn generate_roc_trait_method_impls_for_pod_type(
    type_name: &syn::Ident,
    crate_root: &TokenStream,
) -> TokenStream {
    let from_bytes_input_check = generate_from_bytes_input_check(type_name, crate_root);
    let write_bytes_input_check = generate_write_bytes_input_check(type_name, crate_root);
    quote! {
        fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
            #from_bytes_input_check
            Ok(::bytemuck::pod_read_unaligned(bytes))
        }

        fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
            #write_bytes_input_check
            buffer.copy_from_slice(::bytemuck::bytes_of(self));
            Ok(())
        }
    }
}

fn generate_roc_trait_method_impls_for_non_pod_type(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    let type_name = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => Ok(generate_roc_trait_method_impls_for_struct(
            type_name, data, crate_root,
        )),
        syn::Data::Enum(data) => Ok(generate_roc_trait_method_impls_for_enum(
            type_name, data, crate_root,
        )),
        syn::Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "the `roc` attribute does not support unions",
        )),
    }
}

fn generate_roc_trait_method_impls_for_struct(
    type_name: &syn::Ident,
    data: &syn::DataStruct,
    crate_root: &TokenStream,
) -> TokenStream {
    let from_bytes_input_check = generate_from_bytes_input_check(type_name, crate_root);
    let write_bytes_input_check = generate_write_bytes_input_check(type_name, crate_root);

    match &data.fields {
        syn::Fields::Unit => quote! {
            fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
                #from_bytes_input_check
                Ok(Self)
            }

            fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
                #write_bytes_input_check
                Ok(())
            }
        },
        syn::Fields::Named(fields) => {
            let destructuring = generate_destructuring_for_named_fields(fields);

            let from_bytes_calls = generate_from_bytes_calls_for_fields(&fields.named, crate_root);

            let write_bytes_calls =
                generate_write_bytes_calls_for_fields(&fields.named, crate_root);

            quote! {
                fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
                    #from_bytes_input_check
                    let mut cursor = 0;
                    #from_bytes_calls
                    Ok(Self #destructuring)
                }

                fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
                    #write_bytes_input_check
                    let Self #destructuring = self;
                    let mut cursor = 0;
                    #write_bytes_calls
                    Ok(())
                }
            }
        }
        syn::Fields::Unnamed(fields) => {
            let destructuring = generate_destructuring_for_unnamed_fields(fields);

            let from_bytes_calls =
                generate_from_bytes_calls_for_fields(&fields.unnamed, crate_root);

            let write_bytes_calls =
                generate_write_bytes_calls_for_fields(&fields.unnamed, crate_root);

            quote! {
                fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
                    #from_bytes_input_check
                    let mut cursor = 0;
                    #from_bytes_calls
                    Self #destructuring
                }

                fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
                    #write_bytes_input_check
                    let Self #destructuring = self;
                    let mut cursor = 0;
                    #write_bytes_calls
                }
            }
        }
    }
}

fn generate_roc_trait_method_impls_for_enum(
    type_name: &syn::Ident,
    data: &syn::DataEnum,
    crate_root: &TokenStream,
) -> TokenStream {
    let (matches_with_from_bytes_calls, matches_with_write_bytes_calls): (Vec<_>, Vec<_>) = data
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let ident = &variant.ident;
            let discriminant: u8 = idx.try_into().expect("discriminant should fit in u8");

            match &variant.fields {
                syn::Fields::Unit => (
                    quote! {
                        #discriminant => Ok(Self::#ident),
                    },
                    quote! {
                        Self::#ident => {
                            buffer[0] = #discriminant;
                        }
                    },
                ),
                syn::Fields::Named(fields) => {
                    let destructuring = generate_destructuring_for_named_fields(fields);

                    let from_bytes_calls =
                        generate_from_bytes_calls_for_fields(&fields.named, crate_root);

                    let write_bytes_calls =
                        generate_write_bytes_calls_for_fields(&fields.named, crate_root);

                    (
                        quote! {
                            #discriminant => {
                                let mut cursor = 1;
                                #from_bytes_calls
                                Ok(Self::#ident #destructuring)
                            }
                        },
                        quote! {
                            Self::#ident #destructuring => {
                                buffer[0] = #discriminant;
                                let mut cursor = 1;
                                #write_bytes_calls
                            }
                        },
                    )
                }
                syn::Fields::Unnamed(fields) => {
                    let destructuring = generate_destructuring_for_unnamed_fields(fields);

                    let from_bytes_calls =
                        generate_from_bytes_calls_for_fields(&fields.unnamed, crate_root);

                    let write_bytes_calls =
                        generate_write_bytes_calls_for_fields(&fields.unnamed, crate_root);

                    (
                        quote! {
                            #discriminant => {
                                let mut cursor = 1;
                                #from_bytes_calls
                                Ok(Self::#ident #destructuring)
                            }
                        },
                        quote! {
                            Self::#ident #destructuring => {
                                buffer[0] = #discriminant;
                                let mut cursor = 1;
                                #write_bytes_calls
                            }
                        },
                    )
                }
            }
        })
        .unzip();

    let from_bytes_input_check = generate_from_bytes_input_check(type_name, crate_root);
    let write_bytes_input_check = generate_write_bytes_input_check(type_name, crate_root);
    let type_name = type_name.to_string();

    quote! {
        fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
            #from_bytes_input_check
            match bytes[0] {
                #(#matches_with_from_bytes_calls)*
                invalid => Err(::anyhow::anyhow!(
                    "Got invalid discriminant {invalid} for `{}`", #type_name
                )),
            }
        }

        fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
            #write_bytes_input_check
            match self {
                #(#matches_with_write_bytes_calls)*
            }
            Ok(())
        }
    }
}

fn generate_destructuring_for_named_fields(fields: &syn::FieldsNamed) -> TokenStream {
    let parts = fields.named.iter().enumerate().map(|(idx, field)| {
        let real_ident = field.ident.as_ref().unwrap();
        let dummy_ident = field_ident(idx);
        quote! {
            #real_ident: #dummy_ident
        }
    });
    quote! {
        { #(#parts),* }
    }
}

fn generate_destructuring_for_unnamed_fields(fields: &syn::FieldsUnnamed) -> TokenStream {
    let parts = (0..fields.unnamed.len()).map(|idx| field_ident(idx).to_token_stream());
    quote! {
        ( #(#parts),* )
    }
}

fn generate_from_bytes_calls_for_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    crate_root: &TokenStream,
) -> TokenStream {
    let calls = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| generate_from_bytes_call_for_field(idx, &field.ty, crate_root));
    quote! {
        #(#calls)*
    }
}

fn generate_write_bytes_calls_for_fields(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    crate_root: &TokenStream,
) -> TokenStream {
    let calls = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| generate_write_bytes_call_for_field(idx, &field.ty, crate_root));
    quote! {
        #(#calls)*
    }
}

fn generate_from_bytes_call_for_field(
    field_idx: usize,
    field_ty: &syn::Type,
    crate_root: &TokenStream,
) -> TokenStream {
    let field_ident = field_ident(field_idx);
    quote! {
        cursor += <#field_ty as #crate_root::Roc>::SERIALIZED_SIZE;
        let #field_ident = <#field_ty as #crate_root::Roc>::from_roc_bytes(
            &bytes[cursor - <#field_ty as #crate_root::Roc>::SERIALIZED_SIZE..cursor],
        )?;
    }
}

fn generate_write_bytes_call_for_field(
    field_idx: usize,
    field_ty: &syn::Type,
    crate_root: &TokenStream,
) -> TokenStream {
    let field_ident = field_ident(field_idx);
    quote! {
        cursor += <#field_ty as #crate_root::Roc>::SERIALIZED_SIZE;
        <#field_ty as #crate_root::Roc>::write_roc_bytes(
            #field_ident,
            &mut buffer[cursor - <#field_ty as #crate_root::Roc>::SERIALIZED_SIZE..cursor],
        )?;
    }
}

fn generate_from_bytes_input_check(
    type_name: &syn::Ident,
    crate_root: &TokenStream,
) -> TokenStream {
    let type_name = type_name.to_string();
    quote! {
        if bytes.len() != <Self as #crate_root::Roc>::SERIALIZED_SIZE {
            ::anyhow::bail!(
                "Expected {} bytes for `{}`, got {}",
                <Self as #crate_root::Roc>::SERIALIZED_SIZE,
                #type_name,
                bytes.len()
            );
        }
    }
}

fn generate_write_bytes_input_check(
    type_name: &syn::Ident,
    crate_root: &TokenStream,
) -> TokenStream {
    let type_name = type_name.to_string();
    quote! {
        if buffer.len() != <Self as #crate_root::Roc>::SERIALIZED_SIZE {
            ::anyhow::bail!(
                "Expected buffer of length {} for writing `{}`, got {}",
                <Self as #crate_root::Roc>::SERIALIZED_SIZE,
                #type_name,
                buffer.len()
            );
        }
    }
}

fn field_ident(field_idx: usize) -> syn::Ident {
    format_ident!("field_{field_idx}")
}

#[cfg(feature = "enabled")]
fn generate_registered_type_submit(
    args: &ResolvedAttributeArgs,
    rust_type_name: &Ident,
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    let rust_type_path = generate_qualified_type_path_str(rust_type_name);
    let package_name = string_option_to_tokens(args.package_name.as_deref());
    let parent_modules = string_option_to_tokens(args.parent_modules.as_deref());
    let module_name = &args.module_name;
    let function_postfix = string_option_to_tokens(args.function_postfix.as_deref());
    let flags = generate_type_flags(crate_root, args.type_category);
    let docstring = extract_and_process_docstring(&input.attrs);
    let type_name = &args.type_name;
    let composition =
        generate_type_composition(input, crate_root, args.type_category, static_assertions)?;
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::RegisteredType {
                rust_type_path: Some(#rust_type_path),
                package_name: #package_name,
                parent_modules: #parent_modules,
                module_name: #module_name,
                function_postfix: #function_postfix,
                serialized_size: <#rust_type_name as #crate_root::Roc>::SERIALIZED_SIZE,
                flags: #flags,
                ty: #crate_root::ir::Type {
                    id: <#rust_type_name as #crate_root::Roc>::ROC_TYPE_ID,
                    docstring: #docstring,
                    name: #type_name,
                    composition: #composition,
                }
            }
        }
    })
}

#[cfg(not(feature = "enabled"))]
fn generate_registered_type_submit(
    _args: &ResolvedAttributeArgs,
    _rust_type_name: &Ident,
    _input: &syn::DeriveInput,
    _crate_root: &TokenStream,
    _static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    Ok(quote! {})
}

fn generate_associated_dependencies_submit(
    for_type: &syn::Type,
    dependency_types: &[syn::Type],
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    let dependencies = generate_type_id_list(dependency_types, crate_root, MAX_DEPENDENCIES);
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::ir::AssociatedDependencies {
                for_type_id: <#for_type as #crate_root::Roc>::ROC_TYPE_ID,
                dependencies: #dependencies,
            }
        }
    })
}

fn generate_associated_constant_submit(
    for_type: &syn::Type,
    sequence_number: usize,
    specified_name: Option<String>,
    constant: &syn::ImplItemConst,
    expr: String,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    let docstring = extract_and_process_docstring(&constant.attrs);
    let name = specified_name.unwrap_or_else(|| constant.ident.to_string().to_lowercase());
    let ty = generate_containable_type(
        |ty, crate_root| generate_inferrable_type(generate_translatable_type, ty, crate_root),
        Box::new(constant.ty.clone()),
        crate_root,
    )?;
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::ir::AssociatedConstant {
                for_type_id: <#for_type as #crate_root::Roc>::ROC_TYPE_ID,
                sequence_number: #sequence_number,
                docstring: #docstring,
                name: #name,
                ty: #ty,
                expr: #expr,
            }
        }
    })
}

fn generate_associated_function_submit(
    for_type: &syn::Type,
    sequence_number: usize,
    specified_name: Option<String>,
    function: &syn::ImplItemFn,
    body: String,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    let docstring = extract_and_process_docstring(&function.attrs);
    let name = specified_name.unwrap_or_else(|| function.sig.ident.to_string());
    let arguments = generate_function_arguments::<MAX_FUNCTION_ARGS>(&function.sig, crate_root)?;
    let return_type = generate_function_return_type(&function.sig.output, crate_root)?;
    Ok(quote! {
        #[cfg(feature = "roc_codegen")]
        inventory::submit! {
            #crate_root::ir::AssociatedFunction {
                for_type_id: <#for_type as #crate_root::Roc>::ROC_TYPE_ID,
                sequence_number: #sequence_number,
                docstring: #docstring,
                name: #name,
                arguments: #arguments,
                body: #body,
                return_type: #return_type,
            }
        }
    })
}

fn generate_type_flags(crate_root: &TokenStream, type_category: TypeCategory) -> TokenStream {
    match type_category {
        // All primitives are required to be POD
        TypeCategory::Primitive | TypeCategory::Pod => {
            quote! { #crate_root::RegisteredTypeFlags::IS_POD }
        }
        TypeCategory::Inline => {
            quote! { #crate_root::RegisteredTypeFlags::empty() }
        }
    }
}

fn generate_type_composition(
    input: &syn::DeriveInput,
    crate_root: &TokenStream,
    type_category: TypeCategory,
    static_assertions: &mut Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    if type_category == TypeCategory::Primitive {
        return Ok(quote! {
            #crate_root::ir::TypeComposition::Primitive(
                #crate_root::ir::PrimitiveKind::LibraryProvided {
                    precision: #crate_root::ir::PrimitivePrecision::PrecisionIrrelevant,
               }
            )
        });
    }
    match &input.data {
        syn::Data::Struct(data) => {
            let type_name = &input.ident;

            let fields = generate_fields(input, &data.fields, crate_root, MAX_STRUCT_FIELDS)?;

            Ok(quote! {
                #crate_root::ir::TypeComposition::Struct {
                    alignment: ::std::mem::align_of::<#type_name>(),
                    fields: #fields
                }
            })
        }
        syn::Data::Enum(data) => {
            let variants = generate_variants(input, data, crate_root, false, static_assertions)?;
            Ok(quote! {
                #crate_root::ir::TypeComposition::Enum(#variants)
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
            #crate_root::ir::TypeFields::None
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
                #crate_root::ir::TypeFields::Named(#fields)
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
                #crate_root::ir::TypeFields::Unnamed(#fields)
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
                Some(#crate_root::ir::NamedTypeField {
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
        #crate_root::utils::StaticList([#(#fields)*])
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
                Some(#crate_root::ir::UnnamedTypeField {
                    ty: #ty,
                }),
            }
        })
        .chain(iter::repeat_n(
            quote! {None,},
            max_fields - fields.unnamed.len(),
        ));

    quote! {
        #crate_root::utils::StaticList([#(#fields)*])
    }
}

fn generate_field_type(field: &syn::Field, crate_root: &TokenStream) -> TokenStream {
    if let syn::Type::Array(array) = &field.ty {
        let elem_ty = &array.elem;
        let len = &array.len;
        quote! {
            #crate_root::ir::FieldType::Array {
                elem_type_id: <#elem_ty as #crate_root::Roc>::ROC_TYPE_ID,
                len: #len,
            }
        }
    } else {
        let ty = &field.ty;
        quote! {
            #crate_root::ir::FieldType::Single {
                type_id: <#ty as #crate_root::Roc>::ROC_TYPE_ID,
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

    if data.variants.len() > MAX_ENUM_VARIANTS {
        return Err(syn::Error::new_spanned(
            input,
            format!(
                "the `roc` attribute does not support this many variants ({}/{})",
                data.variants.len(),
                MAX_ENUM_VARIANTS
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
                MAX_ENUM_VARIANT_FIELDS,
            )?;

            let serialized_size = generate_summed_field_size_expr(&variant.fields, crate_root)
                .unwrap_or_else(|| quote! {0});

            let docstring = extract_and_process_docstring(&variant.attrs);
            let ident = &variant.ident;
            let ident_str = ident.to_string();

            Ok(quote! {
                Some(#crate_root::ir::TypeVariant {
                    docstring: #docstring,
                    ident: #ident_str,
                    serialized_size: #serialized_size,
                    fields: #fields,
                }),
            })
        })
        .chain(iter::repeat_n(
            Ok(quote! {None,}),
            MAX_ENUM_VARIANTS - data.variants.len(),
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
        #crate_root::ir::TypeVariants(#crate_root::utils::StaticList([#(#variants)*]))
    })
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
                    quote! { #crate_root::ir::MethodReceiver::RefSelf }
                } else {
                    quote! { #crate_root::ir::MethodReceiver::OwnedSelf }
                };
                Ok(quote! {
                    Some(#crate_root::ir::FunctionArgument::Receiver(#receiver)),
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
                let ty = generate_containable_type(
                    |ty, crate_root| {
                        generate_inferrable_type(generate_translatable_type, ty, crate_root)
                    },
                    arg.ty.clone(),
                    crate_root,
                )?;
                Ok(quote! {
                    Some(#crate_root::ir::FunctionArgument::Typed(
                        #crate_root::ir::TypedFunctionArgument {
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
        #crate_root::ir::FunctionArguments(#crate_root::utils::StaticList([#(#args)*]))
    })
}

fn generate_function_return_type(
    return_type: &syn::ReturnType,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    match return_type {
        syn::ReturnType::Type(_, ty) => generate_containable_type(
            |ty, crate_root| generate_inferrable_type(generate_translatable_type, ty, crate_root),
            ty.clone(),
            crate_root,
        ),
        syn::ReturnType::Default => Err(syn::Error::new_spanned(
            return_type,
            "the `roc` attribute does not support functions returning nothing",
        )),
    }
}

fn generate_containable_type(
    generate_contained_type: impl Fn(Box<syn::Type>, &TokenStream) -> syn::Result<TokenStream>,
    mut ty: Box<syn::Type>,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    ty = unwrap_references(ty)?;
    match ty.as_ref() {
        syn::Type::Array(syn::TypeArray { elem, .. })
        | syn::Type::Slice(syn::TypeSlice { elem, .. }) => {
            let ty = generate_contained_type(elem.clone(), crate_root)?;
            return Ok(quote! {
                #crate_root::ir::Containable::List(#ty)
            });
        }
        syn::Type::Tuple(syn::TypeTuple { elems, .. }) => {
            if elems.len() == 2 || elems.len() == 3 {
                let tuple_len = elems.len();

                let tuple_type_ident = format_ident!("Tuple{tuple_len}");

                let types = elems
                    .iter()
                    .map(|ty| generate_contained_type(Box::new(ty.clone()), crate_root))
                    .collect::<syn::Result<Vec<_>>>()?;

                return Ok(quote! {
                    #crate_root::ir::Containable::#tuple_type_ident(#(#types, )*)
                });
            }
        }
        _ => {}
    }
    if let Some(ok_ty) = extract_result_ok_type(&ty) {
        let ty = generate_contained_type(Box::new(ok_ty.clone()), crate_root)?;
        return Ok(quote! {
            #crate_root::ir::Containable::Result(#ty)
        });
    }

    let ty = generate_contained_type(ty, crate_root)?;
    Ok(quote! {
        #crate_root::ir::Containable::Single(#ty)
    })
}

fn generate_inferrable_type(
    generate_specific_type: impl Fn(Box<syn::Type>, &TokenStream) -> syn::Result<TokenStream>,
    mut ty: Box<syn::Type>,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    ty = unwrap_references(ty)?;
    match ty.as_ref() {
        syn::Type::Path(type_path)
            if type_path.qself.is_none()
                && type_path.path.segments.len() == 1
                && type_path.path.segments[0].ident == "Self" =>
        {
            Ok(quote! {
                #crate_root::ir::Inferrable::SelfType
            })
        }
        _ => {
            let ty = generate_specific_type(ty.clone(), crate_root)?;
            Ok(quote! {
                #crate_root::ir::Inferrable::Specific(#ty)
            })
        }
    }
}

fn generate_translatable_type(
    mut ty: Box<syn::Type>,
    crate_root: &TokenStream,
) -> syn::Result<TokenStream> {
    ty = unwrap_references(ty)?;
    if type_is_string(&ty) {
        Ok(quote! {
            #crate_root::ir::TranslatableType::Special(
                #crate_root::ir::SpecialType::String
            )
        })
    } else {
        Ok(quote! {
            #crate_root::ir::TranslatableType::Registered(
                <#ty as #crate_root::Roc>::ROC_TYPE_ID
            )
        })
    }
}

fn extract_result_ok_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
        return None;
    };
    let segment = path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) =
        &segment.arguments
    else {
        return None;
    };
    let syn::GenericArgument::Type(ok_ty) = args.first()? else {
        return None;
    };
    if args.len() > 2 {
        return None;
    }
    Some(ok_ty)
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

fn unwrap_references(mut ty: Box<syn::Type>) -> syn::Result<Box<syn::Type>> {
    while let syn::Type::Reference(syn::TypeReference {
        elem, mutability, ..
    }) = *ty
    {
        if mutability.is_some() {
            return Err(syn::Error::new_spanned(
                elem,
                "the `roc` attribute does not support mutable references",
            ));
        }
        ty = elem;
    }
    Ok(ty)
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
                Some(<#ty as #crate_root::Roc>::ROC_TYPE_ID),
            }
        })
        .chain(iter::repeat_n(quote! {None,}, max_types - types.len()));

    quote! {
        #crate_root::utils::StaticList([#(#ids)*])
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

fn extract_associated_constant_attribute_args(
    constant: &syn::ImplItemConst,
) -> Option<AssociatedConstantAttributeArgs> {
    for attribute in &constant.attrs {
        if let syn::Meta::List(syn::MetaList { path, tokens, .. }) = &attribute.meta {
            let Some(last) = path.segments.last() else {
                continue;
            };
            if last.ident != "roc" {
                continue;
            }
            return syn::parse2(tokens.clone()).ok();
        }
    }
    None
}

fn extract_associated_function_attribute_args(
    func: &syn::ImplItemFn,
) -> Option<AssociatedFunctionAttributeArgs> {
    for attribute in &func.attrs {
        if let syn::Meta::List(syn::MetaList { path, tokens, .. }) = &attribute.meta {
            let Some(last) = path.segments.last() else {
                continue;
            };
            if last.ident != "roc" {
                continue;
            }
            return syn::parse2(tokens.clone()).ok();
        }
    }
    None
}

fn string_option_to_tokens(opt: Option<&str>) -> TokenStream {
    if let Some(string) = opt {
        quote! { Some(#string) }
    } else {
        quote! { None }
    }
}
