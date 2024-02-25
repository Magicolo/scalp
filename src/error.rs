use crate::parse::Key;
use core::fmt;
use std::{borrow::Cow, collections::VecDeque, error};

#[derive(Clone, PartialEq)]
pub enum Error {
    Help(Option<String>),
    Version(Option<String>),
    Author(Option<String>),
    License(Option<String>),

    MissingOptionValue(Option<Cow<'static, str>>, Option<Key>),
    MissingRequiredValue(Option<Key>),
    DuplicateOption(Option<Key>),
    UnrecognizedArgument(Cow<'static, str>, Vec<(Cow<'static, str>, usize)>),
    ExcessArguments(VecDeque<Cow<'static, str>>),
    DuplicateName(String),
    Format(fmt::Error),
    Regex(regex::Error),
    Other(Cow<'static, str>),
    FailedToParseEnvironmentVariable(
        Cow<'static, str>,
        Cow<'static, str>,
        Option<Cow<'static, str>>,
        Option<Key>,
    ),
    FailedToParseOptionValue(Cow<'static, str>, Option<Cow<'static, str>>, Option<Key>),
    DuplicateNode,
    GroupNestingLimitOverflow,
    InvalidIndex(usize),
    MissingIndex,
    InvalidParseState,
    InvalidOptionName(Cow<'static, str>),
    InvalidVerbName(Cow<'static, str>),
    MissingOptionNameOrPosition,
    MissingVerbName,
    FailedToParseArguments,
    InvalidPrefix(Cow<'static, str>, Cow<'static, str>),
    MissingShortOptionNameForSwizzling,
    InvalidSwizzleOption(char),
    InvalidOptionType(Cow<'static, str>),
    InvalidInitialization,
    InvalidOptionValue(Cow<'static, str>, Option<Key>),
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
            Error::InvalidPrefix(short, long) => write!(f, "Invalid prefix '{short}' or '{long}'. A valid prefix is non-empty, contains only non-alpha-numeric characters and differs from the other prefix.")?,
            Error::DuplicateName(name) => write!(f, "Duplicate name '{name}'.")?,
            Error::InvalidIndex(index) => write!(f, "Invalid index '{index}'.")?,
            Error::MissingIndex => write!(f, "Missing index.")?,
            Error::InvalidVerbName(name) => write!(f, "Invalid verb name '{name}'. A valid verb name is non-empty and contains only ascii characters.")?,
            Error::InvalidOptionName(name) => write!(f, "Invalid option name '{name}'. A valid option name is non-empty and contains only ascii characters.")?,
            Error::InvalidOptionType(type_name) => write!(f, "Invalid option type '{type_name}'.")?,
            Error::InvalidOptionValue(value, name) => {
                write!(f, "Invalid value '{value}'")?;
                if let Some(name) = name {
                    write!(f, " for option '{name}'")?;
                }
                write!(f, ".")?;
            }
            Error::InvalidParseState => write!(f, "Invalid parse state.")?,
            Error::DuplicateNode => write!(f, "Duplicate node.")?,
            Error::GroupNestingLimitOverflow => write!(f, "Group nesting limit overflow.")?,
            Error::MissingOptionNameOrPosition => write!(f, "Missing name or position for option.")?,
            Error::MissingVerbName => write!(f, "Missing name for verb.")?,
            Error::FailedToParseArguments => write!(f, "Failed to parse arguments.")?,
            Error::MissingShortOptionNameForSwizzling => write!(f, "Missing short option name for swizzling. A valid short option name has only a single ascii character.")?,
            Error::InvalidSwizzleOption(value) => write!(f, "Invalid swizzle option '{value}'. A valid swizzle option is tagged for swizzling, has a short name and is of type 'boolean'.")?,
            Error::InvalidInitialization => write!(f, "Invalid initialization.")?,

            Error::Format(error) => error.fmt(f)?,
            Error::Regex(error) => error.fmt(f)?,
            Error::Other(error) => error.fmt(f)?,
        }
        Ok(())
    }
}

impl<T: fmt::Display> From<&T> for Error {
    fn from(value: &T) -> Self {
        Self::from(format!("{value}"))
    }
}

impl<T: fmt::Display> From<&mut T> for Error {
    fn from(value: &mut T) -> Self {
        Self::from(format!("{value}"))
    }
}

impl From<fmt::Error> for Error {
    fn from(error: fmt::Error) -> Self {
        Error::Format(error)
    }
}

impl From<regex::Error> for Error {
    fn from(error: regex::Error) -> Self {
        Error::Regex(error)
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
        Error::Other(value)
    }
}
