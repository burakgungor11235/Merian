/// Basically this is where the fun begins.

#[derive(Debug, Clone, PartialEq)]
pub struct IrDoc {
    pub chunks: Vec<IrChunk>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrChunk {
    /// 1-based source order. Assigned once in `lower()`.
    pub id: usize,
    pub kind: IrBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrBlock {
    Heading {
        level: u8,
        permalink: String,
        inlines: Vec<IrInline>,
    },
    Paragraph(Vec<IrInline>),
    Quote {
        level: u8,
        body: Vec<IrBlock>,
    },
    Code {
        lang: String,
        title: Option<String>,
        src: String,
    },
    List {
        items: Vec<IrListItem>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrListItem {
    pub depth: usize,
    pub ordered: bool,
    pub marker: String,
    pub body: Vec<IrBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrInline {
    Text(String),
    Bold(Vec<IrInline>),
    Italic(Vec<IrInline>),
    Underline(Vec<IrInline>),
    Strike(Vec<IrInline>),
    Code { src: String, lang: Option<String> },
    Link { url: String, text: String },
    Image { src: String, alt: String },
    Ref(usize),
}

pub struct HeadingIds {
    pub counters: Vec<usize>,
}

impl HeadingIds {
    pub fn next(&mut self, level: usize) -> String {
        let level = level.clamp(1, 6);
        if self.counters.len() < level {
            while self.counters.len() < level - 1 {
                self.counters.push(1);
            }
            self.counters.push(1);
        } else {
            self.counters.truncate(level);
            if let Some(last) = self.counters.last_mut() {
                *last += 1;
            }
        }
        self.counters
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
}
