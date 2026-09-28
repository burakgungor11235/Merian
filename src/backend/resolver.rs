use crate::{
    backend::{
        ir::{
            IrBlock, IrChunk, IrDoc,
            IrInline::{self, *},
            IrListItem,
        },
        rir::{RBlock, RChunk, RInline, RListItem, ResolvedDoc},
    },
    math::{ScopeArena, ScopeId, handler::handle_math, rir::write_plain_text},
};

// The resolver is the sole writer of the scope, walking chunks in source order.

pub fn resolve(ir: IrDoc) -> (ResolvedDoc, Vec<String>) {
    let mut resolver = Resolver {
        count: ir.chunks.len(),
        scope: ScopeId::default(),
        tree: ScopeArena::document(),
        diags: Vec::new(),
    };
    resolver.scope = resolver.tree.root();
    let chunks: Vec<RChunk> = ir
        .chunks
        .into_iter()
        .map(|chunk| resolver.chunk(chunk))
        .collect();
    let title = document_title(&chunks).unwrap_or_else(|| "Merian".to_string());
    let Resolver { tree, diags, .. } = resolver;
    (
        ResolvedDoc {
            title,
            chunks,
            symbols: tree,
        },
        diags,
    )
}

struct Resolver {
    count: usize,
    scope: ScopeId,
    tree: ScopeArena,
    diags: Vec<String>,
}

impl Resolver {
    fn chunk(&mut self, chunk: IrChunk) -> RChunk {
        RChunk {
            id: chunk.id,
            kind: self.block(chunk.kind),
        }
    }

    fn block(&mut self, block: IrBlock) -> RBlock {
        match block {
            IrBlock::Heading {
                level,
                permalink,
                inlines,
            } => RBlock::Heading {
                level,
                permalink,
                inlines: self.inlines(inlines),
            },
            IrBlock::Paragraph(inlines) => RBlock::Paragraph(self.inlines(inlines)),
            IrBlock::Quote { level, body } => RBlock::Quote {
                level,
                body: body.into_iter().map(|b| self.block(b)).collect(),
            },
            IrBlock::Code { lang, title, src } => RBlock::Code { lang, title, src },
            IrBlock::List { items } => RBlock::List {
                items: items.into_iter().map(|item| self.list_item(item)).collect(),
            },
            IrBlock::Rule => RBlock::Rule,
        }
    }

    fn list_item(&mut self, item: IrListItem) -> RListItem {
        RListItem {
            depth: item.depth,
            ordered: item.ordered,
            marker: item.marker,
            body: item.body.into_iter().map(|b| self.block(b)).collect(),
        }
    }

    fn inlines(&mut self, inlines: Vec<IrInline>) -> Vec<RInline> {
        inlines
            .into_iter()
            .map(|inline| self.inline(inline))
            .collect()
    }

    fn inline(&mut self, inline: IrInline) -> RInline {
        match inline {
            Text(t) => RInline::Text(t),
            Ref(target) => {
                let count = self.count;
                let exists = target >= 1 && target <= count;
                if !exists {
                    self.diags
                        .push(format!("chunk ref &{target} points outside 1..={count}"));
                }
                RInline::Ref { target, exists }
            }
            Bold(c) => RInline::Bold(self.inlines(c)),
            Italic(c) => RInline::Italic(self.inlines(c)),
            Underline(c) => RInline::Underline(self.inlines(c)),
            Strike(c) => RInline::Strike(self.inlines(c)),
            Code { src, lang } => RInline::Code { src, lang },
            Link { url, text } => RInline::Link { url, text },
            Image { src, alt } => RInline::Image { src, alt },
            Math(source) => match handle_math(&source, &mut self.tree, self.scope) {
                Ok(result) => RInline::Math(result),
                Err(error) => RInline::Error(error),
            },
        }
    }
}

fn document_title(chunks: &[RChunk]) -> Option<String> {
    for chunk in chunks {
        if let RBlock::Heading { inlines, .. } = &chunk.kind {
            let mut s = String::new();
            push_plain_text(inlines, &mut s);
            let t = s.trim().to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

fn push_plain_text(inlines: &[RInline], out: &mut String) {
    for inline in inlines {
        match inline {
            RInline::Text(t) => out.push_str(t),
            RInline::Bold(c) | RInline::Italic(c) | RInline::Underline(c) | RInline::Strike(c) => {
                push_plain_text(c, out)
            }
            RInline::Code { src, .. } => out.push_str(src),
            RInline::Link { text, .. } => {
                if text.is_empty() {
                    out.push_str("A link c:")
                } else {
                    out.push_str(text)
                }
            }
            RInline::Image { alt, .. } => out.push_str(alt),
            RInline::Math(result) => write_plain_text(result, out),
            RInline::Error(error) => {
                out.push_str(error.source.as_deref().unwrap_or(&error.message));
            }
            RInline::Ref { target, .. } => {
                out.push('&');
                out.push_str(&target.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ir::{IrChunk, IrDoc};
    use crate::backend::rir::{RError, RErrorKind};
    use crate::math::{
        MathMode,
        ir::IrMathSource,
        rir::{RBinOp, RMathExpr, RMathResult},
        scope::SymbolKind,
    };

    fn doc_with_refs(targets: &[usize], count: usize) -> IrDoc {
        IrDoc {
            chunks: (1..=count)
                .map(|id| IrChunk {
                    id,
                    kind: IrBlock::Paragraph(vec![]),
                })
                .chain(std::iter::once(IrChunk {
                    id: count + 1,
                    kind: IrBlock::Paragraph(targets.iter().map(|t| IrInline::Ref(*t)).collect()),
                }))
                .collect(),
        }
    }

    #[test]
    fn marks_missing_refs() {
        let (resolved, diags) = resolve(doc_with_refs(&[1, 99], 2));
        // refs live in the last chunk
        let last = resolved.chunks.last().unwrap();
        let RBlock::Paragraph(inlines) = &last.kind else {
            panic!("expected paragraph");
        };
        assert_eq!(
            inlines,
            &vec![
                RInline::Ref {
                    target: 1,
                    exists: true
                },
                RInline::Ref {
                    target: 99,
                    exists: false
                },
            ]
        );
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn zero_is_borked() {
        let (resolved, _) = resolve(doc_with_refs(&[0], 1));
        let RBlock::Paragraph(inlines) = &resolved.chunks.last().unwrap().kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(&inlines[0], RInline::Ref { exists: false, .. }));
    }

    fn math_doc(mode: MathMode, raw: &str) -> IrDoc {
        IrDoc {
            chunks: vec![IrChunk {
                id: 1,
                kind: IrBlock::Paragraph(vec![IrInline::Math(IrMathSource {
                    mode,
                    raw: raw.to_owned(),
                    span: 0..raw.len(),
                    payload_span: 0..raw.len(),
                    line: 1,
                })]),
            }],
        }
    }

    /// One chunk per step, so statements resolve in source order across chunks.
    fn steps_doc(steps: &[(MathMode, &str)]) -> IrDoc {
        IrDoc {
            chunks: steps
                .iter()
                .enumerate()
                .map(|(index, (mode, raw))| IrChunk {
                    id: index + 1,
                    kind: IrBlock::Paragraph(vec![IrInline::Math(IrMathSource {
                        mode: *mode,
                        raw: (*raw).to_owned(),
                        span: 0..raw.len(),
                        payload_span: 0..raw.len(),
                        line: index as u32 + 1,
                    })]),
                })
                .collect(),
        }
    }

    fn nodes(steps: &[(MathMode, &str)]) -> Vec<RInline> {
        let (resolved, diagnostics) = resolve(steps_doc(steps));
        assert!(diagnostics.is_empty());
        resolved
            .chunks
            .iter()
            .map(|chunk| {
                let RBlock::Paragraph(inlines) = &chunk.kind else {
                    panic!("expected paragraph");
                };
                inlines[0].clone()
            })
            .collect()
    }

    fn as_math(node: &RInline) -> &RMathResult {
        match node {
            RInline::Math(result) => result,
            other => panic!("expected math node, got {other:?}"),
        }
    }

    fn results(steps: &[(MathMode, &str)]) -> Vec<RMathResult> {
        nodes(steps).iter().map(as_math).cloned().collect()
    }

    fn displays_an_identifier(result: &RMathResult, name: &str, bound_to: Option<&str>) -> bool {
        let RMathResult::Display(expression) = result else {
            return false;
        };
        fn walk(expression: &RMathExpr, name: &str, bound_to: Option<&str>) -> bool {
            match expression {
                RMathExpr::Ident {
                    name: bound_name,
                    value,
                } => {
                    bound_name == name
                        && match (value, bound_to) {
                            (Some(actual), Some(expected)) => actual.to_plain_string() == expected,
                            (None, None) => true,
                            _ => false,
                        }
                }
                RMathExpr::Binary { lhs, rhs, .. } => {
                    walk(lhs, name, bound_to) || walk(rhs, name, bound_to)
                }
                RMathExpr::Paren(inner) => walk(inner, name, bound_to),
                RMathExpr::Unary(operand) => walk(operand, name, bound_to),
                RMathExpr::Relation { lhs, rhs, .. } => {
                    walk(lhs, name, bound_to) || walk(rhs, name, bound_to)
                }
                RMathExpr::Number(_) => false,
            }
        }
        walk(expression, name, bound_to)
    }

    #[test]
    fn resolves_display_division() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Display, " 1 / 2 "));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(
            &inlines[0],
            RInline::Math(RMathResult::Display(RMathExpr::Binary {
                op: RBinOp::Div,
                ..
            }))
        ));
    }

    #[test]
    fn resolves_display_expression_with_identifiers() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Display, "x + y * 2"));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(
            &inlines[0],
            RInline::Math(RMathResult::Display(RMathExpr::Binary {
                op: RBinOp::Add,
                lhs,
                ..
            })) if matches!(
                **lhs,
                RMathExpr::Ident {
                    ref name,
                    ref value,
                } if name == "x" && value.is_none()
            )
        ));
    }

    #[test]
    fn invalid_display_expression_becomes_error_node() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Display, "1 +"));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(
            &inlines[0],
            RInline::Error(RError {
                kind: RErrorKind::Syntax,
                code,
                ..
            }) if code == "unexpected-eof"
        ));
    }

    #[test]
    fn computes_exact_value() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Compute, " 1 / 3 + 1 / 6 "));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        match &inlines[0] {
            RInline::Math(RMathResult::Value(value)) => {
                assert_eq!(value.to_plain_string(), "1/2");
            }
            other => panic!("expected computed value, got {other:?}"),
        }
    }

    #[test]
    fn both_keeps_input_beside_value() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Both, " 12 * 12 + 5 "));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        let RInline::Math(result) = &inlines[0] else {
            panic!("expected math node, got {:?}", inlines[0]);
        };
        let mut text = String::new();
        write_plain_text(result, &mut text);
        assert_eq!(text, "12 * 12 + 5 = 149");
    }

    #[test]
    fn division_by_zero_becomes_error_node() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Compute, " 1 / 0 "));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(
            &inlines[0],
            RInline::Error(RError {
                kind: RErrorKind::Semantic,
                code,
                ..
            }) if code == "division-by-zero"
        ));
    }

    #[test]
    fn unknown_symbol_becomes_error_node() {
        let (resolved, diagnostics) = resolve(math_doc(MathMode::Compute, " x + 1 "));
        assert!(diagnostics.is_empty());
        let RBlock::Paragraph(inlines) = &resolved.chunks[0].kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(
            &inlines[0],
            RInline::Error(RError {
                kind: RErrorKind::Semantic,
                code,
                ..
            }) if code == "unknown-symbol"
        ));
    }

    #[test]
    fn a_binding_is_silent_and_reaches_later_nodes() {
        let out = results(&[
            (MathMode::Compute, " x := 5 "),
            (MathMode::Compute, " x + 1 "),
            (MathMode::Display, " x + 1 "),
            (MathMode::Both, " x + 1 "),
        ]);
        assert!(matches!(out[0], RMathResult::Silent));
        assert!(matches!(&out[1], RMathResult::Value(value) if value.to_plain_string() == "6"));
        assert!(displays_an_identifier(&out[2], "x", Some("5")));
        assert!(
            matches!(&out[3], RMathResult::Both { value, .. } if value.to_plain_string() == "6")
        );
    }

    #[test]
    fn display_mode_also_binds_but_still_never_evaluates() {
        let out = results(&[
            (MathMode::Display, " y := 9 "),
            (MathMode::Compute, " y + 1 "),
        ]);
        assert!(matches!(out[0], RMathResult::Silent));
        assert!(matches!(&out[1], RMathResult::Value(value) if value.to_plain_string() == "10"));
    }

    #[test]
    fn redeclared_names_overwrite_last_write_wins() {
        let out = results(&[
            (MathMode::Compute, " x := 5 "),
            (MathMode::Compute, " x := 7 "),
            (MathMode::Compute, " x + 1 "),
            (MathMode::Display, " x "),
        ]);
        assert!(matches!(out[0], RMathResult::Silent));
        assert!(matches!(out[1], RMathResult::Silent));
        assert!(matches!(&out[2], RMathResult::Value(value) if value.to_plain_string() == "8"));
        assert!(displays_an_identifier(&out[3], "x", Some("7")));
    }

    #[test]
    fn using_a_name_before_its_declaration_is_an_error() {
        let nodes = nodes(&[
            (MathMode::Compute, " y + 1 "),
            (MathMode::Compute, " y := 2 "),
            (MathMode::Compute, " y + 1 "),
        ]);
        assert!(
            matches!(&nodes[0], RInline::Error(error) if error.code == "unknown-symbol"),
            "got {:?}",
            nodes[0]
        );
        assert!(matches!(&nodes[1], RInline::Math(RMathResult::Silent)));
        assert!(
            matches!(&nodes[2], RInline::Math(RMathResult::Value(value)) if value.to_plain_string() == "3"),
            "got {:?}",
            nodes[2]
        );
    }

    #[test]
    fn a_failed_binding_leaves_the_scope_unchanged() {
        let nodes = nodes(&[
            (MathMode::Compute, " broken := 1 / 0 "),
            (MathMode::Compute, " broken "),
            (MathMode::Display, " broken "),
        ]);
        assert!(
            matches!(&nodes[0], RInline::Error(error) if error.code == "division-by-zero"),
            "got {:?}",
            nodes[0]
        );
        assert!(
            matches!(&nodes[1], RInline::Error(error) if error.code == "unknown-symbol"),
            "got {:?}",
            nodes[1]
        );
        assert!(
            displays_an_identifier(as_math(&nodes[2]), "broken", None),
            "got {:?}",
            nodes[2]
        );
    }

    #[test]
    fn relations_render() {
        let nodes = nodes(&[
            (MathMode::Display, " r = r + 1 "),
            (MathMode::Display, " r: Quantity = 1 "),
            (MathMode::Display, " (n := 3) "), // lol
        ]);
        assert!(
            matches!(
                &nodes[0],
                RInline::Math(RMathResult::Display(RMathExpr::Relation { .. }))
            ),
            "node 0 was {:?}",
            nodes[0]
        );
        for (index, node) in nodes.iter().enumerate().skip(1) {
            assert!(
                matches!(&node, RInline::Error(error) if error.code == "unsupported-statement"),
                "node {index} was {:?}",
                node
            );
        }
    }

    #[test]
    fn symbols_survive_resolution_for_queries() {
        let (resolved, diagnostics) = resolve(steps_doc(&[
            (MathMode::Compute, " x := 5 "),
            (MathMode::Compute, " x := 7 "),
            (MathMode::Compute, " z + 1 "),
        ]));
        assert!(diagnostics.is_empty());

        let dump = resolved.symbols.dump();
        assert_eq!(resolved.symbols.scopes().len(), 1);
        assert!(dump.contains("x = 7"), "{dump}");
        assert!(!dump.contains("  z ="), "{dump}");

        let root = resolved.symbols.root();
        let symbol = resolved.symbols.get(root, "x").expect("x is bound");
        assert_eq!(symbol.declaration_order, 1);
        assert_eq!(symbol.kind, SymbolKind::Inferred);
    }
}
