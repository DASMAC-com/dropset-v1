//! Predefined deterministic keypairs for local testing and examples.

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
        "wuDnL8tvfZdoxUS3fSyuQ9CLrYjuGAAef1FYVYJumeBXnspD3193PWUVubSgB3nNo9LUbv3MzcdeGTykkq6RKBV",
    )
});

/// Pubkey: 333zv4y5CzyYfe84xjGWiWmqsoe966bsBsqM9PVXGtU8
pub static USER_3: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "31oK1X2TzwmXzLq98mQwBzknCtX8LY57jYmidSFLi5Ftivhb57WRUF9idjuDAyHacycXeVx9HwSvNceh6Z6sSeHE",
    )
});

/// Pubkey: 444foDqLXTQNFkwVSc7feYEeBuds5Ta3Ue8hf16KAFLZ
pub static USER_4: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "64GjXTfs6fnDNHYHppHBA74Lz3QYZA9fKEDByRwppd13k88K5uAyB5SmJUi7bVGq7YgRssPn5DhQCjDSvXvzYqpZ",
    )
});

/// Pubkey: 555L6YBKt5zS9tFqdg8XbyHSFSmH2WnvuRKBbAmiGFPj
pub static USER_5: LazyLock<Keypair> = LazyLock::new(|| {
    Keypair::from_base58_string(
        "63yZsA3gHByFRMkurN2oKAGWga3xeK3krrn6BorCjKqTutCx674UU9E4xyqjcdy9mcnwBXZGnbMdtySjMjHs67C9",
    )
});

#[test]
fn check_test_keys() {
    use solana_sdk::signer::Signer;

    let users = [
        "1118YLQaVU9DUhQjwphJukpKDSNMAiJSdvZfv8KY5Yi",
        "222bXXFW4c2UFBRncmEvkGLmQqLGwWBFBNJPx373Kc87",
        "333zv4y5CzyYfe84xjGWiWmqsoe966bsBsqM9PVXGtU8",
        "444foDqLXTQNFkwVSc7feYEeBuds5Ta3Ue8hf16KAFLZ",
        "555L6YBKt5zS9tFqdg8XbyHSFSmH2WnvuRKBbAmiGFPj",
    ];

    assert_eq!(USER_1.pubkey().to_string(), users[0]);
    assert_eq!(USER_2.pubkey().to_string(), users[1]);
    assert_eq!(USER_3.pubkey().to_string(), users[2]);
    assert_eq!(USER_4.pubkey().to_string(), users[3]);
    assert_eq!(USER_5.pubkey().to_string(), users[4]);
}
