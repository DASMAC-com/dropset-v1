//! Derive helper for the [`pack::Pack`] trait.

use instruction_macros_impl::parse::{
    parsed_packed_struct::ParsedStruct,
    parsing_error::ParsingError,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    DeriveInput,
    Fields,
};

pub fn derive_pack(input: DeriveInput) -> syn::Result<TokenStream> {
    let ParsedStruct {
        struct_ident,
        data_struct,
    } = ParsedStruct::new(input)?;

    // Fully qualify the `Pack` trait, otherwise it collides with the proc macro `Pack` derive.
    let pack_trait = quote! { ::instruction_macros::Pack };

    let Fields::Named(fields) = data_struct.fields else {
        return Err(ParsingError::NotAStruct.new_err(data_struct.fields));
    };
    let (field_names, field_lengths, field_offsets) = fields.named.into_iter().fold(
        (vec![], vec![], vec![]),
        |(mut names, mut lengths, mut offsets), arg| {
            let arg_name = arg.ident.expect("All fields should be named");
            let arg_type = arg.ty;
            // The offset is the cumulative lengths of all the fields before the current field.
            let offset = match lengths.len() {
                0 => quote! { 0 },
                _ => quote! { #(#lengths)+* },
            };
            let length = quote! { <#arg_type as #pack_trait>::LEN };

            names.push(quote! { #arg_name });
            lengths.push(length);
            offsets.push(offset);

            (names, lengths, offsets)
        },
    );

    let res = quote! {
        unsafe impl #pack_trait for #struct_ident {
            const LEN: usize = #(#field_lengths)+*;

            unsafe fn pack(&self, dst: *mut u8) {
                #(#pack_trait::pack(&self.#field_names, dst.add(#field_offsets));)*
            }
        }

        // Const assertion checking that `#struct_ident::LEN` is the sum of all field `::LEN`s.
        const _: [(); <#struct_ident as #pack_trait>::LEN] = [(); #(#field_lengths)+*];
    };

    Ok(res)
}
