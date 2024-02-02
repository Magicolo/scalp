# Scalp

A declarative, macro-less parsing library for command-line interfaces. It provides a highly composable and extensible `Parse` trait that ensures comparative performance to a macro-full approach while offering greater flexibility and understandability. 

*Less magic, more control, same speed.*

## Getting Started
```rust
use scalp::{Builder, Case, Error};

fn main() -> Result<(), Error> {
    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        Run,
        Show,
    }

    struct Root {
        debug: bool,
        command: Command,
    }

    let parser = Builder::new()
        .case(Case::Kebab { upper: false })
        .option(|option| option.name("debug").name("d").default(false))
        .group(|group| group
            .verb(|verb| verb.name("run").map(|_| Command::Run))
            .verb(|verb| verb.name("show").map(|_| Command::Show))
            .any_or("Missing command.")
        )
        .map(|(debug, command)| Root { debug, command })
        .build()?;

    let root = parser.parse_with(["run", "-d"], [("", "")])?;
    assert_eq!(root.command, Command::Run);
    assert!(root.debug);
    Ok(())
}

```