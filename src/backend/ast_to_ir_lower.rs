use crate::ast::{Block, Document, Inline, ListItem};
use crate::backend::ir::*;

/// Lower AST to owned IR.
pub fn lower(doc: &Document) -> IrDoc {
    let mut headings = HeadingIds {
        counters: Vec::new(),
    };
    let chunks = doc
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| IrChunk {
            id: i + 1,
            kind: lower_block(b, &mut headings),
        })
        .collect();
    IrDoc { chunks }
}

fn lower_block(block: &Block, headings: &mut HeadingIds) -> IrBlock {
    match block {
        Block::Heading { level, content } => {
            let level = (*level).clamp(1, 6) as u8;
            let permalink = headings.next(level as usize);
            IrBlock::Heading {
                level,
                permalink,
                inlines: lower_inlines(content),
            }
        }
        Block::Paragraph(inlines) => IrBlock::Paragraph(lower_inlines(inlines)),
        Block::Quote { level, content } => {
            let level = (*level).clamp(1, 255) as u8;
            IrBlock::Quote {
                level,
                body: content.iter().map(|b| lower_block(b, headings)).collect(),
            }
        }
        Block::CodeBlock {
            lang,
            title,
            content,
        } => {
            let lang = lang.trim().to_owned();
            let title = title.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
            IrBlock::Code {
                lang,
                title,
                src: (*content).to_owned(),
            }
        }
        Block::List { items } => IrBlock::List {
            items: items
                .iter()
                .map(|it| lower_list_item(it, headings))
                .collect(),
        },
        Block::ThematicBreak => IrBlock::Rule,
    }
}

fn lower_list_item(item: &ListItem, headings: &mut HeadingIds) -> IrListItem {
    let ordered = !item.marker.is_unordered();
    let marker = item.marker.to_string();
    IrListItem {
        depth: item.depth,
        ordered,
        marker,
        body: item
            .blocks
            .iter()
            .map(|b| lower_block(b, headings))
            .collect(),
    }
}

fn lower_inlines(inlines: &[Inline]) -> Vec<IrInline> {
    inlines.iter().map(lower_inline).collect()
}

fn lower_inline(inline: &Inline) -> IrInline {
    match inline {
        Inline::Text(t) => IrInline::Text((*t).to_owned()),
        Inline::ChunkRef { target } => IrInline::Ref(*target),
        Inline::Bold(c) => IrInline::Bold(lower_inlines(c)),
        Inline::Italic(c) => IrInline::Italic(lower_inlines(c)),
        Inline::Underline(c) => IrInline::Underline(lower_inlines(c)),
        Inline::Strikethru(c) => IrInline::Strike(lower_inlines(c)),
        Inline::Code { content, lang } => IrInline::Code {
            src: (*content).to_owned(),
            lang: lang.map(|l| l.to_owned()),
        },
        Inline::Link { url, text } => IrInline::Link {
            url: url.clone().into_owned(),
            text: text.clone().into_owned(),
        },
        Inline::Image { img_source, alt } => IrInline::Image {
            src: img_source.clone().into_owned(),
            alt: alt.clone().into_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headings(levels: &[usize]) -> Vec<String> {
        let mut h = HeadingIds {
            counters: Vec::new(),
        };
        levels.iter().map(|l| h.next(*l)).collect()
    }

    #[test]
    fn permalink_sequence() {
        assert_eq!(headings(&[1, 1, 2]), vec!["1", "2", "2.1"]);
    }

    #[test]
    fn heading_level_clamped_in_lower() {
        let doc = Document {
            meta: crate::ast::DocumentMetadata {},
            blocks: vec![
                Block::Heading {
                    level: 99,
                    content: vec![],
                },
                Block::Quote {
                    level: -3,
                    content: vec![],
                },
            ],
        };
        let ir = lower(&doc);
        assert!(matches!(
            &ir.chunks[0].kind,
            IrBlock::Heading { level: 6, .. }
        ));
        assert!(matches!(
            &ir.chunks[1].kind,
            IrBlock::Quote { level: 1, .. }
        ));
    }
}
