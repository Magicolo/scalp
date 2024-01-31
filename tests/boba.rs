use scalp::{Builder, Case, Error};

#[test]
fn boba() -> Result<(), Error> {
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
        .case(Case::Kebab)
        .option(|option| option.name("debug").name("d").default(false))
        .group(|group| group
            .verb(|verb| verb.name("run").map(|_| Command::Run))
            .verb(|verb| verb.name("show").map(|_| Command::Show))
            .any_or("Missing command.")
        )
        .map(|(debug, command)| Root { debug, command })
        .build()?;

    // Uses `std::env::args()` and `std::env::vars()` by default. See [`Parser::parse_with`]
    // to provide arguments and environment variables manually.
    let root = parser.parse_with(["run", "-d"], [("", "")])?;
    assert_eq!(root.command, Command::Run);
    assert!(root.debug);
    Ok(())
}
