use scalp::{header, Options, Parser};
use std::error;

#[derive(Debug)]
pub struct Jango {
    pub debug: bool,
}

#[derive(Debug)]
pub enum Command {
    Boba,
    Fett { jango: Option<Jango> },
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let command = Parser::builder()
        .pipe(header!())
        .usage("Usage: scalp [OPTIONS]")
        .group(|group| {
            group
                .verb(|verb| {
                    verb.name("b")
                        .name("boba")
                        .summary("Throws a Boba.")
                        .map(|_| Command::Boba)
                })
                .verb(|verb| {
                    verb.name("f")
                        .name("fett")
                        .summary("Catches a Fett.")
                        .verb(|verb| {
                            verb.name("j")
                                .name("jango")
                                .option(|option| option.name("debug").require())
                                .map(|(debug,)| Jango { debug })
                        })
                        .map(|(jango,)| Command::Fett { jango })
                })
                .any::<Command>()
                .require()
        })
        .line()
        .group(|group| group.name("Options:").options(Options::all(true, true)))
        .map(|(command, _)| command)
        .note("A note.")
        .build()?
        .parse()?;
    println!("{:?}", command);
    Ok(())
}
