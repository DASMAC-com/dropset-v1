use std::sync::LazyLock;

use solana_sdk::signature::Keypair;

/// Pubkey: 1118YLQaVU9DUhQjwphJukpKDSNMAiJSdvZfv8KY5Yi
pub static USER_1: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ",
    )
});

/// Pubkey: 222bXXFW4c2UFBRncmEvkGLmQqLGwWBFBNJPx373Kc87
pub static USER_2: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ",
    )
});

/// Pubkey: 333zv4y5CzyYfe84xjGWiWmqsoe966bsBsqM9PVXGtU8
pub static USER_3: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ",
    )
});

/// Pubkey: 444foDqLXTQNFkwVSc7feYEeBuds5Ta3Ue8hf16KAFLZ
pub static USER_4: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ",
    )
});

/// Pubkey: 555L6YBKt5zS9tFqdg8XbyHSFSmH2WnvuRKBbAmiGFPj
pub static USER_5: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ",
    )
});
