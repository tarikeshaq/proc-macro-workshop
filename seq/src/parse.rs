use syn::parse::Parse;

use crate::Seq;



impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![in]>()?;
        let from = input.parse::<syn::LitInt>()?;
        input.parse::<syn::Token![..]>()?;
        let to = input.parse::<syn::LitInt>()?;
        let content;
        syn::braced!(content in input);
        let content = content.parse::<proc_macro2::TokenStream>()?;
        Ok(Self {
            ident,
            from,
            to,
            content,
        })
    }
}