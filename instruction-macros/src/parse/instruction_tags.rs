use syn::{
    spanned::Spanned,
    DataEnum,
    Expr,
    ExprLit,
    Ident,
    Lit,
};

use crate::ParsingError;

#[derive(Clone, Debug)]
pub struct InstructionTags(pub Vec<InstructionTag>);

#[derive(Clone, Debug)]
pub struct InstructionTag {
    pub _name: Ident,
    pub discriminant: u8,
}

impl TryFrom<&DataEnum> for InstructionTags {
    type Error = syn::Error;

    fn try_from(item: &DataEnum) -> syn::Result<Self> {
        // Implicit discriminants either start at 0 or the last variant that was explicitly set + 1.
        let mut implicit_discriminant = 0;

        let with_discriminants = item
            .variants
            .iter()
            .map(|variant| {
                if !variant.fields.is_empty() {
                    return Err(ParsingError::EnumVariantShouldBeFieldless.into_err(variant.span()));
                }

                let discriminant = match &variant.discriminant {
                    Some((_eq, expr)) => match expr {
                        Expr::Lit(ExprLit { lit, .. }) => match lit {
                            Lit::Int(lit) => lit
                                .base10_parse()
                                .or(Err(ParsingError::InvalidLiteralU8.into_err(lit.span())))?,
                            lit => return Err(ParsingError::InvalidLiteralU8.into_err(lit.span())),
                        },
                        expr => return Err(ParsingError::InvalidLiteralU8.into_err(expr.span())),
                    },
                    None => implicit_discriminant,
                };

                implicit_discriminant = discriminant + 1;

                Ok(InstructionTag {
                    _name: variant.ident.clone(),
                    discriminant,
                })
            })
            .collect::<syn::Result<_>>()?;

        Ok(InstructionTags(with_discriminants))
    }
}
