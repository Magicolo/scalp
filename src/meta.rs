use crate::parse::Key;
use core::num::NonZeroUsize;
use std::{borrow::Cow, iter::from_fn, ops::ControlFlow, slice::from_ref};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    Plain,
    Short,
    Long,
}

#[derive(Debug, Clone)]
pub enum Meta {
    Name(Name, Cow<'static, str>),
    Position(usize),
    Version(Cow<'static, str>),
    License(Cow<'static, str>, Cow<'static, str>),
    Author(Cow<'static, str>),
    Help(Cow<'static, str>),
    Line,
    Usage(Cow<'static, str>),
    Summary(Cow<'static, str>),
    Home(Cow<'static, str>),
    Repository(Cow<'static, str>),
    Note(Cow<'static, str>),
    Type(Cow<'static, str>),
    Valid(Cow<'static, str>),
    Require(Cow<'static, str>),
    Many(Option<NonZeroUsize>),
    Default(Cow<'static, str>),
    Environment(Cow<'static, str>),
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
        [
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
            Meta::Position(value) => Meta::Position(*value),
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

    pub(crate) fn require(&self) -> Option<Cow<'static, str>> {
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

    pub(crate) fn key(&self) -> Option<Key> {
        let control = Self::descend(
            from_ref(self),
            (None, None, None, None, false),
            false,
            1,
            |state, meta| {
                ControlFlow::<(), _>::Continue(match meta {
                    Meta::Verb(_) | Meta::Option(_) => (state.0, state.1, state.2, state.3, true),
                    Meta::Group(_) => (state.0, state.1, state.2, state.3, false),
                    Meta::Name(Name::Plain, value) if state.4 => {
                        (state.0.or(Some(value)), state.1, state.2, state.3, state.4)
                    }
                    Meta::Name(Name::Short, value) if state.4 => {
                        (state.0, state.1.or(Some(value)), state.2, state.3, state.4)
                    }
                    Meta::Name(Name::Long, value) if state.4 => {
                        (state.0, state.1, state.2.or(Some(value)), state.3, state.4)
                    }
                    Meta::Position(value) if state.4 => {
                        (state.0, state.1, state.2, state.3.or(Some(value)), state.4)
                    }
                    _ => state,
                })
            },
            |state, _| ControlFlow::Continue(state),
        );
        match control {
            ControlFlow::Continue((Some(value), _, _, _, _)) => Some(Key::Name(value.clone())),
            ControlFlow::Continue((_, Some(value), _, _, _)) => Some(Key::Name(value.clone())),
            ControlFlow::Continue((_, _, Some(value), _, _)) => Some(Key::Name(value.clone())),
            ControlFlow::Continue((_, _, _, Some(value), _)) => Some(Key::Index(*value)),
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
        from_fn(move || loop {
            let meta = metas.next()?;
            match meta {
                Meta::Hide => loop {
                    if let Meta::Show = metas.next()? {
                        break;
                    }
                },
                meta => return Some(meta),
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
