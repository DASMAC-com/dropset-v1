use crate::{
    EncodedPrice,
    OrderInfoError,
    ValidatedPriceMantissa,
    BIAS,
    ENCODED_PRICE_INFINITY,
    ENCODED_PRICE_ZERO,
    PRICE_MANTISSA_BITS,
    PRICE_MANTISSA_MASK,
};

/// An enum representing a decoded `EncodedPrice`.
#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub enum DecodedPrice {
    Zero,
    Infinity,
    ExponentAndMantissa {
        price_exponent_biased: u8,
        price_mantissa: ValidatedPriceMantissa,
    },
}

impl DecodedPrice {
    /// Return the optional tuple of exponent and mantissa from a decoded price.
    /// If the decoded price is not a [`DecodedPrice::ExponentAndMantissa`], this returns `None`.
    pub fn as_exponent_and_mantissa(&self) -> Option<(&u8, &ValidatedPriceMantissa)> {
        if let DecodedPrice::ExponentAndMantissa {
            price_exponent_biased,
            price_mantissa,
        } = self
        {
            Some((price_exponent_biased, price_mantissa))
        } else {
            None
        }
    }
}

impl TryFrom<EncodedPrice> for DecodedPrice {
    type Error = OrderInfoError;

    fn try_from(encoded: EncodedPrice) -> Result<Self, Self::Error> {
        let res = match encoded.as_u32() {
            ENCODED_PRICE_ZERO => Self::Zero,
            ENCODED_PRICE_INFINITY => Self::Infinity,
            value => {
                let price_exponent_biased = (value >> PRICE_MANTISSA_BITS) as u8;
                let validated_mantissa = value & PRICE_MANTISSA_MASK;

                Self::ExponentAndMantissa {
                    price_exponent_biased,
                    price_mantissa: ValidatedPriceMantissa::try_from(validated_mantissa)?,
                }
            }
        };

        Ok(res)
    }
}

impl TryFrom<DecodedPrice> for f64 {
    type Error = OrderInfoError;

    fn try_from(decoded: DecodedPrice) -> Result<Self, Self::Error> {
        match decoded {
            DecodedPrice::Zero => Ok(0f64),
            DecodedPrice::Infinity => Err(OrderInfoError::InfinityIsNotAFloat),
            DecodedPrice::ExponentAndMantissa {
                price_exponent_biased,
                price_mantissa,
            } => {
                let res = (price_mantissa.as_u32() as f64)
                    * 10f64.powi(price_exponent_biased as i32 - BIAS as i32);
                Ok(res)
            }
        }
    }
}
