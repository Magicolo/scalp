use std::{str::FromStr, any::type_name, borrow::Cow};
use scalp::{Error, Builder, Ok};

pub struct Root {
    pub global: GlobalOptions,
    pub command: Command,
}

pub struct GlobalOptions {
    pub config: String,
    pub context: Option<String>,
    pub debug: bool,
    pub host: Vec<String>,
    pub log_level: LogLevel,
}

pub enum Command {
    Attach {
        detach_keys: Option<String>,
        no_stdin: bool,
        sig_proxy: bool,
    },
    Build,
    Commit,
    Copy,
    Create,
    Diff,
    Events,
    Exec,
    Export,
    History,
    Images,
    Import,
    Info,
    Inspect,
    Kill {
        signal: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug = 1,
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
            _ => Err(Error::FailedToParse {
                type_name: type_name::<Self>().into(),
                value: Cow::Owned(s.to_string()),
            }),
        }
    }
}

fn main() -> Result<(), Error> {

    let parser = Builder::new()
        .root(|build| build
            .version(env!("CARGO_PKG_VERSION"))
            .help("Usage: docker [OPTIONS] COMMAND")
            .help("A self-sufficient runtime for containers.")
            .group(|build| build
                .name("Common Commands:")
                .ok()
            )?
            .group(|build| build
                .name("Management Commands:")
                .ok()
            )?
            .group(|build| build
                .name("Swarm Commands:")
                .ok()
            )?
            .group(|build| build
                .name("Commands:")
                .verb(|build| build
                    .name("attach")
                    .help("Attach local standard input, output, and error streams to a running container.")
                    .option(|build| build
                        .name("detach-keys")
                        .help("Override the key sequence for detaching a container.")
                    )
                    .option(|build| build
                        .name("no-stdin")
                        .help("Do not attach STDIN.")
                        .default(|| false)
                    )
                    .option(|build| build
                        .name("sig-proxy")
                        .help("Proxy all received signals to the process.")
                        .default(|| true)
                    )
                    .map(|(detach_keys, no_stdin, sig_proxy)| Command::Attach {
                        detach_keys,
                        no_stdin,
                        sig_proxy
                    })
                    .ok()
                )?
                .verb(|build| build
                    .name("kill")
                    .help("Signal to send to the container.")
                    .option(|build| build
                        .name("signal")
                        .name("s")
                    )
                    .map(|(signal,)| Command::Kill { signal })
                    .ok()
                )?
                .map(|(attach, kill)| attach.or(kill))
                .ok()
            )?
            .group(|build| build
                .name("Global Options:")
                .option(|build| build
                    .name("config")
                    .help("Location of client config files.")
                    .default(|| "/home/goulade/.docker".to_string())
                )
                .option(|build| build
                    .name("context")
                    .name("c")
                    .help(r#"Name of the context to use to connect to the daemon (overrides DOCKER_HOST env var and default context set with "docker context use")."#)
                    .environment("DOCKER_HOST")
                )
                .option(|build| build
                    .name("debug")
                    .name("D")
                    .help("Enable debug mode.")
                    .default(|| false)
                )
                .option(|build| build
                    .name("host")
                    .name("H")
                    .help("Daemon socket to connect to.")
                    .many::<_, Vec<_>>(Some(1))
                )
                .option(|build| build
                    .name("log-level")
                    .name("l")
                    .help("Set the logging level.") // TODO: Should display the available values + default automatically.
                    .default(|| LogLevel::Info)
                )
                .map(|(config, context, debug, host, log_level)| GlobalOptions{
                    config,
                    context,
                    debug,
                    host, 
                    log_level
                })
                .ok()
            )?
            .help("Run 'docker COMMAND --help' for more information on a command.")
            .help("For more help on how to use Docker, head to https://docs.docker.com/go/guides/")
            .try_map(|(_common, _management, _swarm, commands, global)| 
                Root {
                    command: commands.ok_or(Error::MissingVerb)?,
                    global
                }
                .ok::<Error>()
            )
            .ok()
        )?
        .build();
    let arguments = ["--config", "boba", "--debug", "false", "-H", "jango", "--host", "karl"];
    let environment = [("DOCKER_HOST", "fett")];
    match parser.parse_with(arguments, environment) {
        Ok(Some(docker)) => {
            assert_eq!(docker.global.config, "boba".to_string());
            assert_eq!(docker.global.context, Some("fett".to_string()));
            assert!(!docker.global.debug);
            assert_eq!(docker.global.host, vec!["jango".to_string(), "karl".to_string()]);
            assert_eq!(docker.global.log_level, LogLevel::Info);
            Ok(())
        }
        Ok(None) => todo!(),
        Err(Error::Help(Some(help))) => panic!("{help}"),
        Err(Error::Version(Some(version))) => panic!("{version}"),
        Err(error) => Err(error),
    }
}