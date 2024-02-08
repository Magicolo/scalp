use core::{any::TypeId, num::NonZeroUsize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    Plain,
    Short,
    Long,
}

#[derive(Debug)]
pub enum Meta {
    Name(Name, Cow<'static, str>),
    Position,
    Version(Cow<'static, str>),
    License(Cow<'static, str>, Cow<'static, str>),
    Author(Cow<'static, str>),
    Help(Cow<'static, str>),
    Usage(Cow<'static, str>),
    Note(Cow<'static, str>),
    Type(Cow<'static, str>, TypeId),
    Required,
    Many(Option<NonZeroUsize>),
    Default(Cow<'static, str>),
    Valid(Cow<'static, str>),
    Environment(Cow<'static, str>),
    Show,
    Hide,
    Swizzle,
    Root(Vec<Meta>),
    Option(Vec<Meta>),
    Options(Options),
    Verb(Vec<Meta>),
    Group(Vec<Meta>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Options {
    Author { short: bool, long: bool },
    License { short: bool, long: bool },
    Version { short: bool, long: bool },
    Help { short: bool, long: bool },
}

impl Options {
    pub fn all(short: bool, long: bool) -> impl Iterator<Item = Options> {
        [
            Options::Author { short, long },
            Options::License { short, long },
            Options::Version { short, long },
            Options::Help { short, long },
        ]
        .into_iter()
    }

    pub const fn version(short: bool, long: bool) -> Self {
        Self::Version { short, long }
    }

    pub const fn help(short: bool, long: bool) -> Self {
        Self::Help { short, long }
    }

    pub const fn author(short: bool, long: bool) -> Self {
        Self::Author { short, long }
    }

    pub const fn license(short: bool, long: bool) -> Self {
        Self::License { short, long }
    }
}

impl Meta {
    pub fn clone(&self, depth: usize) -> Self {
        match self {
            Meta::Name(name, value) => Meta::Name(*name, value.clone()),
            Meta::Position => Meta::Position,
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::License(name, content) => Meta::License(name.clone(), content.clone()),
            Meta::Author(value) => Meta::Author(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Usage(value) => Meta::Usage(value.clone()),
            Meta::Note(value) => Meta::Note(value.clone()),
            Meta::Type(value, identifier) => Meta::Type(value.clone(), *identifier),
            Meta::Required => Meta::Required,
            Meta::Many(value) => Meta::Many(*value),
            Meta::Valid(value) => Meta::Valid(value.clone()),
            Meta::Default(value) => Meta::Default(value.clone()),
            Meta::Environment(value) => Meta::Environment(value.clone()),
            Meta::Hide => Meta::Hide,
            Meta::Show => Meta::Show,
            Meta::Swizzle => Meta::Swizzle,
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

    pub(crate) fn type_name(&self, depth: usize) -> Option<&Cow<'static, str>> {
        match self {
            Meta::Type(name, _) if depth == 0 => Some(name),
            Meta::Root(metas) | Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas)
                if depth > 0 =>
            {
                metas.iter().find_map(|meta| meta.type_name(depth - 1))
            }
            _ => None,
        }
    }
}
