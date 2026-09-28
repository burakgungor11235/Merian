use crate::{
    backend::rir::{ErrorRenderStrategy, RError, RErrorKind},
    math::debug::to_display,
};

use super::{MathMode, expr, ir::IrMathSource, rir::RMathExpr};

pub fn handle_math(source: &IrMathSource) -> Result<RMathExpr, RError> {
    match source.mode {
        MathMode::Display => expr::parse(&source.raw)
            .map(|expression| to_display(&expression))
            .map_err(|error| translate_error(source, error)),
        MathMode::Compute | MathMode::Both => Err(math_error(
            source,
            RErrorKind::Unsupported,
            "unsupported-mode",
            "math evaluation is not implemented for this mode",
        )),
    }
}

fn translate_error(source: &IrMathSource, error: expr::ExprError) -> RError {
    // Span is payload-relative so it pairs with `source` (see `RError::span`):
    // the renderer highlights `source[span]`.
    let start = error.offset.min(source.raw.len());
    let end = (start + error.len).min(source.raw.len());
    let kind = if error.code == "unsupported-statement" {
        RErrorKind::Unsupported
    } else {
        RErrorKind::Syntax
    };

    RError {
        kind,
        code: error.code.to_string(),
        message: error.message,
        span: Some(start..end),
        source: Some(source.raw.clone()),
        render: ErrorRenderStrategy::SourceFallback,
    }
}

fn math_error(source: &IrMathSource, kind: RErrorKind, code: &str, message: &str) -> RError {
    // cyka
    RError {
        kind,
        code: code.to_owned(),
        message: message.to_owned(),
        span: Some(0..source.raw.len()),
        source: Some(source.raw.clone()),
        render: ErrorRenderStrategy::SourceFallback,
    }
}
