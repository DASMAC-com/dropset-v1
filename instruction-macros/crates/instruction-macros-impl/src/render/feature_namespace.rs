//! Builds feature-gated namespaces so generated instruction APIs can target different
//! environments without conflicts.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{
    format_ident,
    ToTokens,
    TokenStreamExt,
};
use strum::IntoEnumIterator;

use crate::render::Feature;

/// A newtype representing a feature-specific namespace, where the inner value defines the semantic
/// scope used to organize generated code.
#[derive(PartialEq, Eq, Hash)]
pub struct FeatureNamespace(pub Feature);

impl ToTokens for FeatureNamespace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let snake_namespace = self.0.to_string().replace("-", "_");
        tokens.append(format_ident!("generated_{snake_namespace}"));
    }
}

/// Generated tokens and their associated feature namespace.
pub struct NamespacedTokenStream {
    pub tokens: TokenStream,
    pub namespace: FeatureNamespace,
}

/// Merges multiple namespaced token streams into a map grouped by feature namespace.
pub fn merge_namespaced_token_streams(
    streams: Vec<Vec<NamespacedTokenStream>>,
) -> HashMap<FeatureNamespace, Vec<TokenStream>> {
    let mut hash_map: HashMap<FeatureNamespace, Vec<TokenStream>> = Feature::iter()
        .map(|f| (FeatureNamespace(f), vec![]))
        .collect();

    for NamespacedTokenStream { tokens, namespace } in streams.into_iter().flatten() {
        hash_map.entry(namespace).and_modify(|v| v.push(tokens));
    }

    hash_map
}
