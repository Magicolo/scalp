use core::fmt;
use std::str::FromStr;
use scalp::*;

pub struct Docker {
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
    // Common Commands
    Run,
    Execute,
    Process,
    Build,
    Pull,
    Push,
    Images,
    Login,
    Logout,
    Search,
    Version,
    Info,

    // Management Commands
    Builder,
    Buildx,
    Compose,
    Container,
    Context,
    Image,
    Manifest,
    Network,
    Plugin,
    System,
    Trust,
    Volume,

    // Swarm Commands
    Swarm,

    // Commands
    Attach {
        detach_keys: Option<String>,
        no_stdin: bool,
        sig_proxy: bool,
    },
    Commit,
    Copy,
    Create,
    Diff,
    Events,
    Export,
    History,
    Import,
    Inspect,
    Kill {
        signal: Option<String>,
    },
    Load,
    Logs,
    Pause,
    Port,
    Rename,
    Restart,
    RemoveContainer,
    RemoveImage,
    Save,
    Start,
    Stats,
    Stop,
    Tag,
    Top,
    Unpause,
    Update,
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" => Ok(LogLevel::Fatal),
            _ => Err("Failed to parse."),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn common_commands(builder: Builder<scope::Group>) -> Builder<scope::Group, impl Parse<Value = Option<Command>>> {
    builder
        .name("Common Commands:")
        .verb(|verb| verb.name("run").map(|_| Command::Run))
        .verb(|verb| verb.name("exec").map(|_| Command::Execute))
        .verb(|verb| verb.name("ps").map(|_| Command::Process))
        .verb(|verb| verb.name("build").map(|_| Command::Build))
        .verb(|verb| verb.name("pull").map(|_| Command::Pull))
        .verb(|verb| verb.name("push").map(|_| Command::Push))
        .verb(|verb| verb.name("images").map(|_| Command::Images))
        .verb(|verb| verb.name("login").map(|_| Command::Login))
        .verb(|verb| verb.name("logout").map(|_| Command::Logout))
        .verb(|verb| verb.name("search").map(|_| Command::Search))
        .verb(|verb| verb.name("version").map(|_| Command::Version))
        .verb(|verb| verb.name("info").map(|_| Command::Info))
        .any::<Command>()
}

fn management_commands(builder: Builder<scope::Group>) -> Builder<scope::Group, impl Parse<Value = Option<Command>>> {
    builder
        .name("Management Commands:")
        .verb(|verb| verb.name("builder").map(|_| Command::Builder))
        .verb(|verb| verb.name("buildx").map(|_| Command::Buildx))
        .verb(|verb| verb.name("compose").map(|_| Command::Compose))
        .verb(|verb| verb.name("container").map(|_| Command::Container))
        .verb(|verb| verb.name("context").map(|_| Command::Context))
        .verb(|verb| verb.name("image").map(|_| Command::Image))
        .verb(|verb| verb.name("manifest").map(|_| Command::Manifest))
        .verb(|verb| verb.name("network").map(|_| Command::Network))
        .verb(|verb| verb.name("plugin").map(|_| Command::Plugin))
        .verb(|verb| verb.name("system").map(|_| Command::System))
        .verb(|verb| verb.name("trust").map(|_| Command::Trust))
        .verb(|verb| verb.name("volume").map(|_| Command::Volume))
        .any::<Command>()
}

fn swarm_commands(builder: Builder<scope::Group>) -> Builder<scope::Group, impl Parse<Value = Option<Command>>> {
    builder
        .name("Swarm Commands:")
        .verb(|verb| verb.name("swarm").map(|_| Command::Swarm))
        .any::<Command>()
}

fn commands(builder: Builder<scope::Group>) -> Builder<scope::Group, impl Parse<Value = Option<Command>>> {
    builder
        .name("Commands:")
        .verb(|verb| verb
            .name("attach")
            .help("Attach local standard input, output, and error streams to a running container.")
            .option(|option| option
                .name("detach-keys")
                .help("Override the key sequence for detaching a container.")
            )
            .option(|option| option
                .name("no-stdin")
                .help("Do not attach STDIN.")
                .default(false)
            )
            .option(|option| option
                .name("sig-proxy")
                .help("Proxy all received signals to the process.")
                .default(true)
            )
            .map(|(detach_keys, no_stdin, sig_proxy)| Command::Attach { detach_keys, no_stdin, sig_proxy })
        )
        .verb(|verb| verb.name("commit").map(|_| Command::Commit))
        .verb(|verb| verb.name("cp").map(|_| Command::Copy))
        .verb(|verb| verb.name("create").map(|_| Command::Create))
        .verb(|verb| verb.name("diff").map(|_| Command::Diff))
        .verb(|verb| verb.name("events").map(|_| Command::Events))
        .verb(|verb| verb.name("export").map(|_| Command::Export))
        .verb(|verb| verb.name("history").map(|_| Command::History))
        .verb(|verb| verb.name("import").map(|_| Command::Import))
        .verb(|verb| verb.name("inspect").map(|_| Command::Inspect))
        .verb(|verb| verb
            .name("kill")
            .summary("Kill one or more running containers.")
            .usage("Usage: docker kill [OPTIONS] CONTAINER [CONTAINER...]")
            .line()
            .note("Aliases: docker container kill, docker kill")
            .line()
            .option(|option| option
                .name("s")
                .name("signal")
                .help("Signal to send to the container."))
            .map(|(signal,)| Command::Kill { signal })
        )
        .verb(|verb| verb.name("load").map(|_| Command::Load))
        .verb(|verb| verb.name("logs").map(|_| Command::Logs))
        .verb(|verb| verb.name("pause").map(|_| Command::Pause))
        .verb(|verb| verb.name("port").map(|_| Command::Port))
        .verb(|verb| verb.name("rename").map(|_| Command::Rename))
        .verb(|verb| verb.name("restart").map(|_| Command::Restart))
        .verb(|verb| verb.name("rm").map(|_| Command::RemoveContainer))
        .verb(|verb| verb.name("rmi").map(|_| Command::RemoveImage))
        .verb(|verb| verb.name("save").map(|_| Command::Save))
        .verb(|verb| verb.name("start").map(|_| Command::Start))
        .verb(|verb| verb.name("stats").map(|_| Command::Stats))
        .verb(|verb| verb.name("stop").map(|_| Command::Stop))
        .verb(|verb| verb.name("tag").map(|_| Command::Tag))
        .verb(|verb| verb.name("top").map(|_| Command::Top))
        .verb(|verb| verb.name("unpause").map(|_| Command::Unpause))
        .verb(|verb| verb.name("update").map(|_| Command::Update))
        .verb(|verb| verb.name("wait").map(|_| Command::Wait))
        .any::<Command>()
}

fn global_options(builder: Builder<scope::Group>) -> Builder<scope::Group, impl Parse<Value = GlobalOptions>> {
    builder 
        .name("Global Options:")
        .option(|option| option
            .name("config")
            .help("Location of client config files.")
            .default("/home/goulade/.docker")
        )
        .option(|option| option
            .name("c")
            .name("context")
            .help(r#"Name of the context to use to connect to the daemon (overrides default context set with "docker context use")."#)
            .environment("DOCKER_HOST")
        )
        .option(|option| option
            .name("D")
            .name("debug")
            .help("Enable debug mode.")
        )
        .option(|option| option
            .name("H")
            .name("host")
            .help("Daemon socket to connect to.")
            .many()
        )
        .option(|option| option
            .name("l")
            .name("log-level")
            .help("Set the logging level.")
            .valid("i(nfo)?")
            .valid("d(ebug)?")
            .valid("w(arn)?")
            .valid("e(rror)?")
            .valid("f(atal)?")
            .valid("[idwef]+")
            .default(LogLevel::Info)
        )
        .options(Options::common(true, true))
        .map(|(config, context, debug, host, log_level)| GlobalOptions {
            config,
            context,
            debug: debug.unwrap_or_default(),
            host: host.unwrap_or_default(),
            log_level
        })
}

fn main() -> Result<(), Error> {
    let parser = Parser::builder()
        .name(env!("CARGO_BIN_NAME").trim())
        .version(env!("CARGO_PKG_VERSION").trim())
        .summary("A self-sufficient runtime for containers.")
        .usage("Usage: docker [OPTIONS] COMMAND")
        .group(|group| group
            .group(common_commands)
            .group(management_commands)
            .group(swarm_commands)
            .group(commands)
            .any::<Command>()
            .try_map(|command| Ok(command.ok_or("Missing command.")?))
        )
        .group(global_options)
        .help("Run 'docker COMMAND --help' for more information on a command.")
        .line()
        .note("For more help on how to use Docker, head to https://docs.docker.com/go/guides/")
        .map(|(command, global)| Docker { command, global })
        .build()?;
    let arguments = [
        "--help", "--config", "boba", "--debug", "false", "-H", "jango", "--host", "karl", "kill",
    ];
    let environment = [("DOCKER_HOST", "fett")];
    let docker = match parser.parse_with(arguments, environment) {
        Ok(docker) => docker,
        Err(Error::Help(Some(value))
            | Error::Version(Some(value))
            | Error::License(Some(value))
            | Error::Author(Some(value))) => return Ok(println!("{}", value)),
        Err(error) => return Err(error),
    };
    assert_eq!(docker.global.config, "boba".to_string());
    assert_eq!(docker.global.context, Some("fett".to_string()));
    assert!(!docker.global.debug);
    assert_eq!(
        docker.global.host,
        vec!["jango".to_string(), "karl".to_string()]
    );
    assert_eq!(docker.global.log_level, LogLevel::Info);
    Ok(())
}