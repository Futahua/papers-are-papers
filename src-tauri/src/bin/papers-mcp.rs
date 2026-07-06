#[path = "../mcp.rs"]
mod mcp;

use std::io::{self, BufReader};
use std::path::PathBuf;

fn main() {
    let Some(root) = std::env::var_os("PAPERS_STAGING_ROOT").map(PathBuf::from) else {
        eprintln!("PAPERS_STAGING_ROOT is required");
        std::process::exit(2);
    };
    if !root.is_dir() {
        eprintln!("Papers staging root does not exist: {}", root.display());
        std::process::exit(2);
    }
    mcp::serve(BufReader::new(io::stdin()), io::stdout(), root);
}
