use crate::{
    error::Error, help, meta::Meta, spell::Spell, stack::Stack, style, AUTHOR, BREAK, HELP,
    LICENSE, MASK, SHIFT, VERSION,
};
use core::{cmp::min, marker::PhantomData, num::NonZeroUsize};
use regex::RegexSet;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    str::FromStr,
};

pub struct Context<'a> {
    arguments: &'a mut VecDeque<Cow<'static, str>>,
    environment: &'a mut HashMap<Cow<'static, str>, Cow<'static, str>>,
    short: &'a str,
    long: &'a str,
    set: Option<&'a RegexSet>,
    key: Option<&'a Key>,
    meta: Option<&'a Meta>,
    style: &'a dyn style::Style,
    index: Option<usize>,
}

pub struct Parser<P> {
    pub(crate) short: Cow<'static, str>,
    pub(crate) long: Cow<'static, str>,
    pub(crate) parse: P,
    pub(crate) style: Box<dyn style::Style>,
}

#[derive(Default)]
pub(crate) struct Indices {
    pub indices: HashMap<Cow<'static, str>, usize>,
    pub positions: Vec<usize>,
    pub swizzles: HashSet<char>,
}

pub struct Node<P> {
    pub(crate) indices: Indices,
    pub(crate) meta: Meta,
    pub(crate) parse: P,
}

pub struct With<P> {
    pub(crate) parse: P,
    pub(crate) set: RegexSet,
    pub(crate) meta: Meta,
}

pub struct Value<T> {
    pub(crate) tag: Option<Cow<'static, str>>,
    pub(crate) _marker: PhantomData<T>,
}

pub struct Many<P, I, N, F> {
    pub(crate) parse: P,
    pub(crate) per: Option<NonZeroUsize>,
    pub(crate) new: N,
    pub(crate) add: F,
    pub(crate) _marker: PhantomData<I>,
}

pub struct Map<P, F>(pub(crate) P, pub(crate) F);
pub struct Require<P>(pub(crate) P);
pub struct Default<P, T>(pub(crate) P, pub(crate) T);
pub struct Environment<P, F>(pub(crate) P, pub(crate) Cow<'static, str>, pub(crate) F);
pub struct At<P = ()>(pub(crate) P);

#[derive(Clone, PartialEq)]
pub enum Key {
    Index(usize),
    Name(Cow<'static, str>),
}

pub trait Parse {
    type State;
    type Value;
    fn initialize(&self, context: Context) -> Result<Self::State, Error>;
    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error>;
    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error>;
}

pub trait Any<T> {
    fn any(self) -> Option<T>;
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Index(position) => write!(f, "[{}]", position),
            Key::Name(name) => write!(f, "{}", name),
        }
    }
}

impl From<&'static str> for Key {
    fn from(name: &'static str) -> Self {
        Key::Name(name.into())
    }
}

impl From<String> for Key {
    fn from(name: String) -> Self {
        Key::Name(name.into())
    }
}

impl From<usize> for Key {
    fn from(position: usize) -> Self {
        Key::Index(position)
    }
}

impl<T: Stack> Stack for At<T> {
    const COUNT: usize = T::COUNT;
    type Push<U> = At<T::Push<U>>;
    type Pop = At<T::Pop>;
    type Clear = At<T::Clear>;
    type Item = T::Item;

    fn push<U>(self, item: U) -> Self::Push<U> {
        At(self.0.push(item))
    }

    fn pop(self) -> (Self::Item, Self::Pop) {
        let pair = self.0.pop();
        (pair.0, At(pair.1))
    }

    fn clear(self) -> Self::Clear {
        At(self.0.clear())
    }
}

impl<'a> Context<'a> {
    fn own(&mut self) -> Context {
        Context {
            arguments: self.arguments,
            environment: self.environment,
            short: self.short,
            long: self.long,
            set: self.set,
            key: self.key,
            meta: self.meta,
            index: self.index,
            style: self.style,
        }
    }

    fn key(&mut self, swizzles: &HashSet<char>) -> Result<Option<Cow<'static, str>>, Error> {
        let Some(key) = self.arguments.pop_front() else {
            return Ok(None);
        };

        self.index = None;
        self.key = None;
        if key.starts_with(self.short) && !key.starts_with(self.long) {
            let counts = (key.chars().count(), self.short.chars().count());
            if counts.0 > counts.1 + 1 {
                for key in key.chars().skip(counts.1) {
                    if swizzles.contains(&key) {
                        self.arguments
                            .push_front(Cow::Owned(format!("{}{key}", self.short)));
                    } else {
                        return Err(Error::InvalidSwizzleOption(key));
                    }
                }
                return self.key(swizzles);
            }
        }
        Ok(Some(key))
    }

    fn missing_option(&self) -> Error {
        Error::MissingOptionValue(self.type_name().cloned(), self.key.cloned())
    }

    fn missing_required(&self) -> Error {
        Error::MissingRequiredValue(self.key.cloned())
    }

    fn duplicate_option(&self) -> Error {
        Error::DuplicateOption(self.key.cloned())
    }

    fn invalid_option(&self, value: Cow<'static, str>) -> Error {
        Error::InvalidOptionValue(value, self.key.cloned())
    }

    fn failed_parse(&self, value: Cow<'static, str>) -> Error {
        Error::FailedToParseOptionValue(value, self.type_name().cloned(), self.key.cloned())
    }

    fn restore(&mut self, key: Cow<'static, str>) {
        self.arguments.push_front(key)
    }

    fn type_name(&self) -> Option<&Cow<'static, str>> {
        self.meta.and_then(|meta| meta.type_name(1))
    }

    fn with<'b>(
        &'b mut self,
        meta: Option<&'b Meta>,
        set: Option<&'b RegexSet>,
        key: Option<&'b Key>,
        index: Option<usize>,
    ) -> Context {
        let mut state = self.own();
        if let Some(meta) = meta {
            state.meta = Some(meta);
        }
        if let Some(set) = set {
            state.set = Some(set);
        }
        if let Some(key) = key {
            state.key = Some(key);
        }
        if let Some(index) = index {
            state.index = Some(index);
        }
        state
    }
}

impl<T, P: Parse<Value = Option<T>>> Parser<P> {
    pub fn parse(&self) -> Result<T, Error> {
        self.parse_with(std::env::args().skip(1), std::env::vars())
    }

    pub fn parse_with<
        A: Into<Cow<'static, str>>,
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    >(
        &self,
        arguments: impl IntoIterator<Item = A>,
        environment: impl IntoIterator<Item = (K, V)>,
    ) -> Result<T, Error> {
        let mut arguments = arguments
            .into_iter()
            .map(Into::into)
            .filter(|argument| !argument.chars().all(char::is_whitespace))
            .collect();
        let mut environment = environment
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .filter(|(key, _)| !key.chars().all(char::is_whitespace))
            .collect();
        let mut context = Context {
            arguments: &mut arguments,
            environment: &mut environment,
            short: &self.short,
            long: &self.long,
            set: None,
            key: None,
            index: None,
            meta: None,
            style: &*self.style,
        };
        let state = self.parse.initialize(context.own())?;
        let state = self.parse.parse(state, context.own())?;
        let value = self
            .parse
            .finalize(state, context)?
            .ok_or(Error::FailedToParseArguments)?;
        if arguments.is_empty() {
            Ok(value)
        } else {
            Err(Error::ExcessArguments(arguments))
        }
    }
}

impl<P: Parse + ?Sized> Parse for Box<P> {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    #[inline]
    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

    #[inline]
    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        P::finalize(self, state, context)
    }
}

impl<P: Parse + ?Sized> Parse for &P {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    #[inline]
    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

    #[inline]
    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        P::finalize(self, state, context)
    }
}

impl<P: Parse + ?Sized> Parse for &mut P {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    #[inline]
    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

    #[inline]
    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        P::finalize(self, state, context)
    }
}

impl<P: Parse> Parse for Node<P> {
    type State = Option<P::Value>;
    type Value = Option<P::Value>;

    fn initialize(&self, _: Context) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        if state.is_some() {
            return Err(Error::DuplicateNode);
        }

        let mut run = || {
            let mut outer = self.parse.initialize(context.own())?;
            if self.indices.indices.is_empty() && self.indices.positions.is_empty() {
                return self.parse.finalize(outer, context.own());
            }

            let mut positions = self.indices.positions.iter().copied().enumerate();
            while let Some(key) = context.key(&self.indices.swizzles)? {
                match self.indices.indices.get(&key).copied() {
                    Some(HELP) => return Err(Error::Help(None)),
                    Some(VERSION) => return Err(Error::Version(None)),
                    Some(LICENSE) => return Err(Error::License(None)),
                    Some(AUTHOR) => return Err(Error::Author(None)),
                    Some(BREAK) => break,
                    Some(index) => {
                        let key = Key::Name(key);
                        outer = self.parse.parse(
                            outer,
                            context.with(Some(&self.meta), None, Some(&key), Some(index)),
                        )?
                    }
                    None => match positions.next() {
                        Some((i, index)) => {
                            context.restore(key);
                            let key = Key::Index(i);
                            outer = self.parse.parse(
                                outer,
                                context.with(Some(&self.meta), None, Some(&key), Some(index)),
                            )?
                        }
                        None => {
                            let suggestions = Spell::new().suggest(
                                &key,
                                self.indices.indices.keys().cloned(),
                                min(key.len() / 3, 3),
                            );
                            return Err(Error::UnrecognizedArgument(key, suggestions));
                        }
                    },
                };
            }
            self.parse.finalize(outer, context.own())
        };
        match run() {
            Ok(values) => Ok(Some(values)),
            Err(error) => Err(fill(error, &self.meta, context.style)),
        }
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

impl<P: Parse> Parse for With<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, mut context: Context) -> Result<Self::State, Error> {
        self.parse
            .initialize(context.with(Some(&self.meta), Some(&self.set), None, None))
            .map_err(|error| fill(error, &self.meta, context.style))
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        self.parse
            .parse(
                state,
                context.with(Some(&self.meta), Some(&self.set), None, None),
            )
            .map_err(|error| fill(error, &self.meta, context.style))
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        self.parse
            .finalize(
                state,
                context.with(Some(&self.meta), Some(&self.set), None, None),
            )
            .map_err(|error| fill(error, &self.meta, context.style))
    }
}

fn fill<S: style::Style + ?Sized>(error: Error, meta: &Meta, style: &S) -> Error {
    match error {
        Error::Help(None) => Error::Help(help::help(meta, style)),
        Error::Version(None) => Error::Version(help::version(meta, 1)),
        Error::License(None) => Error::License(help::license(meta, 1)),
        Error::Author(None) => Error::Author(help::author(meta, 1)),
        _ => error,
    }
}

impl<P: Parse, T, F: Fn(P::Value) -> Result<T, Error>> Parse for Map<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        self.0.initialize(context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        self.0.parse(state, context)
    }

    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        self.1(self.0.finalize(state, context)?).map_err(Into::into)
    }
}

impl<T, P: Parse<Value = Option<T>>> Parse for Require<P> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        self.0.initialize(context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        self.0.parse(state, context)
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        match self.0.finalize(state, context.own())? {
            Some(value) => Ok(value),
            None => Err(context.missing_required()),
        }
    }
}

impl<T, F: Fn() -> T, P: Parse<Value = Option<T>>> Parse for Default<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        self.0.initialize(context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        self.0.parse(state, context)
    }

    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        match self.0.finalize(state, context)? {
            Some(value) => Ok(value),
            None => Ok(self.1()),
        }
    }
}

impl<T, F: Fn(&str) -> Option<T>, P: Parse<Value = Option<T>>> Parse for Environment<P, F> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        self.0.initialize(context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        self.0.parse(state, context)
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        match self.0.finalize(state, context.own())? {
            Some(value) => Ok(Some(value)),
            None => match context.environment.get(&self.1) {
                Some(value) => match self.2(value) {
                    Some(value) => Ok(Some(value)),
                    None => Err(Error::FailedToParseEnvironmentVariable(
                        self.1.clone(),
                        value.clone(),
                        context.type_name().cloned(),
                        context.key.cloned(),
                    )),
                },
                None => Ok(None),
            },
        }
    }
}

impl<T: FromStr> Parse for Value<T> {
    type State = Option<T>;
    type Value = Option<T>;

    fn initialize(&self, _: Context) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        if state.is_some() {
            return Err(context.duplicate_option());
        }
        let argument = match (context.arguments.pop_front(), &self.tag, &mut context.index) {
            (Some(argument), _, _) => argument,
            (None, Some(tag), Some(index)) if *index == 0 => match tag.parse::<T>() {
                Ok(value) => {
                    *index += 1;
                    return Ok(Some(value));
                }
                Err(_) => return Err(context.failed_parse(tag.clone())),
            },
            _ => return Err(context.missing_option()),
        };
        match (argument.parse::<T>(), &self.tag, &mut context.index) {
            (Ok(value), _, _) => match context.set {
                Some(set) if set.is_empty() || set.is_match(&argument) => Ok(Some(value)),
                Some(_) => Err(context.invalid_option(argument)),
                None => Ok(Some(value)),
            },
            (Err(_), Some(tag), Some(index)) if *index == 0 => {
                context.arguments.push_front(argument);
                *index += 1;
                Ok(Some(
                    tag.parse::<T>()
                        .map_err(|_| context.failed_parse(tag.clone()))?,
                ))
            }
            (Err(_), _, _) => Err(context.failed_parse(argument)),
        }
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

impl<T, P: Parse<Value = Option<T>>, I, N: Fn() -> I, F: Fn(&mut I, T)> Parse for Many<P, I, N, F> {
    type State = Option<I>;
    type Value = Option<I>;

    fn initialize(&self, _: Context) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        let mut items = state.unwrap_or_else(&self.new);
        let mut index = 0;
        let count = self.per.map_or(usize::MAX, NonZeroUsize::get);
        let error = loop {
            if index >= count {
                break None;
            }
            let state = match self.parse.initialize(context.own()) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let state = match self.parse.parse(state, context.own()) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let item = match self.parse.finalize(state, context.own()) {
                Ok(Some(item)) => item,
                Ok(None) => break None,
                Err(error) => break Some(error),
            };
            (self.add)(&mut items, item);
            index += 1;
        };
        if index == 0 {
            match error {
                Some(error) => Err(error),
                None => Err(context.missing_option()),
            }
        } else {
            Ok(Some(items))
        }
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

macro_rules! at {
    ($($name: ident, $index: tt),*) => {
        impl< $($name: Parse,)*> Parse for At<($($name,)*)> {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, mut _context: Context) -> Result<Self::State, Error> {
                Ok(($(self.0.$index.initialize(_context.own())?,)*))
            }

            fn parse(&self, mut _state: Self::State, mut _context: Context) -> Result<Self::State, Error> {
                let Some(index) = _context.index else { return Err(Error::MissingIndex); };
                match index & MASK {
                    $($index => _state.$index = self.0.$index.parse(_state.$index, _context.with(None, None, None, Some(index >> SHIFT)))?,)*
                    index => return Err(Error::InvalidIndex(index)),
                };
                #[allow(unreachable_code)]
                Ok(_state)
            }

            fn finalize(&self, _state: Self::State, mut _context: Context) -> Result<Self::Value, Error> {
                Ok(($(self.0.$index.finalize(_state.$index, _context.own())?,)*))
            }
        }

        impl<T $(, $name: Into<T>)*> Any<T> for ($(Option<$name>,)*) {
            #[inline]
            fn any(self) -> Option<T> {
                $(if let Some(value) = self.$index {
                    return Some(value.into());
                })*
                None
            }
        }
    };
}

at!();
at!(P0, 0);
at!(P0, 0, P1, 1);
at!(P0, 0, P1, 1, P2, 2);
at!(P0, 0, P1, 1, P2, 2, P3, 3);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12, 12
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26, P27, 27
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26, P27, 27, P28, 28
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26, P27, 27, P28, 28, P29, 29
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26, P27, 27, P28, 28, P29, 29, P30, 30
);
at!(
    P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7, P8, 8, P9, 9, P10, 10, P11, 11, P12,
    12, P13, 13, P14, 14, P15, 15, P16, 16, P17, 17, P18, 18, P19, 19, P20, 20, P21, 21, P22, 22,
    P23, 23, P24, 24, P25, 25, P26, 26, P27, 27, P28, 28, P29, 29, P30, 30, P31, 31
);
