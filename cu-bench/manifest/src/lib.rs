// Clippy: we intentionally hold a `RefCell` borrow across `.await`.
// This is safe only because the `Rc<RefCell<ProgramTestContext>>` is never used concurrently.
// Do not call helpers with the same `context` in parallel (e.g. `join!`, `spawn_local`).
#![allow(clippy::await_holding_refcell_ref)]

pub mod fixtures;
pub mod utils;
pub use fixtures::*;
pub use utils::*;
