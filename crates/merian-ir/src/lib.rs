struct Context {
    pool: Vec<Chunk>,
    files: Vec<MerianFile>,
    meta: ContextMeta,
}
struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}
struct ContextMeta {
    version: Version,
}

struct MerianFile {
    chunks: Vec<ChunkRef>,
}

#[derive(Clone)]
struct Chunk {
    chunk_type: ChunkType,
    inline: Vec<InlineEnum>,
}

#[derive(Clone)]
enum ChunkType {
    Paragraph,
}

#[derive(Clone)]
struct InlineEnum {
    inline_type: InlineEnumType,
    endpos: u32,
}

#[derive(Clone)]
enum InlineEnumType {
    Text,
    Bold,
    Italic,
    Strikethrough,
}

#[derive(Clone, Copy)]
struct ChunkRef(usize);

impl<'a> ChunkRef {
    fn get_chunk_ref(&self, ctx: &'a Context) -> &'a Chunk {
        &ctx.pool[self.0]
    }

    // Note clones
    fn get_chunk(&self, ctx: &'a Context) -> Chunk {
        ctx.pool[self.0].clone()
    }
}
