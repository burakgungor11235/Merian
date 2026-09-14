use crate::ast::{Block, Document, Inline, ListMarker};
use std::fmt::Write;

// still pretty rudementary but we're getting there.

const STYLE: &str = include_str!("data/merian-style.css");

pub struct Assembler {
    buffer: String,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Consumes the assembler and generates the final HTML document.
    pub fn assemble(mut self, doc: &Document) -> String {
        self.buffer.push_str("<!doctype html>\n");
        self.buffer.push_str("<html>\n<head>\n");
        self.buffer.push_str("<meta charset=\"utf-8\">\n");
        self.buffer.push_str("<style>\n");
        self.buffer.push_str(STYLE);
        self.buffer.push_str("\n</style>\n");
        self.buffer.push_str("</head>\n<body>\n");

        for block in &doc.blocks {
            self.render_block(block);
            self.buffer.push('\n');
        }

        self.buffer.push_str("</body>\n</html>\n");

        self.buffer
    }

    fn render_block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, content } => {
                let level = level.clamp(&1, &6);

                write!(self.buffer, "<h{}>", level).unwrap();
                self.render_inlines(content);
                writeln!(self.buffer, "</h{}>", level).unwrap();
            }

            Block::Paragraph(content) => {
                self.buffer.push_str("<p>");
                self.render_inlines(content);
                self.buffer.push_str("</p>\n");
            }

            Block::Quote { level, content } => {
                for _ in 0..*level {
                    self.buffer.push_str("<blockquote>\n");
                }

                for block in content {
                    self.render_block(block);
                }

                for _ in 0..*level {
                    self.buffer.push_str("</blockquote>\n");
                }
            }

            Block::CodeBlock {
                lang,
                title,
                content,
            } => {
                if let Some(title) = title {
                    self.buffer.push_str("<div class=\"code-title\">");
                    self.escape_html(title);
                    self.buffer.push_str("</div>\n");
                }

                self.buffer.push_str("<pre><code");

                if !lang.is_empty() {
                    self.buffer.push_str(" class=\"code-lang-");
                    self.escape_html(lang);
                    self.buffer.push('"');
                }

                self.buffer.push('>');
                self.escape_html(content);
                self.buffer.push_str("</code></pre>\n");
            }

            Block::List { items } => {
                self.render_list(items);
            }
        }
    }
    fn render_list(&mut self, items: &[crate::ast::ListItem]) {
        let unordered = items.iter().all(|item| item.marker.is_unordered());

        let list_type = if unordered {
            match &items.first().unwrap().marker {
                ListMarker::Bullet => "bullet",
                ListMarker::Dash => "dash",
                _ => "ordered",
            }
        } else {
            "ordered"
        };

        let tag = if unordered { "ul" } else { "ol" };

        writeln!(self.buffer, "<{} class=\"merian-list {}\">", tag, list_type).unwrap();

        for item in items {
            writeln!(
                self.buffer,
                "<li class=\"merian-list-item depth-{}\">",
                item.depth
            )
            .unwrap();

            self.buffer.push_str("<span class=\"marker\">");
            self.escape_html(&item.marker.to_string());
            self.buffer.push_str("</span>\n");

            self.buffer.push_str("<div class=\"content\">\n");

            for block in &item.blocks {
                self.render_block(block);
            }

            self.buffer.push_str("</div>\n");
            self.buffer.push_str("</li>\n");
        }

        writeln!(self.buffer, "</{}>", tag).unwrap();
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
                self.buffer.push_str("<code");

                if let Some(lang) = lang {
                    self.buffer.push_str(" class=\"code-lang-");
                    self.escape_html(lang);
                    self.buffer.push('"');
                }

                self.buffer.push('>');
                self.escape_html(content);
                self.buffer.push_str("</code>");
            }
        }
    }

    /// Don't do stupid shit. Escape your text.
    fn escape_html(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.buffer.push_str("&amp;"),
                '<' => self.buffer.push_str("&lt;"),
                '>' => self.buffer.push_str("&gt;"),
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
