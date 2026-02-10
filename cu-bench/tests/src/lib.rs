use client::e2e_helpers::mollusk::deploy_file_to_program_name;
use mollusk_svm::Mollusk;
use solana_address::Address;

/// Creates a new [`Mollusk`] context for a cu-bench program.
pub fn new_cu_bench_mollusk(program_id: &Address, so_filename: &str) -> Mollusk {
    Mollusk::new(program_id, &deploy_file_to_program_name(so_filename))
}
