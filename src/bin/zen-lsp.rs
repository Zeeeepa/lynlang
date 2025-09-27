use std::error::Error;
use zen::lsp::ZenLanguageServer;

fn main() -> Result<(), Box<dyn Error>> {
    // Create and run the enhanced Zen Language Server
    let server = ZenLanguageServer::new()?;
    server.run()?;
    Ok(())
}