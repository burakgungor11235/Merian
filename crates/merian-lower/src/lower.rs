/*!
 THIS IS A WIP
 normally this needs to get a concluded IR, but since we don't need one right now and are
 trying to get a vertical slice, I'm just going to make a plain old asp serializer and then add
 the need for an IR (functions / directives) and then do the IR and then come back. 
*/

use merian_frontend::ast::Asp;


trait Lowerer {
    type Output; 

    const VERSION: &'static str;
    const NAME: &'static str;

    fn lower(&self, ast: &Asp) -> Result<Self::Output, Error>;
}

trait Builder {
    type Artifact;

    fn build(
        &mut self,
        artifact: Self::Artifact,
    ) -> Result<(), Error>;
}
