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
    let commands = Parser::builder()
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
                .many::<_, Vec<_>>()
                .require_with("command")
        })
        .line()
        .group(|group| group.name("Options:").options(Options::all(true, true)))
        .map(|(commands, _)| commands)
        .note("A note.")
        .build()?
        .parse()?;
    println!("{:?}", commands);
    Ok(())
}
