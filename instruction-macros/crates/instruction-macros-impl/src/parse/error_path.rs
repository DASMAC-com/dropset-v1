//! See [`ErrorPath`].

use syn::{
    Ident,
    Path,
};

/// Helper type for representing fully qualified error paths (base + variant) in generated code.
#[derive(Debug, Clone)]
pub struct ErrorPath {
    /// The error base segment; e.g. `solana_program_error::ProgramError`
    pub base: Path,
    /// The error variant; e.g. `InvalidInstructionData`
    pub variant: Ident,
}

impl ErrorPath {
    pub fn new(base_str: &str, variant_str: &str) -> Self {
        let base = syn::parse_str::<Path>(base_str).expect("Invalid base path");
        let variant = syn::parse_str::<Ident>(variant_str).expect("Invalid variant ident");

        assert!(
            !base.segments.empty_or_trailing(),
            "Invalid base segment for error type"
        );
        ErrorPath { base, variant }
    }
}
