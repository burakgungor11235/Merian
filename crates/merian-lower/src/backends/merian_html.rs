use std::fs::write;

use merian_frontend::ast::*;

use crate::lower::{LowerContext, Lowerer};

pub struct HtmlLowerer;

impl Lowerer for HtmlLowerer {
    type Output = String;

    const VERSION: &'static str = "0.1.0";
    const NAME: &'static str = "html";

    fn lower(&self, ast: &Asp, _ctx: &mut LowerContext) -> Option<Self::Output> {
        let mut out = String::new();

        for (i, ChunkView(inlines, spans)) in ast.iter().enumerate() {
            let block = ast.block_type_of(i);

            // this special block gets special treatment
            if block == BlockType::HRule {
                out.push_str("<hr/>\n");
                continue;
            }
            // middle class
            let (b_open, b_close) = match block {
                BlockType::Paragraph => ("<p>".into(), "</p>\n".into()),
                BlockType::Heading(n) => (format!("<h{n}>"), format!("</h{n}>\n")),
                _ => continue, // Ignore Comment or Unknown
            };

            out.push_str(&b_open);
            // the lower class >:3
            for (&inline, &span) in inlines.iter().zip(spans) {
                let (i_open, i_close) = match inline {
                    Inline::Text => ("", ""),
                    Inline::Bold => ("<strong>", "</strong>"),
                    Inline::Italic => ("<em>", "</em>"),
                    Inline::BoldItalic => ("<strong><em>", "</em></strong>"),
                    Inline::StrikeThru => ("<s>", "</s>"),
                    Inline::Underline => ("<u>", "</u>"),
                };

                out.push_str(i_open);
                out.push_str(ast.span_text(span));
                out.push_str(i_close);
            }

            out.push_str(&b_close);
        }

        Some(out)
    }
}

/// To be replaced with a proper builder.
pub fn build(contents: String, path: String) -> Result<(), std::io::Error> {
    write(path, contents)
}
