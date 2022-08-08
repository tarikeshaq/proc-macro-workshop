use proc_macro2::TokenTree;
use quote::quote;

use crate::Seq;



impl Seq {
    pub(crate) fn expand(&self) -> syn::Result<proc_macro2::TokenStream> {
        let from_num = self.from.base10_parse::<usize>()?;
        let to_num = self.to.base10_parse::<usize>()?;
        let mut result: Vec<proc_macro2::TokenStream> = Vec::with_capacity(to_num - from_num);
        for i in from_num..to_num {
            result.push(self.replace_ident(self.content.clone(), i)?);
        }
        Ok(quote! {
            #(#result)*
        })
    }

    fn replace_ident(
        &self,
        stream: proc_macro2::TokenStream,
        to_replace_with: usize,
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
                                if *candidate_ident == self.ident {
                                    stream_iter.next();
                                    res.push(TokenTree::Ident(proc_macro2::Ident::new(
                                        &format!("{}{}", ident.to_string(), to_replace_with),
                                        tt.span(),
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
                    let mut to_add = if *ident == self.ident {
                        TokenTree::Literal(proc_macro2::Literal::usize_unsuffixed(to_replace_with))
                    } else {
                        tt.clone()
                    };
                    to_add.set_span(tt.span());
                    res.push(to_add);
                }
                TokenTree::Group(group) => {
                    let inner_stream = self.replace_ident(group.stream(), to_replace_with)?;
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
