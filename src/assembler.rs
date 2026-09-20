use crate::ast::{Block, Document, Inline};
use std::fmt::Write;

const STYLE: &str = include_str!("data/merian-style.css");

// This puts the ass in assembler

pub struct Assembler {
    buffer: String,
    heading_counters: Vec<usize>,
}

impl Assembler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            heading_counters: Vec::new(),
        }
    }

    pub fn assemble(mut self, doc: &Document) -> String {
        let title = Self::document_title(doc).unwrap_or_else(|| "Merian".to_string());

        self.buffer.push_str("<!doctype html>\n");
        self.buffer.push_str("<html lang=\"en\">\n<head>\n");
        self.buffer.push_str("<meta charset=\"utf-8\">\n");
        self.buffer
            .push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
        self.buffer.push_str("<title>");
        self.escape_html(&title);
        self.buffer.push_str("</title>\n<style>\n");
        self.buffer.push_str(STYLE);
        self.buffer.push_str("\n</style>\n</head>\n<body>\n");
        self.buffer
            .push_str("<a class=\"skip-link\" href=\"#main\">Skip to content</a>\n");
        self.buffer.push_str("<main id=\"main\">\n");

        for block in &doc.blocks {
            self.render_block(block);
            self.buffer.push('\n');
        }

        self.buffer.push_str("</main>\n");
        self.buffer.push_str("</body>\n</html>\n");

        self.buffer
    }

    fn document_title(doc: &Document) -> Option<String> {
        for block in &doc.blocks {
            if let Block::Heading { content, .. } = block {
                let mut s = String::new();
                Self::push_plain_text(content, &mut s);
                let t = s.trim().to_string();
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
        None
    }

    fn push_plain_text(inlines: &[Inline], out: &mut String) {
        for inline in inlines {
            match inline {
                Inline::Text(t) => out.push_str(t),
                Inline::Bold(c)
                | Inline::Italic(c)
                | Inline::Underline(c)
                | Inline::Strikethru(c) => Self::push_plain_text(c, out),
                Inline::Code { content, .. } => out.push_str(content),
                Inline::Link { url: _, text } => {
                    if text.is_empty() {
                        out.push_str("A link..")
                    } else {
                        out.push_str(text)
                    }
                }
            }
        }
    }

    /// Computes the coordinate ID for headings
    /// in the future it should be taken from the IR.
    fn next_heading_permalink(&mut self, level: usize) -> String {
        if self.heading_counters.len() < level {
            while self.heading_counters.len() < level - 1 {
                self.heading_counters.push(1);
            }
            self.heading_counters.push(1);
        } else {
            self.heading_counters.truncate(level);
            if let Some(last) = self.heading_counters.last_mut() {
                *last += 1;
            }
        }

        self.heading_counters
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn render_block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, content } => {
                let level = (*level).clamp(1, 6);
                let permalink = self.next_heading_permalink(level);

                write!(self.buffer, "<h{} id=\"{}\">", level, permalink).unwrap();
                self.render_inlines(content);
                write!(
                    self.buffer,
                    "<a class=\"anchor\" href=\"#{}\" aria-label=\"Permalink to section {}\">#</a>",
                    permalink, permalink
                )
                .unwrap();
                writeln!(self.buffer, "</h{}>", level).unwrap();
            }
            Block::Paragraph(content) => {
                self.buffer.push_str("<p>");
                self.render_inlines(content);
                self.buffer.push_str("</p>\n");
            }
            Block::Quote { level, content } => self.render_quote(*level, content),
            Block::CodeBlock {
                lang,
                title,
                content,
            } => {
                self.render_code_block(lang, *title, content);
            }
            Block::List { items } => {
                self.render_list(items);
            }
            Block::ThematicBreak => self.buffer.push_str("<hr>\n"),
        }
    }

    fn render_quote(&mut self, level: i32, content: &[Block]) {
        let safe_level = level.max(1);

        writeln!(
            self.buffer,
            "<blockquote class=\"merian-quote\" data-level=\"{safe_level}\">"
        )
        .unwrap();

        for block in content {
            self.render_block(block);
        }

        self.buffer.push_str("</blockquote>\n");
    }

    fn render_code_block(&mut self, lang: &str, title: Option<&str>, content: &str) {
        let lang = lang.trim();
        let title = title.map(str::trim).filter(|t| !t.is_empty());

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
        self.escape_html(content);
        self.buffer.push_str("</code></pre>\n</figure>\n");
    }

    fn render_list(&mut self, items: &[crate::ast::ListItem]) {
        struct Level {
            depth: usize,
            ordered: bool,
            li_open: bool,
        }
        impl Level {
            fn tag(&self) -> &'static str {
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
            let ordered = !item.marker.is_unordered();

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
                self.escape_html(&item.marker.to_string());
                self.buffer.push_str("</span>\n");
            } else {
                self.buffer
                    .push_str("<span class=\"marker\" aria-hidden=\"true\">\u{2217}</span>\n");
            }

            self.buffer.push_str("<div class=\"content\">\n");
            for block in &item.blocks {
                self.render_block(block);
            }
            self.buffer.push_str("</div>\n");
        }

        while let Some(level) = stack.pop() {
            close_level(&mut self.buffer, level);
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
                self.buffer.push_str("<code");
                if let Some(lang) = lang {
                    // I think we are vulnarable here.
                    self.buffer.push_str(" class=\"code-lang-");
                    self.escape_html(lang);
                    self.buffer.push('"');
                }
                self.buffer.push('>');
                self.escape_html(content);
                self.buffer.push_str("</code>");
            }
            Inline::Link { url, text } => {
                self.buffer.push_str("<a href=");
                self.buffer.push_str(url); // here too. 
                self.buffer.push_str(">\n");

                self.escape_html(text);
                self.buffer.push_str("</a>");
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
}

impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
