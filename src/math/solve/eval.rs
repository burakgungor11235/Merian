use super::value::MathValue;
use crate::math::expr::{BinOp, Expr};
use crate::math::scope::{ScopeArena, ScopeId};

/// epic calculator.

#[derive(Debug, Clone, Copy)]
pub struct EvalCtx<'a> {
    pub tree: &'a ScopeArena,
    pub scope: ScopeId,
}

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
        Expr::Ident(name) => match ctx.tree.get(ctx.scope, name) {
            Some(symbol) => Ok(symbol.value.clone()),
            None => Err(EvalError::new(
                "unknown-symbol",
                format!("`{name}` is not bound"),
            )),
        },
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

    fn eval_in(source: &str, tree: &ScopeArena) -> Result<MathValue, EvalError> {
        eval(
            &parse(source).unwrap(),
            &EvalCtx {
                tree,
                scope: tree.root(),
            },
        )
    }

    fn calc(source: &str) -> String {
        let tree = ScopeArena::document();
        eval_in(source, &tree).unwrap().to_plain_string()
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
    fn unknown_symbols() {
        let tree = ScopeArena::document();
        let error = eval_in("pi * 2", &tree).unwrap_err();
        assert_eq!(error.code, "unknown-symbol");
        assert_eq!(error.message, "`pi` is not bound");
    }

    #[test]
    fn div_by_zero() {
        let tree = ScopeArena::document();
        let error = eval_in("1 / (2 - 2)", &tree).unwrap_err();
        assert_eq!(error.code, "division-by-zero");
    }

    #[test]
    fn bound_symbols_resolve_during_eval() {
        let mut tree = ScopeArena::document();
        let root = tree.root();
        tree.bind(root, "x", MathValue::from_decimal_str("5").unwrap(), 0..6);

        assert_eq!(eval_in("x + 1", &tree).unwrap().to_plain_string(), "6");
        assert_eq!(eval_in("x", &tree).unwrap().to_plain_string(), "5");
        assert_eq!(eval_in("y", &tree).unwrap_err().code, "unknown-symbol");
    }
}
