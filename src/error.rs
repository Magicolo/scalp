use crate::parse::Key;
use core::fmt;
use std::{borrow::Cow, collections::VecDeque, error, mem::replace};

#[derive(Clone, PartialEq)]
pub enum Error {
    Help(Option<String>),
    Version(Option<String>),
    Author(Option<String>),
    License(Option<String>),

    MissingOptionValue(Option<Cow<'static, str>>, Vec<Key>),
    MissingRequiredValue(Vec<Key>, Option<Key>, Option<Cow<'static, str>>),
    DuplicateOption(Vec<Key>),
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
        Vec<Key>,
        Option<Key>,
    ),
    FailedToParseOptionValue(
        Cow<'static, str>,
        Option<Cow<'static, str>>,
        Vec<Key>,
        Option<Key>,
    ),
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
    InvalidOptionValue(Cow<'static, str>, Vec<Key>),
    InvalidArgument(Cow<'static, str>, Vec<Key>, Option<Key>, Vec<String>),
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

            Error::InvalidArgument(argument, path, name, patterns) => {
                write!(f, "Invalid argument '{argument}'")?;
                write_join(f, " for ", "", " ", path.iter().chain(name))?;
                write!(f, ".")?;
                write_join(f, " Argument must match '", "'.", " | ", patterns)?;
            }
            Error::UnrecognizedArgument(argument, suggestions) => {
                write!(f, "Unrecognized argument '{argument}'.")?;
                let suggestions = suggestions.iter().map(|(suggestion, _)| format!("'{suggestion}'"));
                write_join(f, " Similar matches: ", ".", ", ", suggestions)?;
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
            Error::MissingOptionValue(type_name, path) => {
                write!(f, "Missing value")?;
                if let Some(type_name) = type_name {
                    write!(f, " of type '{type_name}'")?;
                }
                write_join(f, " for option ", "", " ", path.iter())?;
                write!(f, ".")?;
            }
            Error::DuplicateOption(path) => {
                write!(f, "Duplicate option")?;
                write_join(f, " ", "", " ", path.iter())?;
                write!(f, ".")?;
            }
            Error::MissingRequiredValue(path, name, type_name) => {
                write!(f, "Missing required value")?;
                if let Some(type_name) = type_name {
                    write!(f, " of type '{type_name}'")?;
                }
                write_join(f, " for ", "", " ", path.iter().chain(name))?;
                write!(f, ".")?;
            }
            Error::FailedToParseEnvironmentVariable(key, value, type_name, path, name) => {
                write!(
                    f,
                    "Failed to parse environment variable '{key}' with value '{value}'"
                )?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                write_join(f, " for option ", "", " ", path.iter().chain(name))?;
                write!(f, ".")?;
            }
            Error::FailedToParseOptionValue(value, type_name, path, name) => {
                write!(f, "Failed to parse value '{value}'")?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                write_join(f, " for option ", "", " ", path.iter().chain(name))?;
                write!(f, ".")?;
            }
            Error::InvalidPrefix(short, long) => write!(f, "Invalid prefix '{short}' or '{long}'. A valid prefix is non-empty, contains only non-alpha-numeric characters and differs from the other prefix.")?,
            Error::DuplicateName(name) => write!(f, "Duplicate name '{name}'.")?,
            Error::InvalidIndex(index) => write!(f, "Invalid index '{index}'.")?,
            Error::MissingIndex => write!(f, "Missing index.")?,
            Error::InvalidVerbName(name) => write!(f, "Invalid verb name '{name}'. A valid verb name is non-empty and contains only ascii characters.")?,
            Error::InvalidOptionName(name) => write!(f, "Invalid option name '{name}'. A valid option name is non-empty and contains only ascii characters.")?,
            Error::InvalidOptionType(type_name) => write!(f, "Invalid option type '{type_name}'.")?,
            Error::InvalidOptionValue(value, path) => {
                write!(f, "Invalid value '{value}'")?;
                write_join(f, " for option ", "", " ", path.iter())?;
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

fn write_join(
    formatter: &mut fmt::Formatter,
    prefix: impl fmt::Display,
    suffix: impl fmt::Display,
    separator: impl fmt::Display,
    items: impl IntoIterator<Item = impl fmt::Display>,
) -> Result<(), fmt::Error> {
    let mut has = false;
    for item in items.into_iter() {
        if replace(&mut has, true) {
            write!(formatter, "{separator}")?;
        } else {
            write!(formatter, "{prefix}")?;
        }
        write!(formatter, "{item}")?;
    }
    if has {
        write!(formatter, "{suffix}")?;
    }
    Ok(())
}

// fn write_join(
//     formatter: &mut fmt::Formatter,
//     prefix: impl fmt::Display,
//     suffix: impl fmt::Display,
//     path: impl IntoIterator<Item = impl Deref<Target = Key>>,
// ) -> Result<bool, fmt::Error> {
//     let mut has = false;
//     for key in path.into_iter() {
//         let key = key.deref();
//         if replace(&mut has, true) {
//             write!(formatter, " ")?;
//         } else {
//             write!(formatter, "{prefix}'")?;
//         }
//         write!(formatter, "{key}")?;
//     }
//     if has {
//         write!(formatter, "'{suffix}")?;
//         Ok(true)
//     } else {
//         Ok(false)
//     }
// }
