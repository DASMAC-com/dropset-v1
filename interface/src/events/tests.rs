use strum::IntoEnumIterator;

extern crate std;
use std::collections::HashSet;

use super::*;

#[test]
fn test_ixn_tag_try_from_u8_happy_path() {
    for variant in DropsetEventTag::iter() {
        let variant_u8 = variant as u8;
        assert_eq!(
            DropsetEventTag::from_repr(variant_u8).unwrap(),
            DropsetEventTag::try_from(variant_u8).unwrap(),
        );
        assert_eq!(DropsetEventTag::try_from(variant_u8).unwrap(), variant);
    }
}

#[test]
fn test_ixn_tag_try_from_u8_exhaustive() {
    let valids = DropsetEventTag::iter()
        .map(|v| v as u8)
        .collect::<HashSet<_>>();

    for v in 0..=u8::MAX {
        if valids.contains(&v) {
            assert_eq!(
                DropsetEventTag::from_repr(v).unwrap(),
                DropsetEventTag::try_from(v).unwrap(),
            );
            assert_eq!(DropsetEventTag::try_from(v).unwrap() as u8, v);
        } else {
            assert_eq!(
                DropsetEventTag::from_repr(v).is_none(),
                DropsetEventTag::try_from(v).is_err(),
            );
        }
    }
}
