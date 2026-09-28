use crate::backend::ir::{IrBlock, IrDoc, IrInline};

use super::expr::BinOp;
use super::rir::{RBinOp, RMathExpr};

use super::{
    expr::{self, Expr},
    ir::IrMathSource,
    solve::{EvalCtx, eval},
};

#[derive(Default)]
struct Stats {
    envelopes: usize,
    parse_errors: usize,
    eval_errors: usize,
}

pub fn dump_document_math(ir: &IrDoc) -> String {
    let mut out = String::new();
    let mut stats = Stats::default();

    for chunk in &ir.chunks {
        walk_block(&chunk.kind, chunk.id, &mut out, &mut stats);
    }

    if stats.envelopes == 0 {
        out.push_str("math: no envelopes found\n");
    } else {
        out.push_str(&format!(
            "math: {} envelope(s), {} parse error(s), {} eval error(s)\n",
            stats.envelopes, stats.parse_errors, stats.eval_errors
        ));
    }
    out
}

pub fn dump_expression(expression: &Expr) -> String {
    format!(
        "parse: {expression:?}\nfree vars: {:?}\n",
        expression.free_vars()
    )
}

fn walk_block(block: &IrBlock, chunk_id: usize, out: &mut String, stats: &mut Stats) {
    match block {
        IrBlock::Heading { inlines, .. } | IrBlock::Paragraph(inlines) => {
            walk_inlines(inlines, chunk_id, out, stats);
        }
        IrBlock::Quote { body, .. } => {
            for inner in body {
                walk_block(inner, chunk_id, out, stats);
            }
        }
        IrBlock::List { items } => {
            for item in items {
                for inner in &item.body {
                    walk_block(inner, chunk_id, out, stats);
                }
            }
        }
        IrBlock::Code { .. } | IrBlock::Rule => {}
    }
}

fn walk_inlines(inlines: &[IrInline], chunk_id: usize, out: &mut String, stats: &mut Stats) {
    for inline in inlines {
        match inline {
            IrInline::Math(source) => dump_envelope(chunk_id, source, out, stats),
            IrInline::Bold(inner)
            | IrInline::Italic(inner)
            | IrInline::Underline(inner)
            | IrInline::Strike(inner) => walk_inlines(inner, chunk_id, out, stats),
            _ => {}
        }
    }
}

fn dump_envelope(chunk_id: usize, source: &IrMathSource, out: &mut String, stats: &mut Stats) {
    stats.envelopes += 1;
    out.push_str(&format!(
        "math #{}: chunk {}, line {}, mode {:?}\n  span {:?}, payload {:?}\n  raw: {:?}\n",
        stats.envelopes,
        chunk_id,
        source.line,
        source.mode,
        source.span,
        source.payload_span,
        source.raw
    ));

    let expression = match expr::parse(&source.raw) {
        Ok(expression) => expression,
        Err(error) => {
            stats.parse_errors += 1;
            out.push_str(&format!(
                "  parse error: {}: {}\n",
                error.code, error.message
            ));
            return;
        }
    };

    out.push_str(&format!("  parse: {expression:?}\n"));
    out.push_str(&format!("  free vars: {:?}\n", expression.free_vars()));

    match eval(&expression, &EvalCtx::default()) {
        Ok(value) => out.push_str(&format!("  eval: {}\n", value.to_plain_string())),
        Err(error) => {
            stats.eval_errors += 1;
            out.push_str(&format!(
                "  eval error: {}: {}\n",
                error.code, error.message
            ));
        }
    }
}

pub fn to_display(expr: &Expr) -> RMathExpr {
    match expr {
        Expr::Number(raw) => RMathExpr::Number(raw.clone()),
        Expr::Ident(name) => RMathExpr::Ident(name.clone()),
        Expr::Binary { op, lhs, rhs } => RMathExpr::Binary {
            op: to_display_op(*op),
            lhs: Box::new(to_display(lhs)),
            rhs: Box::new(to_display(rhs)),
        },
        Expr::Neg(operand) => RMathExpr::Unary(Box::new(to_display(operand))),
        Expr::Paren(inner) => RMathExpr::Paren(Box::new(to_display(inner))),
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
