use std::{any::TypeId, borrow::Cow, num::NonZeroUsize};

#[derive(Debug)]
pub enum Meta {
    Name(Cow<'static, str>),
    Position,
    Version(Cow<'static, str>),
    Help(Cow<'static, str>),
    Usage(Cow<'static, str>),
    Note(Cow<'static, str>),
    Type(Cow<'static, str>, TypeId),
    Required,
    Many(Option<NonZeroUsize>),
    Default(Cow<'static, str>),
    Environment(Cow<'static, str>),
    Show,
    Hide,
    Root(Vec<Meta>),
    Option(Vec<Meta>),
    Options(Options),
    Verb(Vec<Meta>),
    Group(Vec<Meta>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Options {
    Version { short: bool, long: bool },
    Help { short: bool, long: bool },
}

impl Options {
    pub const fn version(short: bool, long: bool) -> Self {
        Self::Version { short, long }
    }

    pub const fn help(short: bool, long: bool) -> Self {
        Self::Help { short, long }
    }
}

impl Meta {
    pub fn clone(&self, depth: usize) -> Self {
        match self {
            Meta::Name(value) => Meta::Name(value.clone()),
            Meta::Position => Meta::Position,
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Usage(value) => Meta::Usage(value.clone()),
            Meta::Note(value) => Meta::Note(value.clone()),
            Meta::Type(value, identifier) => Meta::Type(value.clone(), *identifier),
            Meta::Required => Meta::Required,
            Meta::Many(value) => Meta::Many(*value),
            Meta::Default(value) => Meta::Default(value.clone()),
            Meta::Environment(value) => Meta::Environment(value.clone()),
            Meta::Hide => Meta::Hide,
            Meta::Show => Meta::Show,
            Meta::Root(metas) if depth > 0 => {
                Meta::Root(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Root(_) => Meta::Root(Vec::new()),
            Meta::Option(metas) if depth > 0 => {
                Meta::Option(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Option(_) => Meta::Option(Vec::new()),
            Meta::Options(options) => Meta::Options(*options),
            Meta::Verb(metas) if depth > 0 => {
                Meta::Verb(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Verb(_) => Meta::Verb(Vec::new()),
            Meta::Group(metas) if depth > 0 => {
                Meta::Group(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Group(_) => Meta::Group(Vec::new()),
        }
    }
}