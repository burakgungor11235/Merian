use giallo::{HighlightOptions, HtmlRenderer, Registry, RenderOptions, ThemeVariant};
use std::{fmt::Write, process::exit};

use crate::backend::rir::{RBlock, RInline, RListItem, ResolvedDoc};

const STYLE: &str = include_str!("data/merian-style.css");

/// Any backend consumes the resolved IR and produces its own output.
pub trait Backend {
    type Out;
    fn emit(self, doc: &ResolvedDoc) -> Self::Out;
}

pub struct Assembler {
    buffer: String,
    syntax_reg: Registry,
}

impl Assembler {
    pub fn new() -> Self {
        let mut registry = match Registry::builtin() {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "Merian::Assembler failed because of an error in syntax registry creation:\n{}",
                    e
                );
                exit(-1); // propagate this properly in the future.
                // _yeet_
            }
        };

        registry.link_grammars();
        Self {
            buffer: String::new(),
            syntax_reg: registry,
        }
    }

    pub fn assemble(mut self, doc: &ResolvedDoc) -> String {
        self.buffer.push_str("<!doctype html>\n");
        self.buffer.push_str("<html lang=\"en\">\n<head>\n");
        self.buffer.push_str("<meta charset=\"utf-8\">\n");
        self.buffer
            .push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
        self.buffer.push_str("<title>");
        self.escape_html(&doc.title);
        self.buffer.push_str("</title>\n<style>\n");
        self.buffer.push_str(STYLE);
        self.buffer.push_str("\n</style>\n</head>\n<body>\n");
        self.buffer
            .push_str("<a class=\"skip-link\" href=\"#main\">Skip to content</a>\n");
        self.buffer.push_str("<main id=\"main\">\n");

        for chunk in &doc.chunks {
            writeln!(
                self.buffer,
                "<section id=\"{}\" class=\"merian-chunk\" data-chunk=\"{}\">",
                chunk.id, chunk.id
            )
            .unwrap();
            self.render_block(&chunk.kind);
            self.buffer.push_str("</section>\n\n");
        }

        self.buffer.push_str("</main>\n");
        self.buffer.push_str("</body>\n</html>\n");

        std::mem::take(&mut self.buffer)
    }

    fn render_block(&mut self, block: &RBlock) {
        match block {
            RBlock::Heading {
                level,
                permalink,
                inlines,
            } => {
                write!(self.buffer, "<h{level} id=\"heading-{permalink}\">").unwrap();
                self.render_inlines(inlines);
                write!(
                    self.buffer,
                    "<a class=\"anchor\" href=\"#heading-{permalink}\" aria-label=\"Permalink to section {permalink}\">#</a>",
                )
                .unwrap();
                writeln!(self.buffer, "</h{level}>").unwrap();
            }
            RBlock::Paragraph(content) => {
                self.buffer.push_str("<p>");
                self.render_inlines(content);
                self.buffer.push_str("</p>\n");
            }
            RBlock::Quote { level, body } => self.render_quote(*level, body),
            RBlock::Code { lang, title, src } => {
                self.render_code_block(lang, title.as_deref(), src);
            }
            RBlock::List { items } => {
                self.render_list(items);
            }
            RBlock::Rule => self.buffer.push_str("<hr>\n"),
        }
    }

    fn render_quote(&mut self, level: u8, content: &[RBlock]) {
        writeln!(
            self.buffer,
            "<blockquote class=\"merian-quote\" data-level=\"{level}\">"
        )
        .unwrap();

        for block in content {
            self.render_block(block);
        }

        self.buffer.push_str("</blockquote>\n");
    }

    fn render_code_block(&mut self, lang: &str, title: Option<&str>, content: &str) {
        self.buffer.push_str("<figure class=\"code-block\">\n");

        if title.is_some() || !lang.is_empty() {
            self.buffer.push_str("<figcaption class=\"code-title\">");

            if let Some(t) = title {
                self.buffer.push_str("<span class=\"code-name\">");
                self.escape_html(t);
                self.buffer.push_str("</span>");
            }

            if !lang.is_empty() {
                self.buffer.push_str("<span class=\"code-lang-tag\">");
                self.escape_html(lang);
                self.buffer.push_str("</span>");
            }

            self.buffer.push_str("</figcaption>\n");
        }

        self.buffer.push_str("<pre tabindex=\"0\"><code");
        if !lang.is_empty() {
            self.buffer.push_str(" class=\"code-lang-");
            self.escape_html(lang);
            self.buffer.push('"');
        }
        self.buffer.push('>');
        self.buffer.push_str(&self.render_code(lang, content));
        self.buffer.push_str("</code></pre>\n</figure>\n");
    }

    fn render_list(&mut self, items: &[RListItem]) {
        struct Level {
            depth: usize,
            ordered: bool,
            li_open: bool,
        }
        impl Level {
            fn tag(&self) -> &str {
                if self.ordered { "ol" } else { "ul" }
            }
        }

        fn close_item(buf: &mut String, level: &mut Level) {
            if std::mem::take(&mut level.li_open) {
                buf.push_str("</li>\n");
            }
        }
        fn close_level(buf: &mut String, mut level: Level) {
            close_item(buf, &mut level);
            writeln!(buf, "</{}>", level.tag()).unwrap();
        }

        let mut stack: Vec<Level> = Vec::new();

        for item in items {
            let depth = item.depth;
            let ordered = item.ordered;

            while stack
                .last()
                .is_some_and(|l| l.depth > depth || (l.depth == depth && l.ordered != ordered))
            {
                close_level(&mut self.buffer, stack.pop().unwrap());
            }

            if stack.last().is_none_or(|l| l.depth < depth) {
                writeln!(
                    self.buffer,
                    "<{} class=\"merian-list {}\" role=\"list\">",
                    if ordered { "ol" } else { "ul" },
                    if ordered { "auto" } else { "bullet" },
                )
                .unwrap();
                stack.push(Level {
                    depth,
                    ordered,
                    li_open: false,
                });
            }

            let top = stack.last_mut().unwrap();
            close_item(&mut self.buffer, top);
            top.li_open = true;

            writeln!(
                self.buffer,
                "<li class=\"merian-list-item\" data-depth=\"{depth}\" role=\"listitem\">"
            )
            .unwrap();

            if ordered {
                self.buffer.push_str("<span class=\"marker\">");
                self.escape_html(&item.marker);
                self.buffer.push_str("</span>\n");
            } else {
                self.buffer
                    .push_str("<span class=\"marker\" aria-hidden=\"true\">\u{2217}</span>\n");
            }

            self.buffer.push_str("<div class=\"content\">\n");
            for block in &item.body {
                self.render_block(block);
            }
            self.buffer.push_str("</div>\n");
        }

        while let Some(level) = stack.pop() {
            close_level(&mut self.buffer, level);
        }
    }

    fn render_inlines(&mut self, inlines: &[RInline]) {
        for inline in inlines {
            self.render_inline(inline);
        }
    }

    fn render_inline(&mut self, inline: &RInline) {
        match inline {
            RInline::Text(text) => self.escape_html(text),
            RInline::Bold(content) => {
                self.buffer.push_str("<strong>");
                self.render_inlines(content);
                self.buffer.push_str("</strong>");
            }
            RInline::Italic(content) => {
                self.buffer.push_str("<em>");
                self.render_inlines(content);
                self.buffer.push_str("</em>");
            }
            RInline::Underline(content) => {
                self.buffer.push_str("<u>");
                self.render_inlines(content);
                self.buffer.push_str("</u>");
            }
            RInline::Strike(content) => {
                self.buffer.push_str("<del>");
                self.render_inlines(content);
                self.buffer.push_str("</del>");
            }
            RInline::Code { src, lang } => {
                self.buffer.push_str("<code style=\"white-space: pre;\"");
                if let Some(lang) = lang {
                    // I think we are vulnarable here.
                    self.buffer.push_str(" class=\"code-lang-");
                    self.escape_html(lang);
                    self.buffer.push('"');
                }
                self.buffer.push('>');

                let lang_tag = lang.as_deref().unwrap_or("txt");
                self.buffer.push_str(&self.render_code_inner(lang_tag, src));

                self.buffer.push_str("</code>");
            }
            RInline::Link { url, text } => {
                self.buffer.push_str("<a href=");
                self.escape_html(url); // here too.
                self.buffer.push_str(">\n");

                self.escape_html(text);
                self.buffer.push_str("</a>");
            }
            RInline::Image { src, alt } => {
                self.buffer.push_str("<img src=\"");
                self.escape_html(src);
                self.buffer.push_str("\" alt=\"");
                self.escape_html(alt);
                self.buffer.push_str("\">");
            }
            RInline::Ref { target, exists } => {
                if *exists {
                    write!(
                        self.buffer,
                        "<a class=\"chunk-ref\" href=\"#{target}\">&{target}</a>"
                    )
                    .unwrap();
                } else {
                    write!(
                        self.buffer,
                        "<span class=\"broken-ref\" data-target=\"{target}\">&{target}</span>"
                    )
                    .unwrap();
                }
            }
        }
    }

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

    fn render_code_inner(&self, lang: &str, content: &str) -> String {
        let full_html = self.render_code(lang, content);

        // Find the boundary after Giallo's opening
        let content_start = full_html
            .find("<code")
            .and_then(|idx| full_html[idx..].find('>').map(|end| idx + end + 1))
            .unwrap_or(0);

        // Find the boundary before Giallo's closing
        let content_end = full_html.rfind("</code>").unwrap_or(full_html.len());

        if content_start < content_end {
            full_html[content_start..content_end].to_string()
        } else {
            full_html
        }
    }

    fn render_code(&self, lang: &str, content: &str) -> String {
        let lang_to_use = if lang.is_empty() { "txt" } else { lang };
        let options = HighlightOptions::new(lang_to_use, ThemeVariant::Single("catppuccin-latte"));

        // Fallback to "txt" if the grammar is not found in the registry
        let highlighted = self
            .syntax_reg
            .highlight(content, &options)
            .or_else(|_| {
                let fallback =
                    HighlightOptions::new("txt", ThemeVariant::Single("catppuccin-latte"));
                self.syntax_reg.highlight(content, &fallback)
            })
            .unwrap();

        HtmlRenderer::default().render(&highlighted, &RenderOptions::default())
    }
}

impl Backend for Assembler {
    type Out = String;
    fn emit(self, doc: &ResolvedDoc) -> String {
        self.assemble(doc)
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
