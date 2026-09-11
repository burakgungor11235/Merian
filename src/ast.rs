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
