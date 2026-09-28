use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::math::solve::eval::EvalError;

/// BIG INFORMATION:
/// This is just the native CAS. I'm no trying to compete with sympy or other libraries who are more
/// qualified than me. I'll add a pluggable CAS backend later. I'm just trying to do my best and
/// learn along the way. There might be better ways of implenting proper numbers. But this ishte
/// choice that I make __right now__. I can always change it in the future.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathValue {
    inner: BigRational,
}

impl MathValue {
    pub fn from_parts(numerator: BigInt, denominator: BigInt) -> Result<Self, EvalError> {
        if denominator.is_zero() {
            return Err(EvalError::new("division-by-zero", "denominator is zero"));
        }
        Ok(Self {
            inner: BigRational::new(numerator, denominator),
        })
    }

    pub fn from_decimal_str(raw: &str) -> Result<Self, EvalError> {
        let invalid = || EvalError::new("invalid-number", format!("`{raw}` is not a valid number"));
        if raw.is_empty() || raw.matches('.').count() > 1 {
            return Err(invalid());
        }
        if !raw
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
        {
            return Err(invalid());
        }

        let (int_part, frac_part) = raw.split_once('.').unwrap_or((raw, ""));
        let digits = format!("{int_part}{frac_part}");
        let numerator: BigInt = digits.parse().map_err(|_| invalid())?;
        let denominator = BigInt::from(10u32).pow(frac_part.len() as u32);
        Self::from_parts(numerator, denominator)
    }

    pub fn add(&self, rhs: &Self) -> Self {
        Self {
            inner: self.inner.clone() + rhs.inner.clone(),
        }
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        Self {
            inner: self.inner.clone() - rhs.inner.clone(),
        }
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        Self {
            inner: self.inner.clone() * rhs.inner.clone(),
        }
    }

    pub fn div(&self, rhs: &Self) -> Result<Self, EvalError> {
        if rhs.inner.is_zero() {
            return Err(EvalError::new("division-by-zero", "division by zero"));
        }
        Ok(Self {
            inner: self.inner.clone() / rhs.inner.clone(),
        })
    }

    pub fn neg(&self) -> Self {
        Self {
            inner: -self.inner.clone(),
        }
    }

    pub fn to_plain_string(&self) -> String {
        if self.inner.denom().is_one() {
            self.inner.numer().to_string()
        } else {
            format!("{}/{}", self.inner.numer(), self.inner.denom())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_literals_convert_exactly() {
        assert_eq!(
            MathValue::from_decimal_str("2.5")
                .unwrap()
                .to_plain_string(),
            "5/2"
        );
        assert_eq!(
            MathValue::from_decimal_str(".5").unwrap().to_plain_string(),
            "1/2"
        );
        assert_eq!(
            MathValue::from_decimal_str("007")
                .unwrap()
                .to_plain_string(),
            "7"
        );
        assert_eq!(
            MathValue::from_decimal_str("12").unwrap().to_plain_string(),
            "12"
        );
    }

    #[test]
    fn malformed_numbers_are_rejected() {
        for raw in ["", ".", "1.2.3", "1..2", "2a"] {
            assert_eq!(
                MathValue::from_decimal_str(raw).unwrap_err().code,
                "invalid-number",
                "for `{raw}`"
            );
        }
    }

    #[test]
    fn normalization_moves_the_sign_up() {
        let value = MathValue::from_parts(BigInt::from(6), BigInt::from(-4)).unwrap();
        assert_eq!(value.to_plain_string(), "-3/2");
        assert!(MathValue::from_parts(BigInt::from(1), BigInt::from(0)).is_err());
    }
}
