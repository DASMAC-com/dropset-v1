use std::path::PathBuf;

use mollusk_svm::Mollusk;
use solana_address::Address;

/// Resolves a `.so` filename under `target/deploy/` to the absolute path that [`Mollusk::new`]
/// expects (without the `.so` suffix).
fn deploy_file_to_program_name(so_filename: &str) -> String {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("target/deploy/")
        .join(so_filename)
        .canonicalize()
        .map(|p| {
            p.to_str()
                .expect("Path should convert to a &str")
                .strip_suffix(".so")
                .expect("Deploy file should have an `.so` suffix")
                .to_string()
        })
        .expect("Should create relative target/deploy/ path")
}

/// Creates a new [`Mollusk`] context for a cu-bench program.
pub fn new_cu_bench_mollusk(program_id: &Address, so_filename: &str) -> Mollusk {
    Mollusk::new(program_id, &deploy_file_to_program_name(so_filename))
}
