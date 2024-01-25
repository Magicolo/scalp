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
    Format(fmt::Error),
    Other(Box<dyn error::Error + Send + Sync>),

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
    InvalidName {
        name: Cow<'static, str>,
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
            Error::ExcessArguments { arguments } => {
                write!(f, "Excess arguments '")?;
                let mut join = false;
                for argument in arguments {
                    if join {
                        write!(f, ", ")?;
                    } else {
                        join = true;
                    }
                    write!(f, "{argument}")?;
                }
                write!(f, "'.")?;
            }
            Error::Anyhow(error) => error.fmt(f)?,
            Error::Format(error) => error.fmt(f)?,
            Error::Other(error) => error.fmt(f)?,

            Error::MissingOptionValue => todo!(),
            Error::DuplicateName { name } => todo!(),
            Error::DuplicateOptionValue => todo!(),
            Error::MissingRequiredValue => todo!(),
            Error::DuplicateNode => todo!(),
            Error::GroupNestingLimitOverflow => todo!(),
            Error::InvalidIndex { index } => todo!(),
            Error::FailedToParseEnvironmentVariable { key, value } => todo!(),
            Error::InvalidName { name } => todo!(),
        }
        Ok(())
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::Anyhow(error)
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error::Format(error)
    }
}

impl<E: error::Error + Send + Sync + 'static> From<Box<E>> for Error {
    fn from(error: Box<E>) -> Self {
        Error::Other(error)
    }
}

impl error::Error for Error {}
