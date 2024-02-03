use scalp::{cargo, Builder, Options};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let result = Builder::new()
        .pipe(cargo!())
        .help("")
        .options(Options::all(true, true))
        .build()?
        .parse_with(["--help"], [("", "")]);
    if let Err(error) = result {
        println!("{}", error)
    }
    Ok(())
}
