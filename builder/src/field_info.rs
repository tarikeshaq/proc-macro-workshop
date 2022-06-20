use syn::{spanned::Spanned, Type};

use crate::get_generic_typ;

pub(crate) struct FieldInfo<'a> {
    pub(crate) name: &'a syn::Ident,
    pub(crate) ty: &'a syn::Type,
    pub(crate) optional: Option<&'a Type>,
    pub(crate) vec: Option<&'a Type>,
    pub(crate) span: proc_macro2::Span,
    pub(crate) attrs: &'a [syn::Attribute],
}

impl<'a> TryFrom<&'a syn::Field> for FieldInfo<'a> {
    type Error = syn::Error;
    fn try_from(field: &'a syn::Field) -> Result<Self, Self::Error> {
        let optional = get_generic_typ(&field.ty, "Option")?;
        let vec = get_generic_typ(&field.ty, "Vec")?;
        Ok(Self {
            name: field
                .ident
                .as_ref()
                .ok_or_else(|| syn::Error::new(field.span(), "expected field to have a name"))?,
            ty: &field.ty,
            optional,
            vec,
            span: field.span(),
            attrs: &field.attrs,
        })
    }
}
