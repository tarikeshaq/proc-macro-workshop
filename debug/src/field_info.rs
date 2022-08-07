use syn::{
    spanned::Spanned, Field, GenericArgument, Lit, Meta, PathArguments, ReturnType, Type, TypePath,
};

macro_rules! parse_associated_type_elem {
    ($name:expr, $out:ident) => {
        parse_associated_types(&$name.elem, $out)
    };
}

pub(crate) struct FieldInfo {
    pub(crate) name: syn::Ident,
    pub(crate) name_str: String,
    pub(crate) ty: syn::Type,
    pub(crate) format_str: String,
    pub(crate) phantom_type: Option<Type>,
    pub(crate) associated_types: Vec<TypePath>,
}

impl TryFrom<&'_ syn::Field> for FieldInfo {
    type Error = syn::Error;
    fn try_from(field: &'_ syn::Field) -> Result<Self, Self::Error> {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new(field.span(), "expected field to have an identifier"))?;
        let field_str = field_ident.to_string();
        let field_format_str = get_format_string(field)?;
        let phantom_type = parse_phantom_type(&field.ty)?;
        let mut associated_types = vec![];
        parse_associated_types(&field.ty, &mut associated_types)?;
        Ok(Self {
            name: field_ident.clone(),
            name_str: field_str,
            ty: field.ty.clone(),
            format_str: field_format_str,
            phantom_type,
            associated_types,
        })
    }
}

fn parse_phantom_type(ty: &Type) -> syn::Result<Option<Type>> {
    match ty {
        Type::Path(type_path) => {
            for seg in type_path.path.segments.iter() {
                if let PathArguments::AngleBracketed(inner) = &seg.arguments {
                    if seg.ident.to_string() == "PhantomData" {
                        let type_param = inner.args.iter().next().ok_or_else(|| {
                            syn::Error::new(inner.span(), "PhantomData with no arguments")
                        })?;
                        if let GenericArgument::Type(ty) = type_param {
                            return Ok(Some(ty.clone()));
                        } else {
                            toss_syn_error!(@SPAN type_param.span(), "Invalid type parameter with phantom data")
                        }
                    }
                } else {
                    return Ok(None);
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn get_format_string(field: &Field) -> syn::Result<String> {
    if field.attrs.len() == 0 {
        return Ok(String::from("{:?}"));
    }
    if field.attrs.len() != 1 {
        toss_syn_error!(@ATTR field)
    }
    // We have exactly one attribute, we get it
    let attribute = field.attrs.first().unwrap();
    if let Meta::NameValue(named_value) = attribute.parse_meta()? {
        if let Some(ident) = named_value.path.get_ident() {
            if ident.to_string() != "debug" {
                toss_syn_error!(@ATTR named_value)
            }
        } else {
            toss_syn_error!(@ATTR named_value)
        }
        if let Lit::Str(lit_str) = named_value.lit {
            return Ok(lit_str.value());
        } else {
            toss_syn_error!(@ATTR named_value)
        }
    } else {
        toss_syn_error!(@ATTR attribute)
    }
}

fn parse_associated_types(ty: &Type, out: &mut Vec<TypePath>) -> syn::Result<()> {
    match ty {
        Type::Array(type_arr) => parse_associated_type_elem!(type_arr, out),
        Type::BareFn(bare_fn) => {
            for input in bare_fn.inputs.iter() {
                parse_associated_types(&input.ty, out)?;
            }
            match &bare_fn.output {
                ReturnType::Default => Ok(()),
                ReturnType::Type(_, inner_ty) => parse_associated_types(&inner_ty, out),
            }
        }
        Type::Group(type_group) => parse_associated_type_elem!(type_group, out),
        Type::ImplTrait(_) => Ok(()),
        Type::Infer(_) => Ok(()),
        Type::Macro(_) => Ok(()),
        Type::Never(_) => Ok(()),
        Type::Paren(type_paren) => parse_associated_type_elem!(type_paren, out),
        Type::Path(type_path) => {
            // TODO: Add check to make sure it's a generic type
            if type_path.path.segments.len() > 1 {
                out.push(type_path.clone());
            } else {
                let seg = type_path.path.segments.first().unwrap();
                match &seg.arguments {
                    PathArguments::AngleBracketed(inner_ty) => {
                        for arg in &inner_ty.args {
                            match arg {
                                GenericArgument::Type(generic_type) => {
                                    parse_associated_types(generic_type, out)?
                                }
                                _ => (),
                            }
                        }
                    }
                    PathArguments::Parenthesized(parenthesized) => {
                        for input in &parenthesized.inputs {
                            parse_associated_types(&input, out)?;
                        }
                        match &parenthesized.output {
                            ReturnType::Type(_, inner_ty) => {
                                parse_associated_types(&inner_ty, out)?
                            }
                            _ => (),
                        }
                    }
                    PathArguments::None => (),
                }
            }
            Ok(())
        }
        Type::Ptr(type_ptr) => parse_associated_type_elem!(type_ptr, out),
        Type::Reference(type_ref) => parse_associated_type_elem!(type_ref, out),
        Type::Slice(type_slice) => parse_associated_type_elem!(type_slice, out),
        Type::TraitObject(_) => Ok(()),
        Type::Tuple(type_tuple) => {
            for elem in &type_tuple.elems {
                parse_associated_types(&elem, out)?;
            }
            Ok(())
        }
        Type::Verbatim(_) => Ok(()),
        _ => todo!(),
    }
}
