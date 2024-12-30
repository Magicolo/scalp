use scalp::{Options, group, header, option, style, verb};
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
    let root = verb()
        .node(header!()?)
        .node(
            group()
                .style(style::Plain)
                .name("Commands")?
                .node(
                    verb()
                        .name("b")?
                        .name("boba")?
                        .summary("Throws a Boba.")
                        .map(|_| Command::Boba),
                )
                .node(
                    verb()
                        .name("f")?
                        .name("fett")?
                        .style(style::Termion)
                        .summary("Catches a Fett.")
                        .node(
                            verb()
                                .name("j")?
                                .name("jango")?
                                .node(option::<bool>().name("debug")?.require())
                                .node(
                                    option::<usize>()
                                        .name("exit")?
                                        .map(Option::unwrap_or_default),
                                )
                                .map(|(debug, exit)| Jango { debug, exit }),
                        )
                        .map(|(jango,)| Command::Fett { jango }),
                )
                .any::<Command>()
                .require_with("command"),
        )
        .node(
            group()
                .name("Options:")
                .child(
                    option()
                        .position()
                        .name("iterations")?
                        .help("The number of iterations.")
                        .require(),
                )
                .child(
                    option()
                        .name("n")?
                        .name("name")?
                        .style(style::Plain)
                        .help("A user display name.")
                        .valid("[a-zA-Z0-9_]+")
                        .default("user"),
                )
                .child(
                    option()
                        .name("t")?
                        .name("tag")?
                        .help("Tags for the user.")
                        .many()
                        .map(Option::unwrap_or_default),
                )
                .child(
                    option()
                        .name("d")?
                        .name("debug")?
                        .help("Enables debug logging.")
                        .environment("SCALP_DEBUG")
                        .swizzle()
                        .map(Option::unwrap_or_default),
                )
                .option(
                    option()
                        .name("v")?
                        .name("verbose")?
                        .help("Enables verbose logging.")
                        .swizzle()
                        .map(Option::unwrap_or_default),
                )
                .options(Options::common(true, true)),
        )
        .map(|(command, (count, name, tags, debug, verbose))| Root {
            command,
            count,
            name,
            tags,
            debug,
            verbose,
        })
        .note("A note.")
        .parse_with(["--help"], [("", "")])?;
    println!("{:?}", root);
    Ok(())
}
