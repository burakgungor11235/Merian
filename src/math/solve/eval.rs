use super::value::MathValue;
use crate::math::expr::{BinOp, Expr};

/// epic calculator.

#[derive(Debug, Default, Clone)]
pub struct EvalCtx {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError {
    pub code: &'static str,
    pub message: String,
}

impl EvalError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
pub fn eval(expr: &Expr, ctx: &EvalCtx) -> Result<MathValue, EvalError> {
    match expr {
        Expr::Number(raw) => MathValue::from_decimal_str(raw),
        Expr::Ident(name) => Err(EvalError::new(
            "unknown-symbol",
            format!("`{name}` is not bound; bindings arrive in Math M3"),
        )),
        Expr::Binary { op, lhs, rhs } => {
            let lhs = eval(lhs, ctx)?;
            let rhs = eval(rhs, ctx)?;
            match op {
                BinOp::Add => Ok(lhs.add(&rhs)),
                BinOp::Sub => Ok(lhs.sub(&rhs)),
                BinOp::Mul => Ok(lhs.mul(&rhs)),
                BinOp::Div => lhs.div(&rhs),
            }
        }
        Expr::Neg(operand) => Ok(eval(operand, ctx)?.neg()),
        Expr::Paren(inner) => eval(inner, ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::expr::parse;

    fn calc(source: &str) -> String {
        eval(&parse(source).unwrap(), &EvalCtx::default())
            .unwrap()
            .to_plain_string()
    }

    #[test]
    fn exact_arithmetic_with_precedence() {
        assert_eq!(calc("1 / 3 + 1 / 6"), "1/2");
        assert_eq!(calc("1 + 2 * 3"), "7");
        assert_eq!(calc("(1 + 2) * 3"), "9");
        assert_eq!(calc("12 * 12 + 5"), "149");
        assert_eq!(calc("-2 * 3"), "-6");
        assert_eq!(calc("2.5 * 2"), "5");
        assert_eq!(calc("10 / 4"), "5/2");
        assert_eq!(calc("100000000000000000000 * 3"), "300000000000000000000");
    }

    #[test]
    fn unknown_symbols_and_division_by_zero_error() {
        let error = eval(&parse("pi * 2").unwrap(), &EvalCtx::default()).unwrap_err();
        // pipi lol
        assert_eq!(error.code, "unknown-symbol");

        let error = eval(&parse("1 / (2 - 2)").unwrap(), &EvalCtx::default()).unwrap_err();
        assert_eq!(error.code, "division-by-zero");
    }
}
