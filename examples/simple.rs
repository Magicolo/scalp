use scalp::*;
use std::fs;

fn main() -> Result<(), Error> {
    #[derive(Debug, PartialEq, Eq)]
    enum Command {
        Run {
            settings: Option<String>,
            path: String,
        },
        Show,
    }

    struct Root {
        debug: bool,
        yes: bool,
        force: bool,
        recurse: bool,
        command: Command,
    }
    let a = option::<bool>()
        .name("d")?
        .name("debug")?
        .help("Debug mode.")
        .default(false);
    let b = a.parse()?;

    let parser = verb()
        .case(Case::Kebab { upper: false })
        .node(
            option::<bool>()
                .name("d")?
                .name("debug")?
                .help("Debug mode.")
                .default(false),
        )
        .node(
            option::<bool>()
                .name("y")?
                .name("yes")?
                .swizzle()
                .default(false),
        )
        .node(
            option::<bool>()
                .name("f")?
                .name("force")?
                .swizzle()
                .default(false),
        )
        .node(
            option::<bool>()
                .name("r")?
                .name("recurse")?
                .swizzle()
                .default(false),
        )
        .options([Options::version(true, true), Options::help(true, true)])
        .node(
            group()
                .node(
                    verb()
                        .name("run")?
                        .usage("example run <file> [OPTIONS]")
                        .node(option::<String>().name("file")?.position().require())
                        .node(
                            option::<String>()
                                .name("s")?
                                .name("settings")?
                                .map(|path| fs::read_to_string(path?).ok()),
                        )
                        .map(|(file, settings)| Command::Run {
                            path: file,
                            settings,
                        }),
                )
                .node(verb().name("show")?.map(|_| Command::Show))
                // .any()
                .require(),
        )
        .map(|(debug, yes, force, recurse, command)| Root {
            debug,
            yes,
            recurse,
            force,
            command,
        })
        .line()
        .note("Documentation: https://docs.rs/scalp/latest/scalp/");

    let root = parser.parse_with(["--debug", "-fyr", "run", "./", "-s", "./settings.json"], [
        ("", ""),
    ])?;
    assert!(root.debug);
    assert!(root.force);
    assert!(root.yes);
    assert!(root.recurse);

    let Command::Run { path, .. } = root.command else {
        panic!();
    };
    assert_eq!(path, "./");
    Ok(())
}
