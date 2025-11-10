//! Renders the parsed instruction model into generated modules, helpers, and macros.

mod feature;
mod feature_namespace;
mod instruction_accounts;
mod instruction_data;
mod try_from_tag_macro;

pub use feature::*;
pub use feature_namespace::*;
pub use instruction_accounts::render as render_instruction_accounts;
pub use instruction_data::render as render_instruction_data;
pub use try_from_tag_macro::render as render_try_from_tag_macro;
