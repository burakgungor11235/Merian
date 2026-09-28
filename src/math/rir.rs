use super::expr::{BinOp, Expr};
use super::scope::{ScopeArena, ScopeId};
use super::solve::MathValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RBinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl RBinOp {
    pub fn as_str(self) -> &'static str {
        match self {
            RBinOp::Add => "+",
            RBinOp::Sub => "-",
            RBinOp::Mul => "*",
            RBinOp::Div => "/",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RMathExpr {
    Number(String),
    Ident {
        name: String,
        /// `None` when nothing bound the name earlier in source order.
        value: Option<MathValue>,
    },
    Binary {
        op: RBinOp,
        lhs: Box<RMathExpr>,
        rhs: Box<RMathExpr>,
    },
    Unary(Box<RMathExpr>),
    Paren(Box<RMathExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RMathResult {
    Display(RMathExpr),
    Value(MathValue),
    Both { input: RMathExpr, value: MathValue },
    Silent,
}

pub fn to_display(expr: &Expr, tree: &ScopeArena, scope: ScopeId) -> RMathExpr {
    match expr {
        Expr::Number(raw) => RMathExpr::Number(raw.clone()),
        Expr::Ident(name) => RMathExpr::Ident {
            name: name.clone(),
            value: tree.get(scope, name).map(|symbol| symbol.value.clone()),
        },
        Expr::Binary { op, lhs, rhs } => RMathExpr::Binary {
            op: to_display_op(*op),
            lhs: Box::new(to_display(lhs, tree, scope)),
            rhs: Box::new(to_display(rhs, tree, scope)),
        },
        Expr::Neg(operand) => RMathExpr::Unary(Box::new(to_display(operand, tree, scope))),
        Expr::Paren(inner) => RMathExpr::Paren(Box::new(to_display(inner, tree, scope))),
    }
}

fn to_display_op(op: BinOp) -> RBinOp {
    match op {
        BinOp::Add => RBinOp::Add,
        BinOp::Sub => RBinOp::Sub,
        BinOp::Mul => RBinOp::Mul,
        BinOp::Div => RBinOp::Div,
    }
}

pub fn write_plain_text(result: &RMathResult, out: &mut String) {
    match result {
        RMathResult::Display(expression) => write_expression_text(expression, out),
        RMathResult::Value(value) => out.push_str(&value.to_plain_string()),
        RMathResult::Both { input, value } => {
            write_expression_text(input, out);
            out.push_str(" = ");
            out.push_str(&value.to_plain_string());
        }
        RMathResult::Silent => {}
    }
}

fn write_expression_text(expression: &RMathExpr, out: &mut String) {
    match expression {
        RMathExpr::Number(value) => out.push_str(value),
        RMathExpr::Ident { name, .. } => out.push_str(name),
        RMathExpr::Binary { op, lhs, rhs } => {
            write_expression_text(lhs, out);
            out.push(' ');
            out.push_str(op.as_str());
            out.push(' ');
            write_expression_text(rhs, out);
        }
        RMathExpr::Unary(operand) => {
            out.push('-');
            write_expression_text(operand, out);
        }
        RMathExpr::Paren(inner) => {
            out.push('(');
            write_expression_text(inner, out);
            out.push(')');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_skips_silent_bindings() {
        let mut out = String::new();
        write_plain_text(&RMathResult::Silent, &mut out);
        assert_eq!(out, "");
    }

    #[test]
    fn plain_text_reads_a_display_tree() {
        let mut out = String::new();
        write_plain_text(
            &RMathResult::Display(RMathExpr::Binary {
                op: RBinOp::Div,
                lhs: Box::new(RMathExpr::Number("1".to_string())),
                rhs: Box::new(RMathExpr::Number("2".to_string())),
            }),
            &mut out,
        );
        assert_eq!(out, "1 / 2");
    }
}
