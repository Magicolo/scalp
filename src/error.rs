use std::{borrow::Cow, collections::VecDeque, error, fmt};

pub enum Error {
    Help(Option<Cow<'static, str>>),
    Version(Option<Cow<'static, str>>),
    MissingOptionValue,
    UnrecognizedArgument {
        argument: Cow<'static, str>,
        suggestions: Vec<(Cow<'static, str>, usize)>,
    },
    ExcessArguments {
        arguments: VecDeque<Cow<'static, str>>,
    },
    DuplicateName {
        name: Cow<'static, str>,
    },
    DuplicateOptionValue,
    Anyhow(anyhow::Error),
    MissingRequiredValue,
    FailedToParseEnvironmentVariable {
        key: Cow<'static, str>,
        value: Cow<'static, str>,
    },
    DuplicateNode,
    GroupNestingLimitOverflow,
    InvalidIndex {
        index: usize,
    },
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Help(Some(help)) => write!(f, "{help}")?,
            Error::Help(None) => write!(f, "Missing help.")?,
            Error::Version(Some(version)) => write!(f, "{version}")?,
            Error::Version(None) => write!(f, "Missing version.")?,
            Error::UnrecognizedArgument {
                argument,
                suggestions,
            } => {
                write!(f, "Unrecognized argument '{argument}'.")?;
                let mut join = false;
                for (suggestion, _) in suggestions {
                    if join {
                        write!(f, ", ")?;
                    } else {
                        write!(f, " Similar matches: ")?;
                        join = true;
                    }
                    write!(f, "'{suggestion}'")?;
                }
            }

            Error::MissingOptionValue => todo!(),
            Error::ExcessArguments { arguments } => todo!(),
            Error::DuplicateName { name } => todo!(),
            Error::DuplicateOptionValue => todo!(),
            Error::MissingRequiredValue => todo!(),
            Error::DuplicateNode => todo!(),
            Error::GroupNestingLimitOverflow => todo!(),
            Error::InvalidIndex { index } => todo!(),
            Error::FailedToParseEnvironmentVariable { key, value } => todo!(),
            Error::Anyhow(error) => error.fmt(f)?,
        }
        Ok(())
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::Anyhow(error)
    }
}

impl error::Error for Error {}
