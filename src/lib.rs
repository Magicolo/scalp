use crate as scalp;
use anyhow::anyhow;
use scalp_macro::Parse;
use std::{
    borrow::Cow,
    collections::HashMap,
    convert::Infallible,
    env::{self, Args},
    error,
    fmt::Write,
    iter::Peekable,
    str::FromStr,
};

/*
    TODO:
    - Support for help with #[help(text)].
        - Maybe the #[help] attribute generates a doc string?
        - Allow on fields and structs.
    - Support aliases with #[alias(names...)].
        - Maybe the #[alias] attribute generates a doc string?
    - Support default values with #[default(value?)].
        - Maybe the #[default] attribute generates a doc string?
        - #[default] uses 'Default::default'.
        - #[default(value)] uses 'TryFrom::try_from(value)'.
    - Support environment variables with #[environment(variables...)].
        - Maybe the #[environment] attribute generates a doc string?
    - Support for #[omit(help)]
    - Support for #[version] (uses the cargo version) or #[version(version)] (explicit version).
        - Only add the version option if the version attribute is supplied.
    - Autocomplete?
    Add support for combined flags using the short names when possible.
        - Short names must be of length 1.
        - ex: ls -l -a -r -t => ls -lart


    fn parse(parser: Parser) -> Result<Self> {
        let general = parser.flat();
        let command = parser.;
        Ok(Root { general: general.parse()?, command: command.parse()? })
    }

    trait Meta {}
    trait Parse<T> {}
    struct Parser<T, M: Meta, P: Parse<T>> {
        meta: M,
        parse: P,
        _marker: PhantomData<T>,
    }

    trait Parse {

    }

    impl Parse for Root {
        fn parse(parser: Parser) -> Result<Self, ?> {

        }
    }


    Command::build()
        .help("A self-sufficient runtime for containers.")
        .any(|build| (
            build.argument(|build| build.name("Common Commands")),
            build.argument(|build| build.name("Management Commands")),
            build.argument(|build| build.name("Swarm Commands")),
            build.argument(|build| build.name("Commands"))
        ))
        .argument(|build| build.parse())
        .option(|build|)
        .build(|(general, command)| Root { general, command });
        .with(|build| Root {
            general: Global {
                config: build.name("config").parse(),
                context: build
                    .name("context")
                    .name("c")
                    .environment("DOCKER_HOST")
                    .help("Name of the context to use to connect to the daemon (overrides DOCKER_HOST env var and default context set with "docker context use")")
                    .parse(),
                debug: build.name("debug").parse(),
                log_level: build.name("log-level").parse(),
                tlscacert: build.name("tlscacert").default("~/.docker/ca.pem").parse(),
            }
        })
*/

pub enum Error {
    Help { render: fn(&mut dyn Write) },
    Version { render: fn(&mut dyn Write) },
    MissingOptionValue,
    MissingRequiredArgument { name: String, index: usize },
    UnrecognizedArgument { name: String },
    Parse(Box<dyn error::Error>),
    UnrecognizedOption { name: String },
}

pub trait Parse: Sized {
    fn parse(context: &mut Context) -> Result<Self, Error>;
}

pub struct Context {
    pub arguments: Arguments,
    pub environment: Environment,
}

pub struct Arguments(Vec<String>);

pub struct Environment(HashMap<String, String>);

impl Arguments {
    pub fn new(arguments: impl IntoIterator<Item = String>) -> Self {
        Self(arguments.into_iter().collect())
    }
}

impl Default for Arguments {
    fn default() -> Self {
        Self::new(env::args())
    }
}

impl Environment {
    pub fn new(variables: impl IntoIterator<Item = (String, String)>) -> Self {
        Self(variables.into_iter().collect())
    }

    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    pub fn set(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self(env::vars().collect())
    }
}

#[derive(Parse)]
struct Docker {
    config: Option<String>,
    // #[alias("c")]
    // #[environment("DOCKER_HOST")]
    // #[help("Name of the context to use to connect to the daemon (overrides DOCKER_HOST env var and default context set with "docker context use")")]
    /// Name of the context to use to connect to the daemon (overrides DOCKER_HOST env var and default context set with "docker context use")"
    context: Option<String>,
    // #[default] -> Should be implicit.
    debug: bool,
    // #[default]
    log_level: LogLevel,
    // #[default("~/.docker/ca.pem")]
    tlscacert: String,

    // #[flat] #[argument]
    command: Command,
}

enum Command {
    Run,
    Exec,
    Ps,
    Builder,
    Buildx,
    Compose,
    Swarm,
}

#[derive(Default)]
enum LogLevel {
    Debug = 1,
    #[default]
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl FromStr for LogLevel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "debug" => Ok(LogLevel::Debug),
            "2" | "info" => Ok(LogLevel::Info),
            "3" | "warn" => Ok(LogLevel::Warn),
            "4" | "error" => Ok(LogLevel::Error),
            "5" | "fatal" => Ok(LogLevel::Fatal),
            _ => Err(Error::Parse(format!("Invalid log level '{s}'.").into())),
        }
    }
}

impl<E: error::Error + 'static> From<E> for Error {
    fn from(error: E) -> Self {
        Error::Parse(Box::new(error))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Same,
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    Kebab,
    Scream,
}

fn help_docker(writer: &mut dyn Write) {
    writer.write_str("Usage:  docker [OPTIONS] COMMAND");
    writer.write_char('\n');
    writer.write_str("A self-sufficient runtime for containers.");
    writer.write_char('\n');
}
fn version_docker(writer: &mut dyn Write) {
    writer.write_str("Docker version 24.0.7-1, build afdd53b4e341be38d2056a42113b938559bb1d94");
    writer.write_char('\n');
}
#[deny(unreachable_patterns)] // Causes a compile error for duplicate names :)
fn parse_docker(
    arguments: &mut Peekable<impl Iterator<Item = String>>,
    environment: &HashMap<String, String>,
    case: Case,
) -> Result<Docker, Error> {
    let mut config = None;
    let mut context = None;
    let mut debug = None;
    let mut log_level = None;
    let mut tlscacert = None;
    let mut command = None;

    while let Some(argument) = arguments.next() {
        match (argument.as_str(), case) {
            ("-h", _)
            | ("--help", Case::Same | Case::Camel | Case::Kebab | Case::Lower | Case::Snake)
            | ("--Help", Case::Pascal)
            | ("--HELP", Case::Scream | Case::Upper) => {
                return Err(Error::Help {
                    render: help_docker,
                })
            }
            ("--version" | "-v", _) => {
                return Err(Error::Version {
                    render: version_docker,
                })
            }
            ("--config", _) => match arguments.next() {
                Some(value) => config = Some(value.parse()?),
                None => return Err(Error::MissingOptionValue),
            },
            ("--context" | "-c", _) => match arguments.next() {
                Some(value) => context = Some(value.parse()?),
                None => return Err(Error::MissingOptionValue),
            },
            ("--debug" | "D", _) => match arguments.peek() {
                Some(value) => match value.parse() {
                    Ok(value) => {
                        debug = Some(value);
                        arguments.next();
                    }
                    Err(_) => debug = Some(true),
                },
                None => debug = Some(true),
            },
            ("--log_level", Case::Same | Case::Snake)
            | ("--loglevel", Case::Lower)
            | ("--LOGLEVEL", Case::Upper)
            | ("--log-level", Case::Kebab)
            | ("--LOG_LEVEL", Case::Scream)
            | ("logLevel", Case::Camel)
            | ("LogLevel", Case::Pascal) => match arguments.next() {
                Some(value) => log_level = Some(value.parse()?),
                None => return Err(Error::MissingOptionValue),
            },
            ("--tlscacert", _) => match arguments.next() {
                Some(value) => tlscacert = Some(value.parse()?),
                None => return Err(Error::MissingOptionValue),
            },
            ("run", _) => {
                command = Some(parse_run_command(arguments, environment)?);
                break;
            }
            ("exec", _) => {
                command = Some(parse_exec_command(arguments, environment)?);
                break;
            }
            ("swarm", _) => {
                command = Some(parse_swarm_command(arguments, environment)?);
                break;
            }
            // TODO: Check is provided argument is similar to an existing option/verb.
            _ => return Err(Error::UnrecognizedArgument { name: argument }),
        }
    }

    Ok(Docker {
        config: match config {
            Some(value) => Some(value),
            None => Some("$HOME/.docker".parse()?),
        },
        context: match context {
            Some(value) => Some(value),
            None => match environment.get("DOCKER_HOST") {
                Some(value) => Some(value.parse()?),
                None => None,
            },
        },
        debug: match debug {
            Some(value) => value,
            None => bool::default().try_into()?,
        },
        log_level: match log_level {
            Some(value) => value,
            None => LogLevel::default().try_into()?,
        },
        tlscacert: match tlscacert {
            Some(value) => value,
            None => "~/.docker/ca.pem".try_into()?,
        },
        command: match command {
            Some(value) => value,
            None => {
                return Err(Error::MissingRequiredArgument {
                    name: "command".into(),
                    index: 0,
                })
            }
        },
    })
}

fn parse_run_command(
    arguments: &mut Peekable<impl Iterator<Item = String>>,
    environment: &HashMap<String, String>,
) -> Result<Command, Error> {
    unimplemented!()
}
fn parse_exec_command(
    arguments: &mut Peekable<impl Iterator<Item = String>>,
    environment: &HashMap<String, String>,
) -> Result<Command, Error> {
    unimplemented!()
}
fn parse_swarm_command(
    arguments: &mut Peekable<impl Iterator<Item = String>>,
    environment: &HashMap<String, String>,
) -> Result<Command, Error> {
    unimplemented!()
}
