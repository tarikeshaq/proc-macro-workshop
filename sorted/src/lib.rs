use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;

mod visitor;
use syn::visit_mut::VisitMut;
use visitor::MatchSorted;
#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as syn::Item);
    let _ = args;
    TokenStream::from(match impl_sorted_attr(&parsed) {
        Ok(res) => res,
        Err(e) => {
            let mut err_stream = e.to_compile_error();
            parsed.to_tokens(&mut err_stream);
            err_stream
        }
    })
}

#[proc_macro_attribute]
pub fn check(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut parsed = syn::parse_macro_input!(input as syn::Item);

    TokenStream::from(match impl_check_attr(&mut parsed) {
        Ok(res) => res,
        Err(e) => {
            let mut err_stream = e.to_compile_error();
            parsed.to_tokens(&mut err_stream);
            err_stream
        }
    })
}

fn impl_check_attr(item: &mut syn::Item) -> syn::Result<proc_macro2::TokenStream> {
    let mut match_sorted: MatchSorted = Default::default();
    match_sorted.visit_item_mut(item);
    match_sorted.toss_compiler_error()?;
    let mut res = proc_macro2::TokenStream::new();
    item.to_tokens(&mut res);
    Ok(res)
}

fn impl_sorted_attr(item: &syn::Item) -> syn::Result<proc_macro2::TokenStream> {
    let mut res = proc_macro2::TokenStream::new();
    if let syn::Item::Enum(enum_) = &item {
        validate_enums_sorted(enum_)?;
        enum_.to_tokens(&mut res);
        return Ok(res);
    }
    Err(syn::Error::new(
        Span::call_site(),
        "expected enum or match expression",
    ))
}

fn validate_enums_sorted(enum_: &syn::ItemEnum) -> syn::Result<()> {
    let mut prev: Option<&syn::Variant> = None;
    for variant in &enum_.variants {
        if let Some(prev_variant) = prev {
            let variant_name = variant.ident.to_string();
            if variant_name < prev_variant.ident.to_string() {
                // lets go find the one that is the first value smaller greater than variant_name
                for other_variant in &enum_.variants {
                    let other_variant_name = other_variant.ident.to_string();
                    if variant_name < other_variant_name {
                        return Err(syn::Error::new(
                            variant.ident.span(),
                            format!("{} should sort before {}", variant_name, other_variant_name),
                        ));
                    }
                }
            }
        }
        prev = Some(variant)
    }
    Ok(())
}
