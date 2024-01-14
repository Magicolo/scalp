use std::{borrow::Cow, collections::VecDeque, fmt, error};


#[derive(Debug, Clone)]
pub enum Error {
    Help(Option<Cow<'static, str>>),
    Version(Option<Cow<'static, str>>),
    MissingOptionValue,
    MissingValues,
    MissingRequiredArgument {
        name: Cow<'static, str>,
        index: usize,
    },
    UnrecognizedArgument {
        name: Cow<'static, str>,
    },
    UnrecognizedOption {
        name: Cow<'static, str>,
    },
    ExcessArguments(VecDeque<Cow<'static, str>>),
    MissingArgument,
    InvalidIndex {
        index: usize,
    },
    DuplicateName {
        name: Cow<'static, str>,
    },
    MissingOptionName,
    DuplicateOptionValue,
    DuplicateArgument {
        name: Cow<'static, str>,
        index: usize,
    },
    FailedToParse {
        value: Cow<'static, str>,
        type_name: Cow<'static, str>,
    },
    FailedToConvert {
        source_type: Cow<'static, str>,
        target_type: Cow<'static, str>,
    },
    MissingRequiredValue,
    DuplicateFailure,
    FailedToParseVariable(Cow<'static, str>),
    MissingValue,
    DuplicateNode,
    MissingVerb,
}

pub trait Ok: Sized {
    fn ok<E>(self) -> Result<Self, E>;
}

impl<T> Ok for T {
    fn ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl error::Error for Error {}
