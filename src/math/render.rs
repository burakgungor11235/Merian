use super::rir::{RMathExpr, RMathResult};

pub(crate) fn render(result: &RMathResult, output: &mut String) {
    match result {
        RMathResult::Display(expression) => {
            output.push_str(
                "<math class=\"merian-math\" xmlns=\"http://www.w3.org/1998/Math/MathML\" display=\"inline\">",
            );
            render_expression(expression, output);
            output.push_str("</math>");
        }
    }
}

fn render_expression(expression: &RMathExpr, output: &mut String) {
    match expression {
        RMathExpr::Number(value) => {
            output.push_str("<mn>");
            escape_html(value, output);
            output.push_str("</mn>");
        }
        RMathExpr::Fraction {
            numerator,
            denominator,
        } => {
            output.push_str("<mfrac>");
            render_expression(numerator, output);
            render_expression(denominator, output);
            output.push_str("</mfrac>");
        }
    }
}

fn escape_html(text: &str, output: &mut String) {
    for c in text.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(c),
        }
    }
}
