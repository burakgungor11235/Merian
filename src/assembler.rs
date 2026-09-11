use crate::ast::{Block, Document, Inline};
use std::fmt::Write;

// right now it's pretty rudementary. I need a vertical slice first.

pub struct Assembler {
    buffer: String,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Consumes the Assembler and generates the final HTML string.
    pub fn assemble(mut self, doc: &Document) -> String {
        for block in &doc.blocks {
            self.render_block(block);
            self.buffer.push('\n');
        }

        self.buffer
    }

    fn render_block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, content } => {
                let safe_level = level.clamp(&1, &6);

                let _ = write!(self.buffer, "<h{}>", safe_level);
                self.render_inlines(content);
                let _ = write!(self.buffer, "</h{}>", safe_level);
            }
            Block::Paragraph(content) => {
                self.buffer.push_str("<p>");
                self.render_inlines(content);
                self.buffer.push_str("</p>");
            }
        }
    }

    fn render_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            self.render_inline(inline);
        }
    }

    fn render_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text(text) => self.escape_html(text),
            Inline::Bold(content) => {
                self.buffer.push_str("<strong>");
                self.render_inlines(content);
                self.buffer.push_str("</strong>");
            }
            Inline::Italic(content) => {
                self.buffer.push_str("<em>");
                self.render_inlines(content);
                self.buffer.push_str("</em>");
            }
            Inline::Underline(content) => {
                self.buffer.push_str("<u>");
                self.render_inlines(content);
                self.buffer.push_str("</u>");
            }
            Inline::Strikethru(content) => {
                self.buffer.push_str("<del>");
                self.render_inlines(content);
                self.buffer.push_str("</del>");
            }
            Inline::Code { content, lang } => {
                if let Some(l) = lang {
                    let _ = write!(self.buffer, "<code class=\"code-lang-{}\">", l);
                } else {
                    self.buffer.push_str("<code>");
                }
                self.escape_html(content);
                self.buffer.push_str("</code>");
            }
        }
    }

    /// DDSS principle
    /// Don't do stupid shit. Escape your text.
    fn escape_html(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '<' => self.buffer.push_str("&lt;"),
                '>' => self.buffer.push_str("&gt;"),
                '&' => self.buffer.push_str("&amp;"),
                '"' => self.buffer.push_str("&quot;"),
                '\'' => self.buffer.push_str("&#39;"),
                _ => self.buffer.push(c),
            }
        }
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
