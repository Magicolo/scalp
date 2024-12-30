use crate::{
    help::{Author, Help, License, Version},
    meta::{Meta, Text},
    parse::Key,
};
use core::fmt;
use std::{error, mem::replace};

#[derive(Clone)]
pub enum Error {
    Help(Option<Help>),
    Version(Option<Version>),
    Author(Option<Author>),
    License(Option<License>),

    MissingValue(Option<Text>, Vec<Key>),
    MissingRequired(Option<Text>, Vec<Key>),
    DuplicateOption(Vec<Key>),
    UnrecognizedArgument(Text, Vec<(Text, usize)>),
    ExcessArguments(Vec<Text>),
    DuplicateName(String),
    Format(fmt::Error),
    Regex(regex::Error),
    Other(Text),
    FailedToParseEnvironmentVariable(Text, Text, Option<Text>, Vec<Key>),
    FailedToParseOptionValue(Text, Option<Text>, Vec<Key>),
    DuplicateParse(Vec<Key>),
    DuplicateVerb(Vec<Key>),
    GroupNestingLimitOverflow,
    InvalidIndex(usize),
    MissingIndex,
    InvalidParseState,
    InvalidOptionName(Text),
    InvalidGroupName(Text),
    InvalidVerbName(Text),
    InvalidLeafMeta(Meta),
    MissingOptionNameOrPosition,
    MissingVerbName,
    FailedToParseArguments,
    InvalidPrefix(char),
    MissingShortOptionNameForSwizzling,
    InvalidSwizzleOption(Text),
    InvalidOptionType(Text),
    InvalidInitialization,
    InvalidOptionValue(Text, Vec<String>, Vec<Key>),
    InvalidArgument(Text, Vec<String>, Vec<Key>),
    EmptyShortOption,
    EmptyArgument,
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
            Error::InvalidArgument(argument, patterns, path) => {
                write!(f, "Invalid argument '{argument}'")?;
                write_join(f, " for '", "'", " ", path)?;
                write!(f, ".")?;
                write_join(f, " Argument must match pattern '", "'.", " | ", patterns)?;
            }
            Error::UnrecognizedArgument(argument, suggestions) => {
                write!(f, "Unrecognized argument '{argument}'.")?;
                let suggestions = suggestions
                    .iter()
                    .map(|(suggestion, _)| format!("'{suggestion}'"));
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
            Error::MissingValue(type_name, path) => {
                write!(f, "Missing value")?;
                if let Some(type_name) = type_name {
                    write!(f, " of type '{type_name}'")?;
                }
                write_join(f, " for option '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::DuplicateParse(path) => {
                write!(f, "Duplicate parse")?;
                write_join(f, " '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::DuplicateOption(path) => {
                write!(f, "Duplicate option")?;
                write_join(f, " '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::DuplicateVerb(path) => {
                write!(f, "Duplicate verb")?;
                write_join(f, " '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::MissingRequired(type_name, path) => {
                write!(f, "Missing required value")?;
                if let Some(type_name) = type_name {
                    write!(f, " of type '{type_name}'")?;
                }
                write_join(f, " at '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::FailedToParseEnvironmentVariable(key, value, type_name, path) => {
                write!(
                    f,
                    "Failed to parse environment variable '{key}' with value '{value}'"
                )?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                write_join(f, " for option '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::FailedToParseOptionValue(value, type_name, path) => {
                write!(f, "Failed to parse value '{value}'")?;
                if let Some(type_name) = type_name {
                    write!(f, " as type '{type_name}'")?;
                }
                write_join(f, " for option '", "'", " ", path)?;
                write!(f, ".")?;
            }
            Error::InvalidPrefix(prefix) => write!(
                f,
                "Invalid prefix '{prefix}'. A valid prefix is a non-whitespace, non-control, \
                 non-alpha-numeric character."
            )?,
            Error::DuplicateName(name) => write!(f, "Duplicate name '{name}'.")?,
            Error::InvalidIndex(index) => write!(f, "Invalid index '{index}'.")?,
            Error::MissingIndex => write!(f, "Missing index.")?,
            Error::InvalidGroupName(name) => write!(
                f,
                "Invalid group name '{name}'. A valid group name contains at least 1 \
                 non-whitespace character."
            )?,
            Error::InvalidVerbName(name) => write!(
                f,
                "Invalid verb name '{name}'. A valid verb name is non-empty and contains only \
                 ascii character(s)."
            )?,
            Error::InvalidOptionName(name) => write!(
                f,
                "Invalid option name '{name}'. A valid option name is non-empty and contains only \
                 ascii character(s)."
            )?,
            Error::InvalidOptionType(type_name) => write!(f, "Invalid option type '{type_name}'.")?,
            Error::InvalidOptionValue(value, patterns, path) => {
                write!(f, "Invalid value '{value}'")?;
                write_join(f, " for option '", "'", " ", path)?;
                write!(f, ".")?;
                write_join(f, " Value must match pattern '", "'.", "|", patterns)?;
            }
            Error::InvalidLeafMeta(meta) => write!(
                f,
                "Invalid leaf meta '{meta:?}'. A node meta such as `Meta::Option | Meta::Verb | \
                 Meta::Group` is required."
            )?,
            Error::InvalidParseState => write!(f, "Invalid parse state.")?,
            Error::GroupNestingLimitOverflow => write!(f, "Group nesting limit overflow.")?,
            Error::MissingOptionNameOrPosition => {
                write!(f, "Missing name or position for option.")?
            }
            Error::MissingVerbName => write!(f, "Missing name for verb.")?,
            Error::FailedToParseArguments => write!(f, "Failed to parse arguments.")?,
            Error::MissingShortOptionNameForSwizzling => write!(
                f,
                "Missing short option name for swizzling. A valid short option name has only a \
                 single ascii character."
            )?,
            Error::InvalidSwizzleOption(value) => write!(
                f,
                "Invalid swizzle option '{value}'. A valid swizzle option is tagged for \
                 swizzling, has a short name and is of type 'boolean'."
            )?,
            Error::InvalidInitialization => write!(f, "Invalid initialization.")?,
            Error::Format(error) => error.fmt(f)?,
            Error::Regex(error) => error.fmt(f)?,
            Error::Other(error) => error.fmt(f)?,
            Error::EmptyShortOption => write!(f, "Empty short option.")?,
            Error::EmptyArgument => write!(f, "Empty argument.")?,
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
        Error::from(Text::from(value))
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::from(Text::from(value))
    }
}

impl From<Text> for Error {
    fn from(value: Text) -> Self {
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
