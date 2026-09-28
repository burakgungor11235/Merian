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
    Ident(String),
    Binary {
        op: RBinOp,
        lhs: Box<RMathExpr>,
        rhs: Box<RMathExpr>,
    },
    Unary(Box<RMathExpr>),
    Paren(Box<RMathExpr>),
    Fraction {
        numerator: Box<RMathExpr>,
        denominator: Box<RMathExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RMathResult {
    Display(RMathExpr),
}

pub fn write_plain_text(result: &RMathResult, out: &mut String) {
    match result {
        RMathResult::Display(expression) => write_expression_text(expression, out),
    }
}

fn write_expression_text(expression: &RMathExpr, out: &mut String) {
    match expression {
        RMathExpr::Number(value) => out.push_str(value),
        RMathExpr::Ident(name) => out.push_str(name),
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
        RMathExpr::Fraction {
            numerator,
            denominator,
        } => {
            write_expression_text(numerator, out);
            out.push('/');
            write_expression_text(denominator, out);
        }
    }
}
