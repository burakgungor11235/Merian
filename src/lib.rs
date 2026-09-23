pub mod assembler;
pub mod ast;
pub mod backend;
pub mod lexer;
pub mod parser;

pub use backend::rir::ResolvedDoc as MerianIR;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_pipeline_preserves_basic_document() {
        let document = parser::parse("Hello");
        let ir = backend::ast_to_ir_lower::lower(&document);
        let (resolved, diagnostics) = backend::resolver::resolve(ir);
        let _: &MerianIR = &resolved;

        assert_eq!(resolved.title, "Merian");
        assert!(diagnostics.is_empty());

        let html = assembler::Backend::emit(assembler::Assembler::default(), &resolved);
        assert!(html.contains("<p>Hello</p>"));
    }
}
