use core::fmt;
use std::{borrow::Cow, collections::VecDeque, error};

#[derive(Clone)]
pub enum Error {
    Help(Option<String>),
    Version(Option<String>),
    Author(Option<String>),
    License(Option<String>),

    MissingOptionValue(Option<Cow<'static, str>>, Option<Cow<'static, str>>),
    MissingRequiredValue(Option<Cow<'static, str>>),
    DuplicateOption(Option<Cow<'static, str>>),
    UnrecognizedArgument(Cow<'static, str>, Vec<(Cow<'static, str>, usize)>),
    ExcessArguments(VecDeque<Cow<'static, str>>),
    DuplicateName(String),
    Format(fmt::Error),
    Text(Cow<'static, str>),
    FailedToParseEnvironmentVariable(
        Cow<'static, str>,
        Cow<'static, str>,
        Option<Cow<'static, str>>,
        Option<Cow<'static, str>>,
    ),
    FailedToParseOptionValue(
        Cow<'static, str>,
        Option<Cow<'static, str>>,
        Option<Cow<'static, str>>,
    ),

    DuplicateNode,
    GroupNestingLimitOverflow,
    InvalidIndex(usize),
    InvalidOptionName(String),
    InvalidVerbName(String),
    MissingOptionNameOrPosition,
    MissingVerbName,
    FailedToParseArguments,
    InvalidShortPrefix(Cow<'static, str>),
    InvalidLongPrefix(Cow<'static, str>),
    MissingShortOptionNameForSwizzling,
    InvalidSwizzleOption(char),
}

impl error::Error for Error {}

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
            Error::Author(Some(author)) => write!(f, "{author}")?,
            Error::Author(None) => write!(f, "Missing author.")?,
            Error::License(Some(author)) => write!(f, "{author}")?,
            Error::License(None) => write!(f, "Missing license.")?,

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
            Error::MissingOptionValue(type_name, option) => {
                write!(f, "Missing value")?;
                if let Some(type_name) = type_name {
                    write!(f, " of type '{type_name}'")?;
                }
                if let Some(option) = option {
                    write!(f, " for option '{option}'")?;
                }
                write!(f, ".")?;
            }
            Error::DuplicateOption(key) => {
                write!(f, "Duplicate option")?;
                if let Some(key) = key {
                    write!(f, " '{key}'")?;
                }
                write!(f, ".")?;
            }
            Error::MissingRequiredValue(key) => {
                write!(f, "Missing required option")?;
                if let Some(key) = key {
                    write!(f, " '{key}'")?;
                }
                write!(f, ".")?;
            }
            Error::FailedToParseEnvironmentVariable(key, value, type_name, option) => {
                write!(
                    f,
                    "Failed to parse environment variable '{key}' with value '{value}'"
                )?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                if let Some(option) = option {
                    write!(f, " for option '{option}'")?;
                }
                write!(f, ".")?;
            }
            Error::FailedToParseOptionValue(value, type_name, option) => {
                write!(f, "Failed to parse value '{value}'")?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                if let Some(option) = option {
                    write!(f, " for option '{option}'")?;
                }
                write!(f, ".")?;
            }
            Error::InvalidShortPrefix(prefix) => write!(f, "Invalid short prefix '{prefix}'. A valid short prefix is non-empty, contains only non-alpha-numeric characters and differs from the long prefix.")?,
            Error::InvalidLongPrefix(prefix) => write!(f, "Invalid long prefix '{prefix}'. A valid long prefix is non-empty, contains only non-alpha-numeric characters and differs from the short prefix.")?,
            Error::DuplicateName(name) => write!(f, "Duplicate name '{name}'.")?,
            Error::InvalidIndex(index) => write!(f, "Invalid index '{index}'.")?,
            Error::InvalidVerbName(name) => write!(f, "Invalid verb name '{name}'. A valid verb name is non-empty and contains only ascii characters.")?,
            Error::InvalidOptionName(name) => write!(f, "Invalid option name '{name}'. A valid option name is non-empty and contains only ascii characters.")?,
            Error::DuplicateNode => write!(f, "Duplicate node.")?,
            Error::GroupNestingLimitOverflow => write!(f, "Group nesting limit overflow.")?,
            Error::MissingOptionNameOrPosition => write!(f, "Missing name or position for option.")?,
            Error::MissingVerbName => write!(f, "Missing name for verb.")?,
            Error::FailedToParseArguments => write!(f, "Failed to parse arguments.")?,
            Error::MissingShortOptionNameForSwizzling => write!(f, "Missing short option name for swizzling. A valid short option name has only a single ascii character.")?,
            Error::InvalidSwizzleOption(value) => write!(f, "Invalid swizzle option '{value}'. A valid swizzle option is tagged for swizzling, has a short name and is of type 'boolean'.")?,

            Error::Format(error) => error.fmt(f)?,
            Error::Text(error) => error.fmt(f)?,
        }
        Ok(())
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error::Format(error)
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

impl From<Cow<'static, str>> for Error {
    fn from(value: Cow<'static, str>) -> Self {
        Error::Text(value)
    }
}
