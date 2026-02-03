//! Parsing implementations for the various [`ArgumentType`]s that can be used for the `args`
//! derive attribute.

use std::fmt::Display;

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    quote,
    ToTokens,
};
use syn::{
    parse::Parse,
    Type,
};

use crate::{
    parse::known_type::KnownType,
    render::pack_struct_fields::fully_qualified_pack_trait,
};

#[derive(Debug, Clone)]
pub enum ArgumentType {
    KnownType(KnownType),
    UnknownType(Type),
}

#[derive(Debug, Clone)]
pub enum Size {
    Lit(usize),
    Expr(TokenStream),
}

impl Default for Size {
    fn default() -> Self {
        Size::Lit(0)
    }
}

impl Size {
    /// Add two sizes, folding where possible.
    pub fn plus(self, rhs: Size) -> Size {
        match (self, rhs) {
            // Two size literals can be reduced to a single literal instead of an expression.
            (Size::Lit(a), Size::Lit(b)) => Size::Lit(a + b),
            // 0 + Size or Size + 0 is simplified to Size.
            (Size::Lit(0), v) | (v, Size::Lit(0)) => v,
            // Create an expression of the two sizes added together.
            (a, b) => Size::Expr(quote! { #a + #b }),
        }
    }
}

impl ToTokens for Size {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let size_tokens = match self {
            Size::Lit(n) => {
                let unsuffixed = Literal::usize_unsuffixed(*n);
                quote! { #unsuffixed }
            }
            Size::Expr(ts) => ts.clone(),
        };

        tokens.extend(size_tokens)
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Remove spaces for the Display impl to make types more readable in doc comments.
        write!(f, "{}", self.to_token_stream().to_string().replace(" ", ""))
    }
}

impl Parse for ArgumentType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;

        let res = match KnownType::new(ty.clone()) {
            Some(known_type) => Self::KnownType(known_type),
            None => Self::UnknownType(ty),
        };

        Ok(res)
    }
}

pub trait ParsedPackableType {
    fn size(&self) -> Size;

    fn as_fully_qualified_type(&self) -> Type;
}

impl ParsedPackableType for ArgumentType {
    fn size(&self) -> Size {
        let pack_trait = fully_qualified_pack_trait();
        match self {
            Self::KnownType(k) => k.size(),
            Self::UnknownType(uk) => Size::Expr(quote! { <#uk as #pack_trait>::LEN }),
        }
    }

    fn as_fully_qualified_type(&self) -> Type {
        match self {
            Self::KnownType(k) => k.as_fully_qualified_type(),
            Self::UnknownType(uk) => uk.clone(),
        }
    }
}

impl Display for ArgumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Format the known types as simplified strings rather than fully qualified paths to
            // keep them simple and readable.
            ArgumentType::KnownType(k) => write!(f, "{k}"),
            // Otherwise just use the `TokenStream` `Display` implementation.
            _ => write!(f, "{}", self.as_fully_qualified_type().to_token_stream()),
        }
    }
}
