use proc_macro2::{TokenTree, TokenStream, Ident};
use syn::{parse::{Parse, ParseBuffer},parenthesized, Token, braced, token::{Brace, Bracket, Paren}, bracketed};

use crate::Seq;

macro_rules! booyaah {
    (@BRACE $paren_type:ident, $res:ident, $buff:ident, $from:ident, $to:ident, $replace:ident) => {
        let inside;
        braced!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $from, $to, $replace)
    };
    (@BRACKET $paren_type:ident, $res:ident, $buff:ident, $from:ident, $to:ident, $replace:ident) => {
        let inside;
        bracketed!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $from, $to, $replace)
    };
    (@PAREN $paren_type:ident, $res:ident, $buff:ident, $from:ident, $to:ident, $replace:ident) => {
        let inside;
        parenthesized!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $from, $to, $replace)
    };
    (@COMMON $paren_type:ident, $inside:ident, $res:ident, $buff:ident, $from:ident, $to:ident, $replace:ident) => {
        let mut inner_res = Vec::new();
        Self::eval_token_tree(&$inside, $from, $to, $replace, (&mut inner_res, $res.1))?;
        let mut tt = TokenTree::Group(proc_macro2::Group::new($paren_type, inner_res.into_iter().collect()));
        tt.set_span($inside.span());
        $res.0.push(tt);
    };
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![in]>()?;
        let from = input.parse::<syn::LitInt>()?;
        input.parse::<syn::Token![..]>()?;
        let to = input.parse::<syn::LitInt>()?;
        let content;
        syn::braced!(content in input);
        let from_num = from.base10_parse::<usize>()?;
        let to_num = to.base10_parse::<usize>()?;
        let mut actual_content = Vec::new();
        let mut found_repeated_section = false;
        Self::eval_token_tree(&content, from_num, to_num, &ident, (&mut actual_content, &mut found_repeated_section))?;
        let mut content_stream = actual_content.into_iter().collect::<TokenStream>();
        if !found_repeated_section {
            // we repeat the whole content
            let mut repeated = Vec::new();
            for num in from_num..to_num {
                let res = Self::replace_ident(content_stream.clone(), num, &ident)?;
                for val in res {
                    repeated.push(val)
                }
            }
            content_stream = repeated.into_iter().collect();
        }
        Ok(
            Self {
                content: content_stream
            }
        )
    }
}


impl Seq {
    fn eval_token_tree(buff: &ParseBuffer, from: usize, to: usize, replace_ident: &Ident, res: (&mut Vec<TokenTree>, &mut bool)) -> syn::Result<()> {
        if buff.is_empty() {
            return Ok(())
        }
        if buff.peek(syn::Token![#]) && buff.peek2(syn::token::Paren) {
            // We are starting the repeat section, lets consume the hash token
            // then consume the rest into the repeat
            buff.parse::<Token![#]>()?;
            let repeat_content;
            parenthesized!(repeat_content in buff);
            let repeat_content_stream = repeat_content.parse::<TokenStream>();
            for i in from..to {
                let replaced = Self::replace_ident(repeat_content_stream.clone()?, i, &replace_ident)?;
                for item in replaced {
                    res.0.push(item)
                }
            }
            buff.parse::<Token![*]>()?;
            *res.1 = true;
        } else if buff.peek(Brace) {
            let brr = proc_macro2::Delimiter::Brace;
            booyaah!(@BRACE brr, res, buff, from, to, replace_ident);
        } else if buff.peek(Bracket) {
            let brr = proc_macro2::Delimiter::Bracket;
            booyaah!(@BRACKET brr, res, buff, from, to, replace_ident); 
        } else if buff.peek(Paren) {
            let brr = proc_macro2::Delimiter::Parenthesis;
            booyaah!(@PAREN brr, res, buff, from, to, replace_ident);
        } else {
        res.0.push(buff.parse()?);
       }
       return Self::eval_token_tree(buff, from, to, replace_ident, res)
    }
}