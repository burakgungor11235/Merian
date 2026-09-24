#[derive(Debug, Clone, PartialEq)]
pub enum RMathExpr {
    Number(String),
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
