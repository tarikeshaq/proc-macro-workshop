use field_info::FieldInfo;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_quote, spanned::Spanned, token::Where, Data, DeriveInput, Fields, GenericArgument,
    GenericParam, Ident, PathArguments, ReturnType, Type, TypePath, WhereClause, WherePredicate,
};

macro_rules! toss_syn_error {
    (@ATTR $spanner:expr) => {
        toss_syn_error!(@SPAN $spanner.span(), "expected #[debug = \"...\"]")
    };
    (@STRUCT $spanner:expr) => {
        toss_syn_error!(@SPAN $spanner.span(), "can only implement Custom debug on named structs")
    };
    (@SPAN $span:expr, $message:expr) => {
        return Err(syn::Error::new($span, $message))
    }
}

mod field_info;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as DeriveInput);
    TokenStream::from(match impl_debug_derive(parsed) {
        Ok(res) => res,
        Err(e) => e.to_compile_error(),
    })
}

fn impl_debug_derive(mut ast: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if let Data::Struct(data_struct) = &ast.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            let fields: Vec<FieldInfo> = fields_named
                .named
                .iter()
                .map(TryInto::try_into)
                .collect::<syn::Result<Vec<_>>>()?;
            let field_debug_struct = get_field_debug_struct(&fields)?;
            let excluded_phantom_types = add_trait_debug_bound(&mut ast, &fields)?;
            let associated_type_paths = fields
                .iter()
                .flat_map(|field| field.associated_types.iter())
                .collect::<Vec<&TypePath>>();
            let struct_ident = &ast.ident;
            let struct_str = struct_ident.to_string();
            let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
            let updated_where_clause = update_where_clause(
                &excluded_phantom_types,
                &associated_type_paths,
                where_clause,
            )?;
            Ok(quote! {
                impl #impl_generics ::std::fmt::Debug for #struct_ident #ty_generics #updated_where_clause {
                    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        fmt.debug_struct(#struct_str)
                        #field_debug_struct
                        .finish()
                    }
                }
            })
        } else {
            toss_syn_error!(@STRUCT ast)
        }
    } else {
        toss_syn_error!(@STRUCT ast)
    }
}

fn get_field_debug_struct(fields: &[FieldInfo]) -> syn::Result<proc_macro2::TokenStream> {
    let field_debug_structs = fields
        .iter()
        .map(|field| {
            let field_str = field.name_str.clone();
            let field_format_str = field.format_str.clone();
            let field_ident = field.name.clone();
            Ok(quote! {
                .field(#field_str, &::std::format_args!(#field_format_str, &self.#field_ident))
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        #(#field_debug_structs)*
    })
}

fn add_trait_debug_bound(
    ast: &mut DeriveInput,
    fields: &[FieldInfo],
) -> syn::Result<Vec<GenericParam>> {
    let mut phantom_types = vec![];
    for param in &mut ast.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if fields
                .iter()
                .filter(|field_info| is_ident_used(&field_info.ty, &type_param.ident))
                .count()
                == fields
                    .iter()
                    .filter(|field_info| is_in_phantom_type(&field_info, &type_param.ident))
                    .count()
            {
                // The type is only mentioned in phantom types, we should not add it to the bounds
                // and instead, note it down, and add as a where PhantomData<T>: Debug later
                phantom_types.push(param.clone())
            } else {
                type_param.bounds.push(parse_quote!(::std::fmt::Debug));
            }
        }
    }
    Ok(phantom_types)
}

fn is_ident_used(ty: &Type, ident: &Ident) -> bool {
    match ty {
        Type::Array(type_array) => is_ident_used(&type_array.elem, ident),
        Type::BareFn(type_bare_fn) => {
            type_bare_fn
                .inputs
                .iter()
                .any(|fn_arg| is_ident_used(&fn_arg.ty, ident))
                || {
                    match &type_bare_fn.output {
                        ReturnType::Type(_, inner) => is_ident_used(&inner, ident),
                        _ => false,
                    }
                }
        }
        Type::Group(type_group) => is_ident_used(&type_group.elem, ident),
        Type::ImplTrait(_) => false,
        Type::Infer(_) => false,
        Type::Macro(_) => false,
        Type::Ptr(type_ptr) => is_ident_used(&type_ptr.elem, ident),
        Type::Reference(type_ref) => is_ident_used(&type_ref.elem, ident),
        Type::Slice(type_slice) => is_ident_used(&type_slice.elem, ident),
        Type::Paren(type_paren) => is_ident_used(&type_paren.elem, ident),
        Type::Path(type_path) => {
            type_path.path.is_ident(ident)
                || type_path.path.segments.iter().any(|seg| {
                    &seg.ident == ident
                        || match &seg.arguments {
                            PathArguments::AngleBracketed(inner) => {
                                inner.args.iter().any(|arg| match &arg {
                                    GenericArgument::Binding(binding) => {
                                        &binding.ident == ident || is_ident_used(&binding.ty, ident)
                                    }
                                    GenericArgument::Const(_) => {
                                        todo!("Const generics are not yet supported")
                                    }
                                    GenericArgument::Type(inner_ty) => {
                                        is_ident_used(&inner_ty, ident)
                                    }
                                    GenericArgument::Lifetime(_) => false,
                                    GenericArgument::Constraint(constraint) => {
                                        &constraint.ident == ident
                                    }
                                })
                            }
                            PathArguments::Parenthesized(parenthesized_arg) => {
                                parenthesized_arg
                                    .inputs
                                    .iter()
                                    .any(|ty| is_ident_used(ty, ident))
                                    || {
                                        match &parenthesized_arg.output {
                                            ReturnType::Type(_, inner) => {
                                                is_ident_used(&inner, ident)
                                            }
                                            _ => false,
                                        }
                                    }
                            }
                            PathArguments::None => false,
                        }
                })
        }
        Type::Never(_) => false,
        Type::TraitObject(_) => false,
        Type::Tuple(type_tuple) => type_tuple.elems.iter().any(|ty| is_ident_used(ty, ident)),
        Type::Verbatim(_) => false,
        _ => todo!(),
    }
}

fn is_in_phantom_type(field_info: &FieldInfo, ident: &Ident) -> bool {
    if let Some(inner_type) = &field_info.phantom_type {
        if let Type::Path(inner_ty) = inner_type {
            inner_ty.path.is_ident(ident)
        } else {
            false
        }
    } else {
        false
    }
}

fn update_where_clause(
    excluded_types: &[GenericParam],
    associated_type_paths: &[&TypePath],
    where_clause: Option<&WhereClause>,
) -> syn::Result<WhereClause> {
    Ok(match where_clause {
        Some(where_clause) => {
            let mut res = where_clause.clone();
            for excluded_type in excluded_types {
                let where_predicate: WherePredicate = syn::parse_quote!(::std::marker::PhantomData<#excluded_type>: ::std::fmt::Debug);
                res.predicates.push(where_predicate);
            }
            for associated_type in associated_type_paths {
                res.predicates
                    .push(syn::parse_quote!(#associated_type: ::std::fmt::Debug))
            }
            res
        }
        None => {
            let excluded_type_token_streams = excluded_types
                .iter()
                .map(|excluded_type| {
                    quote! {
                        ::std::marker::PhantomData<#excluded_type>: ::std::fmt::Debug,
                    }
                })
                .chain(associated_type_paths.iter().map(|associated_typ| {
                    quote! {
                        #associated_typ: ::std::fmt::Debug
                    }
                }));
            WhereClause {
                where_token: Where {
                    span: Span::call_site(),
                },
                predicates: syn::parse_quote!(#(#excluded_type_token_streams)*),
            }
        }
    })
}
