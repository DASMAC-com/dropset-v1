//! Generates a helper macro that maps raw instruction tags to their corresponding
//! enum variants using efficient, `unsafe` but sound transmutations.
//!
//! Includes compile-time checks to guarantee the generated code’s soundness. These checks output no
//! code in release builds.

use itertools::Itertools;
use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};

use crate::parse::{
    instruction_variant::InstructionVariant,
    parsed_enum::ParsedEnum,
};

/// Renders a declarative macro that fallibly converts a primitive `u8` instruction tag byte to an
/// enum type `T`.
///
/// ## Rendered output
/// ```
/// #[repr(u8)]
/// // `ProgramInstruction` creates a declarative macro for this enum.
/// #[derive(ProgramInstruction)]
/// pub enum MyInstruction {
///     CloseSeat = 0,
///     Deposit = 1,
///     Withdraw = 4,
///     TagWithImplicitDiscriminant, // implicit discriminant `5`
///     OutOfOrderDiscriminant = 3,
/// }
///
/// // Which expands this:
/// MyInstruction_try_from_tag!(tag, ProgramError::InvalidInstructionData)
///
/// // To this:
/// {
///     // The const assertions here ensure soundness.
///     const _: [(); 0] = [(); MyInstruction::CloseSeat as usize];
///     const _: [(); 1] = [(); MyInstruction::Deposit as usize];
///     const _: [(); 4] = [(); MyInstruction::Withdraw as usize];
///     const _: [(); 5] = [(); MyInstruction::TagWithImplicitDiscriminant as usize];
///     const _: [(); 3] = [(); MyInstruction::OutOfOrderDiscriminant as usize];
///
///     match tag {
///         0..=1 | 3..=5 => Ok(unsafe { ::core::mem::transmute::<u8, MyInstruction>(tag) }),
///         _ => Err(ProgramError::InvalidInstructionData),
///     }
/// }
/// ```
///
/// ## Example
/// ```
/// // Use it to implement `TryFrom<u8>`:
/// impl TryFrom<u8> for MyInstruction {
///     type Error = ProgramError;
///   
///     #[inline(always)]
///     fn try_from(tag: u8) -> Result<Self, Self::Error> {
///         MyInstruction_try_from_tag!(tag, ProgramError::InvalidInstructionData)
///     }
/// }
///
/// // Calling it and matching on the tag variant with an early return:
/// match MyInstruction::try_from(tag)? {
///     MyInstruction::CloseSeat => { /* do close seat things */ },
///     MyInstruction::Deposit => { /* do deposit things */ },
///     _ => { /* etc */ },
/// }
/// ```
pub fn render(
    parsed_enum: &ParsedEnum,
    instruction_variants: &[InstructionVariant],
) -> TokenStream {
    let enum_ident = &parsed_enum.enum_ident;

    let sorted_by_discriminants = instruction_variants
        .iter()
        .sorted_by_key(|t| t.discriminant)
        .collect_vec();

    // Build a 2d collection of disjoint ranges, grouped by contiguous discriminants.
    // For example: [0..2, 3..5, 7..99]
    let chunks = sorted_by_discriminants
        .chunk_by(|a, b| a.discriminant + 1 == b.discriminant)
        .collect_vec();

    let ranges = chunks.iter().map(|chunk| {
        let start = Literal::u8_unsuffixed(chunk[0].discriminant);
        if chunk.len() == 1 {
            quote! { #start }
        } else {
            let end =
                Literal::u8_unsuffixed(chunk.last().expect("Should have 1+ elements").discriminant);
            quote! { #start..=#end }
        }
    });

    let full_macro_ident = format_ident!("{}_try_from_tag", enum_ident);
    let doc_comment_1 = format!(
        "Tries to convert a `u8` to a `{}`. `{}` must be defined in the local namespace.",
        enum_ident, enum_ident
    );

    let soundness_checks = const_assertions(parsed_enum, instruction_variants);

    quote! {
        #[macro_export]
        #[doc = #doc_comment_1]
        macro_rules! #full_macro_ident {
            ($tag:expr, $err_variant:path) => {{
                let tag = $tag;
                #soundness_checks
                // Safety: Only valid discriminants are transmuted.
                match tag {
                    #(#ranges)|* => Ok(unsafe { ::core::mem::transmute::<u8, #enum_ident>(tag) }),
                    _ => Err($err_variant),
                }
            }}
        }

        pub use #full_macro_ident;
    }
}

/// Ensure soundness even if the user passes an invalid enum
/// that possesses the same identifier as an enum with a
/// corresponding transmute macro.
///
/// To clarify, this stops the user from doing this:
///
/// ```rust
/// // In some file, `Enum1` is defined with the derive attribute:
/// // This creates the unsafe transmute macro.
/// #[repr(u8)]
/// #[derive(ProgramInstruction)]
/// enum Enum1 { A = 0, B = 1 }
///
/// // In another file, the user creates another `Enum1` with
/// // incompatible variants.
/// #[repr(u8)]
/// enum Enum1 { A = 0, B = 2 }
///
/// // The transmute macro for Enum1 would then expand to:
/// match $tag {
///   // This is *undefined behavior*. Since `Enum1::B == 2`
///   // here, the transmute is invalid and results in UB.
///   0..1 => Ok(unsafe { transmute::<u8, Enum1> }),
///   _ => // ...
/// }
/// ```
///
/// To prevent this, the macro inserts compile time const assertions
/// for every single variant to ensure isomorphism between the original
/// enum and the passed enum, even if they're not strictly the same type.
///
/// This guarantees the transmute is *sound*.
///
/// More specifically, the following const assertions would be generated,
/// triggering a compile-time failure:
/// ```rust
/// const _: [(); 0] = [(); Enum1::A as usize];
/// // ❌ fails because Enum1::B == 2
/// const _: [(); 1] = [(); Enum1::B as usize];
/// ```
fn const_assertions(
    parsed_enum: &ParsedEnum,
    instruction_variants: &[InstructionVariant],
) -> TokenStream {
    instruction_variants
        .iter()
        .map(|variant| {
            let discriminant = Literal::usize_unsuffixed(variant.discriminant as usize);
            let variant_base = &parsed_enum.enum_ident;
            let variant_name = &variant.variant_name;
            let fully_qualified_variant_path = quote! { #variant_base::#variant_name };
            quote! {
                const _: [(); #discriminant] = [(); #fully_qualified_variant_path as usize];
            }
        })
        .collect()
}
