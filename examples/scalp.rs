use scalp::{header, Builder, Options};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let result = Builder::new()
        .pipe(header!())
        .usage("Usage: scalp [OPTIONS]")
        .verb(|verb| verb.name("b").name("boba").help("Throws a Boba."))
        .group(|group| group.name("Options:").options(Options::all(true, true)))
        .verb(|verb| verb.name("f").name("fett").help("Catches a Fett."))
        .note("A note.")
        .build()?
        .parse_with(["--help"], [("", "")]);
    if let Err(error) = result {
        println!("{}", error)
    }
    Ok(())
}
