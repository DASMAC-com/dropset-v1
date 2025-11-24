//! See [`render`].

use proc_macro2::TokenStream;
use quote::quote;

/// Render the `Unpack` trait that utilizes a generic error type to facilitate feature-gated
/// aka SDK-based error types.
pub fn render() -> TokenStream {
    quote! {
        pub trait Unpack<E>: Sized {
            fn unpack(data: &[u8]) -> Result<Self, E>;
        }
    }
}
