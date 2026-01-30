// NOTE: `cargo-expand` must be available to run this test.
//
// Ideally `rustfmt` is available, too, otherwise the expanded code won't be formatted.

#[test]
pub fn expand_client() {
    macrotest::expand_args("src/client.rs", ["--features", "client"]);
}

#[test]
pub fn expand_program() {
    macrotest::expand_args("src/program.rs", ["--features", "program"]);
}

#[test]
pub fn expand_events() {
    macrotest::expand_args("src/events.rs", ["--features", "client"]);
}

#[test]
pub fn expand_pack() {
    macrotest::expand("src/pack.rs");
}
