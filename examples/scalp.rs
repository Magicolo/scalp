use scalp::{header, Options, Parser};
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let result = Parser::builder()
        .pipe(header!())
        .usage("Usage: scalp [OPTIONS]")
        .verb(|verb| verb.name("b").name("boba").help("Throws a Boba."))
        .verb(|verb| verb.name("f").name("fett").help("Catches a Fett."))
        .line()
        .group(|group| group.name("Options:").options(Options::all(true, true)))
        .note("A note.")
        .build()?
        .parse_with(["--help"], [("", "")]);
    if let Err(error) = result {
        println!("{}", error)
    }
    Ok(())
}
