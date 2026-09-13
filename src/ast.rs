#[derive(Debug)]
pub struct Document<'a> {
    pub meta: DocumentMetadata,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug)]
pub struct DocumentMetadata {}

#[derive(Debug)]
pub enum Block<'a> {
    Heading {
        level: usize,
        content: Vec<Inline<'a>>,
    },
    Paragraph(Vec<Inline<'a>>),
    Quote {
        level: i32,
        content: Vec<Block<'a>>,
    },
    CodeBlock {
        lang: &'a str,
        title: Option<&'a str>,
        content: &'a str,
    },
    List {
        items: Vec<ListItem<'a>>,
    },
}

#[derive(Debug)]
pub struct ListItem<'a> {
    pub depth: usize,
    pub marker: String, // promote this to an enum in the future.
    pub blocks: Vec<Block<'a>>,
}
#[derive(Debug)]
pub enum Inline<'a> {
    Text(&'a str),

    Bold(Vec<Inline<'a>>),
    Italic(Vec<Inline<'a>>),
    Strikethru(Vec<Inline<'a>>),
    Underline(Vec<Inline<'a>>),
    Code {
        content: &'a str,
        lang: Option<&'a str>,
    },
}
