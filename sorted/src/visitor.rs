use syn::{
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Pat,
};

#[derive(Default)]
pub struct MatchSorted {
    compiler_error: Option<syn::Error>,
}

impl VisitMut for MatchSorted {
    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        if let Some((idx, _)) = expr
            .attrs
            .iter()
            .enumerate()
            .find(|(_, attr)| is_attribute_sorted(*attr))
        {
            expr.attrs.remove(idx); // we first remove the sorted attribute
                                    // now we can check if the expression has a sorted match arms
            self.check_sorted_match_arms(&expr.arms);
        }
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_expr_match_mut(self, expr);
    }
}

fn is_attribute_sorted(attr: &syn::Attribute) -> bool {
    if let Ok(meta) = attr.parse_meta() {
        match meta {
            syn::Meta::Path(path) => {
                if path.segments.len() != 1 {
                    return false;
                }
                let seg = &path.segments[0];
                if seg.ident.to_string() != "sorted" {
                    return false;
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

impl MatchSorted {
    fn check_sorted_match_arms(&mut self, arms: &[syn::Arm]) {
        let mut prev = None;
        for arm in arms {
            let curr_name = match arm_to_string(arm) {
                Ok(name) => name,
                Err(e) => {
                    self.compiler_error = Some(e);
                    return;
                }
            };
            let span = arm_to_span(arm);
            if let Some(prev_name) = &prev {
                if prev_name == "_" || curr_name < *prev_name {
                    // we now find the first one that is greater than the current
                    for other_arm in arms {
                        let other_name = match arm_to_string(other_arm) {
                            Ok(name) => name,
                            Err(e) => {
                                self.compiler_error = Some(e);
                                return;
                            }
                        };
                        if other_name == "_" || curr_name < other_name {
                            self.compiler_error = Some(syn::Error::new(
                                span,
                                format!("{} should sort before {}", curr_name, other_name),
                            ))
                        }
                    }
                }
            }
            prev = Some(curr_name)
        }
    }

    pub fn toss_compiler_error(&self) -> syn::Result<()> {
        if let Some(compiler_error) = &self.compiler_error {
            return Err(compiler_error.clone());
        }
        Ok(())
    }
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}

fn arm_to_string(arm: &syn::Arm) -> syn::Result<String> {
    Ok(match &arm.pat {
        Pat::Ident(ident) => ident.ident.to_string(),
        Pat::Path(path) => path_to_string(&path.path),
        Pat::TupleStruct(tuple_struct) => path_to_string(&tuple_struct.path),
        Pat::Struct(struct_) => path_to_string(&struct_.path),
        Pat::Wild(_) => "_".to_string(),
        _ => return Err(syn::Error::new(arm.span(), "unsupported by #[sorted]")),
    })
}

fn arm_to_span(arm: &syn::Arm) -> proc_macro2::Span {
    match &arm.pat {
        Pat::Ident(ident) => ident.ident.span(),
        Pat::Path(path) => path.path.span(),
        Pat::TupleStruct(tuple_struct) => tuple_struct.path.span(),
        Pat::Struct(struct_) => struct_.path.span(),
        Pat::Wild(pat_wild) => pat_wild.span(),
        arm => arm.span(),
    }
}
