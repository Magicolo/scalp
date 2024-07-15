use scalp::{header, Options, Parser};
use std::error;

#[derive(Debug)]
pub struct Jango {
    pub debug: bool,
    pub exit: usize,
}

#[derive(Debug)]
pub enum Command {
    Boba,
    Fett { jango: Option<Jango> },
}

#[derive(Debug)]
pub struct Root {
    pub command: Command,
    pub count: usize,
    pub name: String,
    pub tags: Vec<String>,
    pub debug: bool,
    pub verbose: bool,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let root = Parser::builder()
        .pipe(header!())
        .group(|group| {
            group
                .name("Commands")
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
                                .option(|option| option.name("exit").map(Option::unwrap_or_default))
                                .map(|(debug, exit)| Jango { debug, exit })
                        })
                        .map(|(jango,)| Command::Fett { jango })
                })
                .any::<Command>()
                .require_with("command")
        })
        .group(|group| {
            group
                .name("Options:")
                .option(|option| {
                    option
                        .position()
                        .name("iterations")
                        .help("The number of iterations.")
                        .require()
                })
                .option(|option| {
                    option
                        .name("n")
                        .name("name")
                        .help("A user display name.")
                        .valid("[a-zA-Z0-9_]+")
                        .default("user")
                })
                .option(|option| {
                    option
                        .name("t")
                        .name("tag")
                        .help("Tags for the user.")
                        .many()
                        .map(Option::unwrap_or_default)
                })
                .option(|option| {
                    option
                        .name("d")
                        .name("debug")
                        .help("Enables debug logging.")
                        .environment("SCALP_DEBUG")
                        .swizzle()
                        .map(Option::unwrap_or_default)
                })
                .option(|option| {
                    option
                        .name("v")
                        .name("verbose")
                        .help("Enables verbose logging.")
                        .swizzle()
                        .map(Option::unwrap_or_default)
                })
                .options(Options::common(true, true))
        })
        .map(|(command, (count, name, tags, debug, verbose))| Root {
            command,
            count,
            name,
            tags,
            debug,
            verbose,
        })
        .note("A note.")
        .build()?
        .parse_with(["--help"], [("", "")])?;
    println!("{:?}", root);
    Ok(())
}
