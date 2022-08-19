use proc_macro2::{TokenTree, Ident};
use quote::quote;

use crate::Seq;



impl Seq {
    pub(crate) fn expand(&self) -> syn::Result<proc_macro2::TokenStream> {
        let content = &self.content;
        Ok(quote! {
            #content
        })
    }

    pub(crate) fn replace_ident(
        stream: proc_macro2::TokenStream,
        to_replace_with: usize,
        replace_ident: &Ident
    ) -> syn::Result<proc_macro2::TokenStream> {
        let mut res = Vec::new();
        let mut stream_iter = stream.into_iter().peekable();
        while let Some(tt) = stream_iter.next() {
            match &tt {
                TokenTree::Punct(punct) => {
                    if punct.as_char() == '~' {
                        if let Some(TokenTree::Ident(_)) = res.last() {
                            let ident = res.pop().unwrap();
                            if let Some(TokenTree::Ident(candidate_ident)) = stream_iter.peek() {
                                if *candidate_ident == *replace_ident {
                                    stream_iter.next();
                                    res.push(TokenTree::Ident(proc_macro2::Ident::new(
                                        &format!("{}{}", ident.to_string(), to_replace_with),
                                        ident.span(),
                                    )));
                                } else {
                                    // This is probably a compiler error
                                    // but we assume that's what the author
                                    // is expecting to see
                                    // we push back the '~'
                                    res.push(ident);
                                    res.push(tt);
                                }
                            } else {
                                // if we are here, we took out the last identifier
                                // prematurely, we put it back in.
                                res.push(ident);
                            }
                        }
                    } else {
                        res.push(tt)
                    }
                }
                TokenTree::Ident(ident) => {
                    let mut to_add = if *ident == *replace_ident {
                        TokenTree::Literal(proc_macro2::Literal::usize_unsuffixed(to_replace_with))
                    } else {
                        tt.clone()
                    };
                    to_add.set_span(tt.span());
                    res.push(to_add);
                }
                TokenTree::Group(group) => {
                    let inner_stream = Self::replace_ident(group.stream(), to_replace_with, replace_ident)?;
                    let mut to_add =
                        TokenTree::Group(proc_macro2::Group::new(group.delimiter(), inner_stream));
                    to_add.set_span(tt.span());
                    res.push(to_add);
                }
                _ => res.push(tt.clone()),
            }
        }
        Ok(res.into_iter().collect())
    }
}
