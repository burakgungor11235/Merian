use crate::backend::ir::{IrBlock, IrDoc, IrInline};

use super::{
    expr::{self, Expr, Statement},
    ir::IrMathSource,
    scope::{ScopeArena, ScopeId},
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
    let mut tree = ScopeArena::document();
    let scope = tree.root();

    for chunk in &ir.chunks {
        walk_block(
            &chunk.kind,
            chunk.id,
            &mut out,
            &mut stats,
            &mut tree,
            scope,
        );
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

fn walk_block(
    block: &IrBlock,
    chunk_id: usize,
    out: &mut String,
    stats: &mut Stats,
    tree: &mut ScopeArena,
    scope: ScopeId,
) {
    match block {
        IrBlock::Heading { inlines, .. } | IrBlock::Paragraph(inlines) => {
            walk_inlines(inlines, chunk_id, out, stats, tree, scope);
        }
        IrBlock::Quote { body, .. } => {
            for inner in body {
                walk_block(inner, chunk_id, out, stats, tree, scope);
            }
        }
        IrBlock::List { items } => {
            for item in items {
                for inner in &item.body {
                    walk_block(inner, chunk_id, out, stats, tree, scope);
                }
            }
        }
        IrBlock::Code { .. } | IrBlock::Rule => {}
    }
}

fn walk_inlines(
    inlines: &[IrInline],
    chunk_id: usize,
    out: &mut String,
    stats: &mut Stats,
    tree: &mut ScopeArena,
    scope: ScopeId,
) {
    for inline in inlines {
        match inline {
            IrInline::Math(source) => dump_envelope(chunk_id, source, out, stats, tree, scope),
            IrInline::Bold(inner)
            | IrInline::Italic(inner)
            | IrInline::Underline(inner)
            | IrInline::Strike(inner) => walk_inlines(inner, chunk_id, out, stats, tree, scope),
            _ => {}
        }
    }
}
/// Bleh!
fn dump_envelope(
    chunk_id: usize,
    source: &IrMathSource,
    out: &mut String,
    stats: &mut Stats,
    tree: &mut ScopeArena,
    scope: ScopeId,
) {
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

    let statement = match expr::parse_statement(&source.raw) {
        Ok(statement) => statement,
        Err(error) => {
            stats.parse_errors += 1;
            out.push_str(&format!(
                "  parse error: {}: {}\n",
                error.code, error.message
            ));
            return;
        }
    };

    let (statement_label, binding, expression) = match statement {
        Statement::Expr(expression) => ("expression".to_string(), None, expression),
        Statement::Binding { name, value } => (format!("binding {name} :="), Some(name), value),
    };

    out.push_str(&format!("  statement: {statement_label}\n"));
    out.push_str(&format!("  parse: {expression:?}\n"));
    out.push_str(&format!("  free vars: {:?}\n", expression.free_vars()));

    match eval(&expression, &EvalCtx { tree, scope }) {
        Ok(value) => {
            let plain = value.to_plain_string();
            out.push_str(&format!("  eval: {plain}\n"));
            if let Some(name) = binding {
                out.push_str(&format!("  bound: {name} = {plain}\n"));
                tree.bind(scope, &name, value, source.payload_span.clone());
            }
        }
        Err(error) => {
            stats.eval_errors += 1;
            out.push_str(&format!(
                "  eval error: {}: {}\n",
                error.code, error.message
            ));
        }
    }
}
