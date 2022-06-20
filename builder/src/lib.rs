use builder_attr::BuilderAttr;
use field_info::FieldInfo;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Ident, PathArguments, Type};

mod builder_attr;
mod field_info;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = syn::parse_macro_input!(input as syn::DeriveInput);
    proc_macro::TokenStream::from(match impl_builder_derive(&parsed.ident, &parsed) {
        Ok(res) => res,
        Err(e) => e.to_compile_error(),
    })
}

fn impl_builder_derive(struct_name: &Ident, ast: &DeriveInput) -> Result<TokenStream, syn::Error> {
    Ok(match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let builder_name =
                    syn::Ident::new(&format!("{struct_name}Builder"), ast.span());
                let fields = fields
                    .named
                    .iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?;
                let setters = get_setters(&fields)?;
                let members = get_members(&fields)?;
                let checks = get_checks(struct_name, &fields)?;
                quote! {
                    #[derive(Default)]
                    struct #builder_name {
                        #members
                    }
                    impl #builder_name {
                        #setters
                        fn build(&mut self) -> ::std::result::Result<#struct_name, ::std::boxed::Box<dyn ::std::error::Error>> {
                            Ok(#checks)
                        }
                    }
                    impl #struct_name {
                        fn builder() -> #builder_name {
                            Default::default()
                        }
                    }
                }
            }
            _ => Err(syn::Error::new(ast.span(), "expected named fields"))?,
        },
        _ => Err(syn::Error::new(
            ast.span(),
            "Builder derive is only supported on structs",
        ))?,
    })
}

fn get_setters(fields: &[FieldInfo]) -> Result<TokenStream, syn::Error> {
    let field_setters = fields
        .iter()
        .map(|field| {
            let arg_typ = field.optional.unwrap_or(&field.ty);
            let field_name = field.name;
            if let Some(vec_typ) = field.vec {
                let attr = field.attrs.iter().next();
                match attr {
                    Some(attr) => {
                        let builder_attr: BuilderAttr = attr.try_into()?;
                        let func_name = &builder_attr.value_ident;
                        let mut setter = quote_spanned! { field.span.clone() =>
                            fn #func_name(&mut self, val: #vec_typ) -> &mut Self {
                                self.#field_name.push(val);
                                self
                            }
                        };
                        if func_name.to_string() != field_name.to_string() {
                            setter = quote_spanned! { field.span.clone() =>
                                #setter

                                fn #field_name(&mut self, val: #arg_typ) -> &mut Self {
                                    self.#field_name = val;
                                    self
                                }
                            }
                        }
                        Ok(setter)
                    }
                    None => Ok(quote_spanned! { field.span.clone() =>
                        fn #field_name(&mut self, val: #arg_typ) -> &mut Self {
                            self.#field_name = val;
                            self
                        }
                    }),
                }
            } else {
                Ok(quote_spanned! { field.span.clone() =>
                    fn #field_name(&mut self, val: #arg_typ) -> &mut Self {
                        self.#field_name = Some(val);
                        self
                    }
                })
            }
        })
        .collect::<Result<Vec<_>, syn::Error>>()?;
    Ok(quote! {
        #(#field_setters)*
    })
}

fn get_members(fields: &[FieldInfo]) -> Result<TokenStream, syn::Error> {
    let members = fields
        .iter()
        .map(|field| {
            let name = field.name;
            let option_type = field.optional.unwrap_or(field.ty);
            if let Some(inner_type) = field.vec {
                Ok(quote! {
                    #name: ::std::vec::Vec<#inner_type>,
                })
            } else {
                Ok(quote! {
                    #name: ::std::option::Option<#option_type>,
                })
            }
        })
        .collect::<Result<Vec<_>, syn::Error>>()?;
    Ok(quote! {
        #(#members)*
    })
}

fn get_checks(struct_name: &Ident, fields: &[FieldInfo]) -> Result<TokenStream, syn::Error> {
    let checks = fields
        .iter()
        .map(|field| {
            let name = field.name;
            Ok(if field.optional.is_some() || field.vec.is_some() {
                quote! {
                    #name: self.#name.clone(),
                }
            } else {
                quote! {
                    #name: self.#name.clone().ok_or_else(|| String::from("#name is not set"))?,
                }
            })
        })
        .collect::<Result<Vec<_>, syn::Error>>()?;
    Ok(quote! {
        #struct_name {
            #(#checks)*
        }
    })
}

fn get_generic_typ<'a>(typ: &'a Type, gen_name: &str) -> Result<Option<&'a Type>, syn::Error> {
    Ok(match typ {
        Type::Path(type_path) => {
            if type_path.path.segments.len() != 1 {
                None
            } else {
                let segment = &type_path.path.segments[0];
                if segment.ident.to_string() != gen_name {
                    None
                } else {
                    match &segment.arguments {
                        PathArguments::AngleBracketed(args) => {
                            if args.args.len() != 1 {
                                None
                            } else {
                                match &args.args[0] {
                                    syn::GenericArgument::Type(typ) => Some(typ),
                                    _ => None,
                                }
                            }
                        }
                        _ => None,
                    }
                }
            }
        }
        _ => None,
    })
}
