use scalp::{header, Builder, Options};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let result = Builder::new()
        .pipe(header!())
        .help("")
        .options(Options::all(true, true))
        .build()?
        .parse_with(["--help"], [("", "")]);
    if let Err(error) = result {
        println!("{}", error)
    }
    Ok(())
}
