# Scalp

A declarative, macro-less parsing library for command-line interfaces. It provides a highly composable and extensible `Parse` trait that ensures comparative performance to a macro-full approach while offering greater flexibility and understandability. 

*Less magic, more control, same speed.*

## Getting Started
```rust
use scalp::{Builder, Case, Error, Ok};

fn main() -> Result<(), Error> {
    enum Command {
        Run,
        Show,
    }

    struct Root {
        debug: bool,
        command: Command
    }

    let parser = Builder::new()
        .case(Case::Kebab)
        .root(|build| build
            .option(|build| build.name("debug").default(false))
            .verb(|build| build.name("run").map(|_| Command::Run).ok())?
            .verb(|build| build.name("show").map(|_| Command::Show).ok())?
            .map(|(debug, run, show)| Root { debug, command: ! })
            .ok()
        )?
        .build();

    // Uses `std::env::args()` and `std::env::vars()` by default. See [`Parser::parse_with`] to provide arguments and environment variables manually.
    let root: Root = parser.parse()?; 
}
```