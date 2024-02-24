<div align="center"> <h1> scalp </h1> </div>

<p align="center">
    <em> A declarative parsing library for command-line interfaces. It provides a highly composable and extensible `Parse` trait that ensures comparative performance to a macro-full approach while offering greater flexibility and understandability.

*Less magic, more control, same speed.* </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/scalp/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/scalp/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/scalp"> <img src="https://img.shields.io/crates/v/scalp.svg"> </a>
</div>
<p/>

---

## Getting Started
```rust
use std::fs;
use scalp::*;

fn main() -> Result<(), Error> {
    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        Run { settings: Option<String>, path: String },
        Show,
    }

    struct Root {
        debug: bool,
        yes: bool,
        force: bool,
        recurse: bool,
        command: Command,
    }

    let parser = Parser::builder()
        .case(Case::Kebab { upper: false })
        .option(|option| option.name("d").name("debug").help("Debug mode.").default(false))
        .option(|option| option.name("y").name("yes").swizzle().default(false))
        .option(|option| option.name("f").name("force").swizzle().default(false))
        .option(|option| option.name("r").name("recurse").swizzle().default(false))
        .options([Options::version(true, true), Options::help(true, true)])
        .group(|group| group
            .verb(|verb| verb.name("run")
                .usage("example run [OPTIONS]")
                .option(|option| option.position().require())
                .option(|option| option.name("s").name("settings").parse::<String>().map(|path| fs::read_to_string(path?).ok()))
                .map(|(file, settings)| Command::Run { path: file, settings }))
            .verb(|verb| verb.name("show").map(|_| Command::Show))
            .any_or("Missing command.")
        )
        .map(|(debug, yes, force, recurse, command)| Root { debug, yes, recurse, force, command })
        .line()
        .note("Documentation: https://docs.rs/scalp/latest/scalp/")
        .build()?;

    let root = parser.parse_with(["--debug", "-fyr", "run", "./", "-s", "./settings.json"], [("", "")])?;
    assert!(root.debug);
    assert!(root.force);
    assert!(root.yes);
    assert!(root.recurse);

    let Command::Run { path, .. } = root.command else { panic!(); };
    assert_eq!(path, "./");
    Ok(())
}
```