macro_rules! acc_keypair_fn {
    ($fn_name:ident, $b58:literal) => {
        #[inline]
        pub fn $fn_name() -> &'static ::solana_sdk::signature::Keypair {
            static KP: ::std::sync::LazyLock<::solana_sdk::signature::Keypair> =
                ::std::sync::LazyLock::new(|| {
                    ::solana_sdk::signature::Keypair::from_base58_string($b58)
                });

            ::std::sync::LazyLock::force(&KP)
        }
    };
}

#[allow(unused_imports, dead_code, non_snake_case)]
#[rustfmt::skip]
mod unformatted {
    acc_keypair_fn!(default_payer, "4UahpeecuKmCQJv3XbkzmBxSd5JxdviM6UGMCZ5FsMw5XZv7APjzDb1WM9WcCdKX7rmsYWSi7Cumcf59TvXyLR45");
    acc_keypair_fn!(acc_1111, "4phi3FwSKx8CQssCGPgBZxabjeLBZK7ZhhPrVVo1vsP4T3F9iUdQPFf7wphop5dhYg9CJV35GoPMSTdb95w3FfoE");
    acc_keypair_fn!(acc_2222, "5LqH8f3NYsSHGvEYbaWnRM8swJgEB9SdSrT6KkCeerTMbyZnQgSZhPQwJEVSsfZVMRPz4q4P4UYynEwvw2KApYrP");
    acc_keypair_fn!(acc_3333, "4L8vaVEXLM2kfRtyo99qNhN1DzuBTcoraWjG6JpJWQ8EypLA26pmi3oGyTaFGNi2ZmNQnx2eg1t827YWLnpg3gcM");
    acc_keypair_fn!(acc_4444, "3opivcTjFYRsZ3LZDdeufMmCeVa448THYJPHTPUxMm6btt3U5k32HsG8nMgabnxzPDjKHnaD6fihDWtc7iTVuTot");
    acc_keypair_fn!(acc_5555, "2wrdskZ9ijYkskrBu5UKu8uWLVVSiw1pJuT3i9FdGujTF8jhA95jHR6cpD8VoW87QDHEpHWtBbimPL1ibzB3naUx");
    acc_keypair_fn!(acc_6666, "2LRRFZbrg5NajJaVatxSRsgH7nPQLejnZuLwTatQgUDxa5BqH7FuaxhCvZ2RTN8Gaiib6j7ftWjtBnwGK3HYoaAS");
    acc_keypair_fn!(acc_7777, "4yamknrysKCNWKP1hEWFF3hXJ9ZPpFTxUhp2xkNKAPEGVkm8fGZahn1EPWhqxsNq5XgXeDtwaHWoaW7GWfrZcuqU");
    acc_keypair_fn!(acc_8888, "4Bec61bWHfVz26V2VAbNrwDscGbGrJS6X9D6xGUZNsTH9i6PdtcRLV9wqGpWDFe4R1vyEr8AtCn38HFND6LFWK9");
    acc_keypair_fn!(acc_9999, "3VAwuaDPLt1tSAVADQ8ZfcKsjpuSXVjo2T3zZ4RxMBGDw1DppjGKdTQy8P6Bp2XSx5aEwNY8mvmVabZiKzWZ2oRZ");
    acc_keypair_fn!(acc_AAAA, "ZLnVUjNHpSpYnR9sHRg3RJy7G5Y36gjFrgjkDULdfpPVWbGevpe6LBhhYsLCYPhoVAzJCnENJ64LfGrzGEy4fst");
    acc_keypair_fn!(acc_BBBB, "25b5eC6H82GNrzSF11yva5sZ6ZfvAA9aH6aLYEYTv2ETC79HaLoHrvWethqtiXG8JqHwZ97ZrYwAhug9tW9TpY61");
    acc_keypair_fn!(acc_CCCC, "4HZxL12zVSKskZFaP7eZhFu2iaYmCXcC5Ng4XZUUai4Xi2H2pWyUfbv7YPDQH1PtBAPyT6QqyEZKmyhQPv1aM3iL");
    acc_keypair_fn!(acc_DDDD, "5Ct6Mazy79xKmthE7x1tkJwuKVorWHdRksHCkYKXUBNH9qUAYaeXmHbHqS8nmph58uDrK6kMEHrbT4LgSQzkUx8x");
    acc_keypair_fn!(acc_EEEE, "r44Jg2W77HPeTG7ayubSgLn2f59QdNGnfXMy2eLUKkSrEd8S6u5fZSHyumketSUtu8Q5qRmWETUPNhWhUpHJWLi");
    acc_keypair_fn!(acc_FFFF, "5hcjDdYk6G8Tap95ZFpuasQU5ovxKDVKZkhyZbXdZfWhX3RyfS6ZzsZMTMhKo9xAVBmz7sFPe7gRce8G9DRtjDAK");
}

#[allow(unused_imports)]
pub use unformatted::*;

#[cfg(test)]
mod tests {
    use solana_sdk::signer::Signer;

    use super::*;

    #[test]
    #[rustfmt::skip]
    fn check_pubkeys() {
        assert_eq!(default_payer().pubkey().to_string(), "PAYRidU5w5wtJUxohMzVn9KH8p9p5PERU1SFsrUtKUV");
        assert_eq!(acc_1111().pubkey().to_string(), "11113MwGAy1Aq8qkfPuukq892Zn3tV6uGHWoRYLaUBS");
        assert_eq!(acc_2222().pubkey().to_string(), "2222VkwR14uJeobbCRfr67aEudhqv9gwjaZHpHqja9M5");
        assert_eq!(acc_3333().pubkey().to_string(), "33332xggw8sZC286ToAdABGWbbMRoY4R7j2Wf19GwcYD");
        assert_eq!(acc_4444().pubkey().to_string(), "44448QRU5TV9o1sxmAf9JSEtSbkWB5gqhM9uNNPVdGMc");
        assert_eq!(acc_5555().pubkey().to_string(), "5555yw8g3UayPg4ahVL4YK7F1ziczGzFem4ixBsNQ87a");
        assert_eq!(acc_6666().pubkey().to_string(), "66666jVh9CweunfWTKU8xwQFKRUZ6JxKwv4Bc9XkNkva");
        assert_eq!(acc_7777().pubkey().to_string(), "7777szTJdwzi24nouHHkkJgkoc9vZ5446sSKhfypBtAn");
        assert_eq!(acc_8888().pubkey().to_string(), "8888fUwgatWrZ8yiG7o2HS7YVyj6kbDLKdtMHdV9eNeK");
        assert_eq!(acc_9999().pubkey().to_string(), "9999Zzn87sFQxvAiehDuatABsJ3v9sAmwnBQCXdy5P5");
        assert_eq!(acc_AAAA().pubkey().to_string(), "AAAAoNU5cLHwJv96q6rPH77b7zB6g7ooNjTTaqLzB2CN");
        assert_eq!(acc_BBBB().pubkey().to_string(), "BBBBJv93Rrvsd3Ed24niKAKbx5KZgYGR7fW62XSga9kZ");
        assert_eq!(acc_CCCC().pubkey().to_string(), "CCCCEwvHVX3Ngmdn1zWPWQ8A95odPgkfuJ2BqYmnYyZE");
        assert_eq!(acc_DDDD().pubkey().to_string(), "DDDDLB31znqqwU3uSh5gzzK1XDBFsHpyTpysq8n9LBMA");
        assert_eq!(acc_EEEE().pubkey().to_string(), "EEEEtnxXendMxjR37GzZxECZwKoGPK4E5kPXQUhAdYj4");
        assert_eq!(acc_FFFF().pubkey().to_string(), "FFFF6rhkJEG6KFkmJSK5taWbVNHUb1zqt2ufFp7Sxetb");
    }
}
