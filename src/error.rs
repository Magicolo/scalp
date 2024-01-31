use std::{borrow::Cow, collections::VecDeque, error, fmt};

pub enum Error {
    Help(Option<String>),
    Version(Option<Cow<'static, str>>),
    MissingOptionValue(Option<Cow<'static, str>>, &'static str),
    MissingRequiredValue(Option<Cow<'static, str>>),
    DuplicateOption(Option<Cow<'static, str>>),
    UnrecognizedArgument(String, Vec<(Cow<'static, str>, usize)>),
    ExcessArguments(VecDeque<Cow<'static, str>>),
    DuplicateName(Cow<'static, str>),
    Format(fmt::Error),
    Text(Cow<'static, str>),
    Box(Box<dyn error::Error + Send + Sync>),

    FailedToParseEnvironmentVariable {
        key: Cow<'static, str>,
        value: Cow<'static, str>,
        type_name: &'static str,
    },
    DuplicateNode,
    GroupNestingLimitOverflow,
    InvalidIndex(usize),
    InvalidName(Cow<'static, str>),
    MissingOptionNameOrPosition,
    MissingVerbName,
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
            Error::UnrecognizedArgument(argument, suggestions) => {
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
            Error::ExcessArguments(arguments) => {
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
            Error::MissingOptionValue(None, value) => write!(f, "Missing value of type '{value}'.")?,
            Error::MissingOptionValue(Some(key), value) => {
                write!(f, "Missing value of type '{value}' for option '{key}'.")?
            }
            Error::DuplicateOption(Some(key)) => write!(f, "Duplicate option '{key}'.")?,
            Error::DuplicateOption(None) => write!(f, "Duplicate option.")?,
            Error::MissingRequiredValue(Some(key)) => write!(f, "Missing required option '{key}'.")?,
            Error::MissingRequiredValue(None) => write!(f, "Missing required option.")?,
            Error::FailedToParseEnvironmentVariable { key, value, type_name } => {
                write!(f, "Failed to parse environment variable '{key}' with value '{value}' as type '{type_name}'.")?
            },

            Error::Format(error) => error.fmt(f)?,
            Error::Text(error) => error.fmt(f)?,
            Error::Box(error) => error.fmt(f)?,

            Error::DuplicateName(name) => todo!(),
            Error::DuplicateNode => todo!(),
            Error::GroupNestingLimitOverflow => todo!(),
            Error::InvalidIndex(index) => todo!(),
            Error::InvalidName(name) => todo!(),
            Error::MissingOptionNameOrPosition => todo!(),
            Error::MissingVerbName => todo!(),
        }
        Ok(())
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error::Format(error)
    }
}

impl From<Cow<'static, str>> for Error {
    fn from(value: Cow<'static, str>) -> Self {
        Error::Text(value)
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Error::from(Cow::Borrowed(value))
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::from(Cow::Owned(value))
    }
}

impl<E: error::Error + Send + Sync + 'static> From<Box<E>> for Error {
    fn from(error: Box<E>) -> Self {
        Error::Box(error)
    }
}

impl error::Error for Error {}
