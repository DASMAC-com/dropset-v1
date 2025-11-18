use proc_macro2::TokenStream;
use quote::quote;

pub fn render() -> TokenStream {
    quote! {
        pub trait PackIntoSlice {
            /// This is the byte length **including** the tag byte; i.e., the size of the full event
            /// instruction data in an `instruction_data: &[u8]` slice with the tag.
            const LEN_WITH_TAG: usize;

            /// Packs `Self` as bytes into a given mutable slice.
            ///
            /// Caller is responsible for ensuring the buffer length
            /// is sufficient and that its length is tracked properly.
            unsafe fn pack_into_slice(
                &self,
                buf: &mut [::core::mem::MaybeUninit<u8>],
                offset: usize,
            );
        }
    }
}
