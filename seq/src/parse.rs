use proc_macro2::{TokenTree, TokenStream, Ident};
use syn::{parse::{Parse, ParseBuffer},parenthesized, Token, braced, token::{Brace, Bracket, Paren}, bracketed};

use crate::Seq;

macro_rules! booyaah {
    (@BRACE $paren_type:ident, $res:ident, $buff:ident, $range_type:ident, $replace:ident) => {
        let inside;
        braced!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $range_type, $replace)
    };
    (@BRACKET $paren_type:ident, $res:ident, $buff:ident, $range_type:ident, $replace:ident) => {
        let inside;
        bracketed!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $range_type, $replace)
    };
    (@PAREN $paren_type:ident, $res:ident, $buff:ident, $range_type:ident, $replace:ident) => {
        let inside;
        parenthesized!(inside in $buff);
        booyaah!(@COMMON $paren_type, inside, $res, $buff, $range_type, $replace)
    };
    (@COMMON $paren_type:ident, $inside:ident, $res:ident, $buff:ident, $range_type:ident, $replace:ident) => {
        let mut inner_res = Vec::new();
        Self::eval_token_tree(&$inside, $range_type, $replace, (&mut inner_res, $res.1))?;
        let mut tt = TokenTree::Group(proc_macro2::Group::new($paren_type, inner_res.into_iter().collect()));
        tt.set_span($inside.span());
        $res.0.push(tt);
    };
}

enum RangeType {
    Inclusive { from: usize, to: usize },
    Exclusive { from: usize, to: usize }
}

impl RangeType {
    fn repeat_range<F>(&self, mut repeat_fn: F) -> syn::Result<()>
    where F: FnMut(usize) -> syn::Result<()> {
        match &self {
            Self::Inclusive { from, to } => {
                for i in *from..=*to {
                    repeat_fn(i)?
                }
            },
            Self::Exclusive { from, to } => {
                for i in *from..*to {
                    repeat_fn(i)?
                }
            }
        };
        Ok(())
    }
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![in]>()?;
        let from = input.parse::<syn::LitInt>()?;
        input.parse::<syn::Token![..]>()?;
        let range_type: RangeType;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let to = input.parse::<syn::LitInt>()?;
            range_type = RangeType::Inclusive { from: from.base10_parse()?, to: to.base10_parse()? };
        } else {
            let to = input.parse::<syn::LitInt>()?;
            range_type = RangeType::Exclusive { from: from.base10_parse()?, to: to.base10_parse()? };
        }
        let content;
        syn::braced!(content in input);
        let mut actual_content = Vec::new();
        let mut found_repeated_section = false;
        Self::eval_token_tree(&content, &range_type, &ident, (&mut actual_content, &mut found_repeated_section))?;
        let mut content_stream = actual_content.into_iter().collect::<TokenStream>();
        if !found_repeated_section {
            // we repeat the whole content
            let mut repeated = Vec::new();
            range_type.repeat_range(|num| {
                let res = Self::replace_ident(content_stream.clone(), num, &ident)?;
                for val in res {
                    repeated.push(val);
                }
                Ok(())
            })?;
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
    fn eval_token_tree(buff: &ParseBuffer, range_type: &RangeType, replace_ident: &Ident, res: (&mut Vec<TokenTree>, &mut bool)) -> syn::Result<()> {
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
            range_type.repeat_range(|num| {
                let replaced = Self::replace_ident(repeat_content_stream.clone()?, num, &replace_ident)?;
                for item in replaced {
                    res.0.push(item)
                }
                Ok(())
            })?;

            buff.parse::<Token![*]>()?;
            *res.1 = true;
        } else if buff.peek(Brace) {
            let brr = proc_macro2::Delimiter::Brace;
            booyaah!(@BRACE brr, res, buff, range_type, replace_ident);
        } else if buff.peek(Bracket) {
            let brr = proc_macro2::Delimiter::Bracket;
            booyaah!(@BRACKET brr, res, buff, range_type, replace_ident); 
        } else if buff.peek(Paren) {
            let brr = proc_macro2::Delimiter::Parenthesis;
            booyaah!(@PAREN brr, res, buff, range_type, replace_ident);
        } else {
        res.0.push(buff.parse()?);
       }
       return Self::eval_token_tree(buff, range_type, replace_ident, res)
    }
}