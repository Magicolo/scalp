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

#[derive(Debug)]
pub struct Root {
    pub commands: Vec<Command>,
    pub kroule: String,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let root = Parser::builder()
        .pipe(header!())
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
        .group(|group| {
            group
                .name("Options:")
                .option(|option| option.name("kroule").require())
                .options(Options::all(true, true))
        })
        .map(|(commands, (kroule,))| Root { commands, kroule })
        .note("A note.")
        .build()?
        // .parse_with(["fett", "j", "--help"], [("", "")])?;
        .parse()?;
    println!("{:?}", root);
    Ok(())
}
