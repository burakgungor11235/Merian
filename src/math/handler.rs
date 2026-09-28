use crate::backend::rir::{ErrorRenderStrategy, RError, RErrorKind};

use super::{
    MathMode,
    expr::{self, Statement},
    ir::IrMathSource,
    rir::{RMathResult, to_display},
    scope::{ScopeArena, ScopeId},
    solve::{EvalCtx, MathValue, eval, eval::EvalError},
};

pub fn handle_math(
    source: &IrMathSource,
    tree: &mut ScopeArena,
    scope: ScopeId,
) -> Result<RMathResult, RError> {
    let statement =
        expr::parse_statement(&source.raw).map_err(|error| translate_error(source, error))?;

    match statement {
        Statement::Binding { name, value } => {
            let bound = eval(&value, &EvalCtx { tree, scope })
                .map_err(|error| translate_eval_error(source, error))?;
            tree.bind(scope, &name, bound, source.payload_span.clone());
            Ok(RMathResult::Silent)
        }
        Statement::Expr(expression) => match source.mode {
            MathMode::Display => Ok(RMathResult::Display(to_display(&expression, tree, scope))),

            MathMode::Compute => eval(&expression, &EvalCtx { tree, scope })
                .map(RMathResult::Value)
                .map_err(|error| translate_eval_error(source, error)),

            MathMode::Both => {
                let input = to_display(&expression, tree, scope);

                eval(&expression, &EvalCtx { tree, scope })
                    .map(|value| RMathResult::Both { input, value })
                    .map_err(|error| translate_eval_error(source, error))
            }
        },
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

fn translate_eval_error(source: &IrMathSource, error: EvalError) -> RError {
    RError {
        kind: RErrorKind::Semantic,
        code: error.code.to_string(),
        message: error.message,
        span: Some(0..source.raw.len()),
        source: Some(source.raw.clone()),
        render: ErrorRenderStrategy::SourceFallback,
    }
}
