use syn::{
    Expr,
    ExprLit,
    Lit,
    Variant,
};

use crate::ParsingError;

pub fn try_parse_instruction_discriminant(
    implicit_discriminant: u8,
    variant: &Variant,
) -> syn::Result<u8> {
    if !variant.fields.is_empty() {
        return Err(ParsingError::EnumVariantShouldBeFieldless.new_err(variant));
    }

    let discriminant = match variant.discriminant.as_ref() {
        // Parse the explicit discriminant as a base-10 `u8` value.
        Some((_eq, expr)) => match expr {
            Expr::Lit(ExprLit { lit, .. }) => match lit {
                Lit::Int(lit) => lit
                    .base10_parse()
                    .or(Err(ParsingError::InvalidLiteralU8.new_err(lit)))?,
                lit => return Err(ParsingError::InvalidLiteralU8.new_err(lit)),
            },
            expr => return Err(ParsingError::InvalidLiteralU8.new_err(expr)),
        },
        // There is no discriminant; use the implicit discriminant.
        None => implicit_discriminant,
    };

    Ok(discriminant)
}
