use syn::{spanned::Spanned, Lit, Meta, NestedMeta};

pub(crate) struct DebugAttribute {
   pub(crate) format_str: String
}

impl<'a> TryFrom<&'a syn::Attribute> for BuilderAttr {
    type Error = syn::Error;
    fn try_from(value: &'a syn::Attribute) -> Result<Self, Self::Error> {
        let outer_meta = value.parse_meta()?;
        if let Meta::List(list) = &outer_meta {
            let inner = list.nested.iter().next().ok_or_else(|| {
                syn::Error::new(list.span(), "expected attribute to not be empty")
            })?;
            if let NestedMeta::Meta(meta) = inner {
                if let Meta::NameValue(named_value) = meta {
                    let name_ident = match named_value.path.get_ident() {
                        Some(ident) => {
                            if ident.to_string() != "each" {
                                return Err(syn::Error::new(
                                    named_value.span(),
                                    "expected `builder(each = \"...\")`",
                                ));
                            }
                            ident
                        }
                        None => {
                            return Err(syn::Error::new(
                                named_value.span(),
                                "expected path to be an identity \"each\"",
                            ))
                        }
                    };
                    if let Lit::Str(lit_str) = named_value.lit.clone() {
                        let value_ident = syn::Ident::new(&lit_str.value(), named_value.span());
                        Ok(Self {
                            name_ident: name_ident.clone(),
                            value_ident,
                            span: value.span(),
                        })
                    } else {
                        Err(syn::Error::new(
                            named_value.span(),
                            "expected `builder(each = \"...\")`",
                        ))
                    }
                } else {
                    Err(syn::Error::new(
                        meta.span(),
                        "expected `builder(each = \"...\")`",
                    ))
                }
            } else {
                Err(syn::Error::new(
                    inner.span(),
                    "expected `builder(each = \"...\")`",
                ))
            }
        } else {
            Err(syn::Error::new(
                outer_meta.span(),
                "expected `builder(each = \"...\")`",
            ))
        }
    }
}
