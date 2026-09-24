use crate::backend::rir::{ErrorRenderStrategy, RError, RErrorKind};

use super::{MathMode, ir::IrMathSource, rir::RMathExpr};

pub fn handle_math(source: &IrMathSource) -> Result<RMathExpr, RError> {
    match source.mode {
        MathMode::Display => parse_fraction(&source.raw, source),
        MathMode::Compute | MathMode::Both => Err(math_error(
            source,
            RErrorKind::Unsupported,
            "unsupported-mode",
            "math evaluation is not implemented for this mode",
        )),
    }
}

fn parse_fraction(raw: &str, source: &IrMathSource) -> Result<RMathExpr, RError> {
    let mut parts = raw.trim().split('/');
    let numerator = parts.next().unwrap_or_default().trim();
    let denominator = parts.next().unwrap_or_default().trim();

    if numerator.is_empty()
        || denominator.is_empty()
        || parts.next().is_some()
        || !numerator.bytes().all(|byte| byte.is_ascii_digit())
        || !denominator.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(math_error(
            source,
            RErrorKind::Syntax,
            "unsupported-expression",
            "right now supports numeric fractions only",
        ));
    }

    Ok(RMathExpr::Fraction {
        numerator: Box::new(RMathExpr::Number(numerator.to_owned())),
        denominator: Box::new(RMathExpr::Number(denominator.to_owned())),
    })
}

fn math_error(source: &IrMathSource, kind: RErrorKind, code: &str, message: &str) -> RError {
    RError {
        kind,
        code: code.to_owned(),
        message: message.to_owned(),
        span: Some(source.payload_span.clone()),
        source: Some(source.raw.clone()),
        render: ErrorRenderStrategy::SourceFallback,
    }
}
