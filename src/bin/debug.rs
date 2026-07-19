#[path = "../simgit/setup_parser.rs"]
pub mod setup_parser;

use std::path::Path;

fn main() {
    let file = "C:\\Users\\bukar\\opendav\\workspace\\s\\setups\\bmwm4gt3_snetterton 300 2026-04-14 20-49-32.ibt";
    match setup_parser::SetupData::from_ibt_file(Path::new(file)) {
        Ok(setup) => {
            println!("Parsed SetupData with {} params.", setup.parameters.len());
            for (k, v) in &setup.parameters {
                println!("  {}: {}", k, v);
            }
        }
        Err(e) => println!("SetupData error: {}", e),
    }
}
