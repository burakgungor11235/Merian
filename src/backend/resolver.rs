use crate::backend::{
    ir::{
        IrBlock, IrChunk, IrDoc,
        IrInline::{self, *},
        IrListItem,
    },
    rir::{RBlock, RChunk, RInline, RListItem, ResolvedDoc},
};

// right now this file is technically a placeholder, we are doing pretty much nothing.
//
// right now I just need to get the AST -> IR -> (resolve) -> RIR -> Backend
// sooo, it's the vertical slice saga all over again.

pub fn resolve(ir: IrDoc) -> (ResolvedDoc, Vec<String>) {
    let count = ir.chunks.len();
    let title = document_title(&ir).unwrap_or_else(|| "Merian".to_string());
    let mut diags = Vec::new();
    let chunks = ir
        .chunks
        .into_iter()
        .map(|c| resolve_chunk(c, count, &mut diags))
        .collect();
    (ResolvedDoc { title, chunks }, diags)
}

fn resolve_chunk(chunk: IrChunk, count: usize, diags: &mut Vec<String>) -> RChunk {
    RChunk {
        id: chunk.id,
        kind: resolve_block(chunk.kind, count, diags),
    }
}

fn resolve_block(block: IrBlock, count: usize, diags: &mut Vec<String>) -> RBlock {
    match block {
        IrBlock::Heading {
            level,
            permalink,
            inlines,
        } => RBlock::Heading {
            level,
            permalink,
            inlines: resolve_inlines(inlines, count, diags),
        },
        IrBlock::Paragraph(inlines) => RBlock::Paragraph(resolve_inlines(inlines, count, diags)),
        IrBlock::Quote { level, body } => RBlock::Quote {
            level,
            body: body
                .into_iter()
                .map(|b| resolve_block(b, count, diags))
                .collect(),
        },
        IrBlock::Code { lang, title, src } => RBlock::Code { lang, title, src },
        IrBlock::List { items } => RBlock::List {
            items: items
                .into_iter()
                .map(|it| resolve_list_item(it, count, diags))
                .collect(),
        },
        IrBlock::Rule => RBlock::Rule,
    }
}

fn resolve_list_item(item: IrListItem, count: usize, diags: &mut Vec<String>) -> RListItem {
    RListItem {
        depth: item.depth,
        ordered: item.ordered,
        marker: item.marker,
        body: item
            .body
            .into_iter()
            .map(|b| resolve_block(b, count, diags))
            .collect(),
    }
}

fn resolve_inlines(inlines: Vec<IrInline>, count: usize, diags: &mut Vec<String>) -> Vec<RInline> {
    inlines
        .into_iter()
        .map(|i| resolve_inline(i, count, diags))
        .collect()
}

fn resolve_inline(inline: IrInline, count: usize, diags: &mut Vec<String>) -> RInline {
    match inline {
        Text(t) => RInline::Text(t),
        Ref(target) => {
            let exists = target >= 1 && target <= count;
            if !exists {
                diags.push(format!("chunk ref &{target} points outside 1..={count}"));
            }
            RInline::Ref { target, exists }
        }
        Bold(c) => RInline::Bold(resolve_inlines(c, count, diags)),
        Italic(c) => RInline::Italic(resolve_inlines(c, count, diags)),
        Underline(c) => RInline::Underline(resolve_inlines(c, count, diags)),
        Strike(c) => RInline::Strike(resolve_inlines(c, count, diags)),
        Code { src, lang } => RInline::Code { src, lang },
        Link { url, text } => RInline::Link { url, text },
        Image { src, alt } => RInline::Image { src, alt },
    }
}

fn document_title(ir: &IrDoc) -> Option<String> {
    for chunk in &ir.chunks {
        if let IrBlock::Heading { inlines, .. } = &chunk.kind {
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

fn push_plain_text(inlines: &[IrInline], out: &mut String) {
    for inline in inlines {
        match inline {
            Text(t) => out.push_str(t),
            Bold(c) | Italic(c) | Underline(c) | Strike(c) => push_plain_text(c, out),
            Code { src, .. } => out.push_str(src),
            Link { text, .. } => {
                if text.is_empty() {
                    out.push_str("A link c:")
                } else {
                    out.push_str(text)
                }
            }
            Image { alt, .. } => out.push_str(alt),
            Ref(target) => {
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
        // Le vent se lève !
        //      ... il faut tenter de vivre !
        let (resolved, _) = resolve(doc_with_refs(&[0], 1));
        let RBlock::Paragraph(inlines) = &resolved.chunks.last().unwrap().kind else {
            panic!("expected paragraph");
        };
        assert!(matches!(&inlines[0], RInline::Ref { exists: false, .. }));
    }
}
