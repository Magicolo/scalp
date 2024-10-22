use core::num::NonZeroUsize;
use regex::Regex;
use std::{
    borrow::Cow,
    cmp, fmt, hash,
    iter::from_fn,
    ops::{ControlFlow, Deref},
    slice::from_ref,
    sync::Arc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    None,
    Short,
    Long,
}

#[derive(Debug, Clone, Eq)]
pub enum Text {
    Static(&'static str),
    Shared(Arc<str>),
}

#[derive(Debug, Clone)]
pub enum Meta {
    Name(Text),
    Position,
    Version(Text),
    License(Text, Text),
    Author(Text),
    Help(Text),
    Line,
    Usage(Text),
    Summary(Text),
    Home(Text),
    Repository(Text),
    Note(Text),
    Type(Text),
    Valid(Regex),
    Require(Text),
    Many(Option<NonZeroUsize>),
    Default(Text),
    Environment(Text),
    Show,
    Hide,
    Swizzle,
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

    pub fn common(short: bool, long: bool) -> impl Iterator<Item = Options> {
        [Options::Version { short, long }, Options::Help {
            short,
            long,
        }]
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
            Meta::Name(value) => Meta::Name(value.clone()),
            Meta::Position => Meta::Position,
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::License(name, content) => Meta::License(name.clone(), content.clone()),
            Meta::Author(value) => Meta::Author(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Line => Meta::Line,
            Meta::Summary(value) => Meta::Summary(value.clone()),
            Meta::Home(value) => Meta::Home(value.clone()),
            Meta::Repository(value) => Meta::Repository(value.clone()),
            Meta::Usage(value) => Meta::Usage(value.clone()),
            Meta::Note(value) => Meta::Note(value.clone()),
            Meta::Type(value) => Meta::Type(value.clone()),
            Meta::Require(value) => Meta::Require(value.clone()),
            Meta::Many(value) => Meta::Many(*value),
            Meta::Default(value) => Meta::Default(value.clone()),
            Meta::Environment(value) => Meta::Environment(value.clone()),
            Meta::Valid(value) => Meta::Valid(value.clone()),
            Meta::Hide => Meta::Hide,
            Meta::Show => Meta::Show,
            Meta::Swizzle => Meta::Swizzle,
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

    pub(crate) fn require(&self) -> Option<Text> {
        let control = Self::descend(
            from_ref(self),
            None,
            false,
            1,
            |state, meta| {
                ControlFlow::<(), _>::Continue(match meta {
                    Meta::Require(value) => state.or(Some(value)),
                    _ => state,
                })
            },
            |state, _| ControlFlow::Continue(state),
        );
        match control {
            ControlFlow::Continue(Some(value)) => Some(value.clone()),
            _ => None,
        }
    }

    pub(crate) fn descend<'a, T, S>(
        metas: &'a [Meta],
        mut state: S,
        hidden: bool,
        depth: usize,
        mut down: impl FnMut(S, &'a Meta) -> ControlFlow<T, S>,
        mut up: impl FnMut(S, &'a Meta) -> ControlFlow<T, S>,
    ) -> ControlFlow<T, S> {
        if hidden {
            for meta in metas {
                state = meta.descend_one(state, hidden, depth, &mut down, &mut up)?;
            }
        } else {
            for meta in Meta::visible(metas) {
                state = meta.descend_one(state, hidden, depth, &mut down, &mut up)?;
            }
        }
        ControlFlow::Continue(state)
    }

    pub(crate) fn children(&self) -> &[Meta] {
        match self {
            Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas) => metas,
            _ => &[],
        }
    }

    pub(crate) fn visible<'a>(
        metas: impl IntoIterator<Item = &'a Meta>,
    ) -> impl Iterator<Item = &'a Meta> {
        let mut metas = metas.into_iter();
        from_fn(move || {
            loop {
                let meta = metas.next()?;
                match meta {
                    Meta::Hide => loop {
                        if let Meta::Show = metas.next()? {
                            break;
                        }
                    },
                    meta => return Some(meta),
                }
            }
        })
    }

    fn descend_one<'a, T, S>(
        &'a self,
        mut state: S,
        hidden: bool,
        depth: usize,
        down: &mut impl FnMut(S, &'a Self) -> ControlFlow<T, S>,
        up: &mut impl FnMut(S, &'a Self) -> ControlFlow<T, S>,
    ) -> ControlFlow<T, S> {
        state = down(state, self)?;
        if depth > 0 {
            if hidden {
                for child in self.children() {
                    state = child.descend_one(state, hidden, depth - 1, down, up)?;
                }
            } else {
                for child in Self::visible(self.children()) {
                    state = child.descend_one(state, hidden, depth - 1, down, up)?;
                }
            }
        }
        up(state, self)
    }
}

impl Text {
    pub fn as_str(&self) -> &str {
        match self {
            Text::Static(value) => value,
            Text::Shared(value) => value,
        }
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl hash::Hash for Text {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Text::Static(value) => value,
            Text::Shared(value) => value,
        }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Text::Static(value) => fmt::Display::fmt(value, f),
            Text::Shared(value) => fmt::Display::fmt(value, f),
        }
    }
}

impl FromIterator<char> for Text {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        Text::Shared(iter.into_iter().collect::<String>().into())
    }
}

impl From<&'static str> for Text {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<Cow<'static, str>> for Text {
    fn from(value: Cow<'static, str>) -> Self {
        match value {
            Cow::Borrowed(value) => Self::Static(value),
            Cow::Owned(value) => Self::Shared(value.into()),
        }
    }
}

macro_rules! from {
    ($type: ty) => {
        impl From<$type> for Text {
            fn from(value: $type) -> Self {
                Self::Shared(value.into())
            }
        }
    };
}

from!(String);
from!(Box<str>);
from!(Arc<str>);

pub(crate) fn prefix(value: &str) -> Prefix {
    let mut value = value.chars();
    if value.next().is_none() {
        Prefix::None
    } else if value.next().is_none() {
        Prefix::Short
    } else {
        Prefix::Long
    }
}
