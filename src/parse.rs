use regex::RegexSet;

use crate::{
    error::Error, help, meta::Meta, spell::Spell, stack::Stack, AUTHOR, BREAK, HELP, LICENSE, MASK,
    SHIFT, VERSION,
};
use core::{cmp::min, marker::PhantomData, num::NonZeroUsize};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    str::FromStr,
};

pub struct State<'a> {
    arguments: &'a mut VecDeque<Cow<'static, str>>,
    environment: &'a mut HashMap<Cow<'static, str>, Cow<'static, str>>,
    short: &'a str,
    long: &'a str,
    set: Option<&'a RegexSet>,
    key: Option<&'a Cow<'static, str>>,
    meta: Option<&'a Meta>,
    index: Option<usize>,
}

pub struct Parser<P> {
    pub(crate) short: Cow<'static, str>,
    pub(crate) long: Cow<'static, str>,
    pub(crate) parse: P,
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

pub trait Parse {
    type State;
    type Value;
    fn initialize(&self, state: State) -> Result<Self::State, Error>;
    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error>;
    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error>;
}

pub trait Any<T> {
    fn any(self) -> Option<T>;
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

impl<'a> State<'a> {
    fn own(&mut self) -> State {
        State {
            arguments: self.arguments,
            environment: self.environment,
            short: self.short,
            long: self.long,
            set: self.set,
            key: self.key,
            meta: self.meta,
            index: self.index,
        }
    }

    fn key(&mut self, swizzles: &HashSet<char>) -> Result<Option<Cow<'static, str>>, Error> {
        let Some(key) = self.arguments.pop_front() else {
            return Ok(None);
        };

        self.index = None;
        self.key = None;
        if key.len() > self.short.len() + 1
            && key.starts_with(self.short)
            && !key.starts_with(self.long)
        {
            for key in key.chars().skip(self.short.len()) {
                if swizzles.contains(&key) {
                    self.arguments
                        .push_front(Cow::Owned(format!("{}{key}", self.short)));
                } else {
                    return Err(Error::InvalidSwizzleOption(key));
                }
            }
            self.key(swizzles)
        } else {
            Ok(Some(key))
        }
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
        key: Option<&'b Cow<'static, str>>,
        index: Option<usize>,
    ) -> State {
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
    pub fn parse(&mut self) -> Result<T, Error> {
        self.parse_with(std::env::args(), std::env::vars())
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
        let mut state = State {
            arguments: &mut arguments,
            environment: &mut environment,
            short: &self.short,
            long: &self.long,
            set: None,
            key: None,
            index: None,
            meta: None,
        };
        let states = (self.parse.initialize(state.own())?, state.own());
        let states = (self.parse.parse(states)?, state);
        let value = self
            .parse
            .finalize(states)?
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
    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse + ?Sized> Parse for &P {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse + ?Sized> Parse for &mut P {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse> Parse for Node<P> {
    type State = Option<P::Value>;
    type Value = Option<P::Value>;

    fn initialize(&self, _: State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, (inner, mut state): (Self::State, State)) -> Result<Self::State, Error> {
        if inner.is_some() {
            return Err(Error::DuplicateNode);
        }

        let run = || {
            let mut outer = self.parse.initialize(state.own())?;
            if self.indices.indices.is_empty() && self.indices.positions.is_empty() {
                return self.parse.finalize((outer, state));
            }

            let mut positions = self.indices.positions.iter().copied();
            while let Some(key) = state.key(&self.indices.swizzles)? {
                match self.indices.indices.get(&key).copied() {
                    Some(HELP) => return Err(Error::Help(None)),
                    Some(VERSION) => return Err(Error::Version(None)),
                    Some(LICENSE) => return Err(Error::License(None)),
                    Some(AUTHOR) => return Err(Error::Author(None)),
                    Some(BREAK) => break,
                    Some(index) => {
                        outer = self.parse.parse((
                            outer,
                            state.with(Some(&self.meta), None, Some(&key), Some(index)),
                        ))?
                    }
                    None => match positions.next() {
                        Some(index) => {
                            state.restore(key);
                            outer = self.parse.parse((
                                outer,
                                state.with(Some(&self.meta), None, None, Some(index)),
                            ))?
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
            self.parse.finalize((outer, state))
        };
        match run() {
            Ok(values) => Ok(Some(values)),
            Err(error) => Err(fill(error, &self.meta)),
        }
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

impl<P: Parse> Parse for With<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, mut state: State) -> Result<Self::State, Error> {
        self.parse
            .initialize(state.with(Some(&self.meta), Some(&self.set), None, None))
            .map_err(|error| fill(error, &self.meta))
    }

    fn parse(&self, (inner, mut state): (Self::State, State)) -> Result<Self::State, Error> {
        self.parse
            .parse((
                inner,
                state.with(Some(&self.meta), Some(&self.set), None, None),
            ))
            .map_err(|error| fill(error, &self.meta))
    }

    fn finalize(&self, (inner, mut state): (Self::State, State)) -> Result<Self::Value, Error> {
        self.parse
            .finalize((
                inner,
                state.with(Some(&self.meta), Some(&self.set), None, None),
            ))
            .map_err(|error| fill(error, &self.meta))
    }
}

fn fill(error: Error, meta: &Meta) -> Error {
    match error {
        Error::Help(None) => Error::Help(help::help(meta)),
        Error::Version(None) => Error::Version(help::version(meta, 1)),
        Error::License(None) => Error::License(help::license(meta, 1)),
        Error::Author(None) => Error::Author(help::author(meta, 1)),
        _ => error,
    }
}

impl<P: Parse, T, F: Fn(P::Value) -> Result<T, Error>> Parse for Map<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        self.1(self.0.finalize(states)?).map_err(Into::into)
    }
}

impl<T, P: Parse<Value = Option<T>>> Parse for Require<P> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, (inner, mut state): (Self::State, State)) -> Result<Self::Value, Error> {
        match self.0.finalize((inner, state.own()))? {
            Some(value) => Ok(value),
            None => Err(state.missing_required()),
        }
    }
}

impl<T, F: Fn() -> T, P: Parse<Value = Option<T>>> Parse for Default<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        match self.0.finalize(states)? {
            Some(value) => Ok(value),
            None => Ok(self.1()),
        }
    }
}

impl<T, F: Fn(&str) -> Option<T>, P: Parse<Value = Option<T>>> Parse for Environment<P, F> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, (inner, mut state): (Self::State, State)) -> Result<Self::Value, Error> {
        match self.0.finalize((inner, state.own()))? {
            Some(value) => Ok(Some(value)),
            None => match state.environment.get(&self.1) {
                Some(value) => match self.2(value) {
                    Some(value) => Ok(Some(value)),
                    None => Err(Error::FailedToParseEnvironmentVariable(
                        self.1.clone(),
                        value.clone(),
                        state.type_name().cloned(),
                        state.key.cloned(),
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

    fn initialize(&self, _: State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, (inner, mut state): (Self::State, State)) -> Result<Self::State, Error> {
        if inner.is_some() {
            return Err(state.duplicate_option());
        }
        let argument = match (state.arguments.pop_front(), &self.tag, &mut state.index) {
            (Some(argument), _, _) => argument,
            (None, Some(tag), Some(index)) if *index == 0 => match tag.parse::<T>() {
                Ok(value) => {
                    *index += 1;
                    return Ok(Some(value));
                }
                Err(_) => return Err(state.failed_parse(tag.clone())),
            },
            _ => return Err(state.missing_option()),
        };
        if let Some(set) = state.set {
            if !set.is_match(&argument) {
                return Err(state.invalid_option(argument));
            }
        }
        match (argument.parse::<T>(), &self.tag, &mut state.index) {
            (Ok(value), _, _) => Ok(Some(value)),
            (Err(_), Some(tag), Some(index)) if *index == 0 => {
                state.arguments.push_front(argument);
                *index += 1;
                Ok(Some(
                    tag.parse::<T>()
                        .map_err(|_| state.failed_parse(tag.clone()))?,
                ))
            }
            (Err(_), _, _) => Err(state.failed_parse(argument)),
        }
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

impl<T, P: Parse<Value = Option<T>>, I, N: Fn() -> I, F: Fn(&mut I, T)> Parse for Many<P, I, N, F> {
    type State = Option<I>;
    type Value = Option<I>;

    fn initialize(&self, _: State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, (inner, mut state): (Self::State, State)) -> Result<Self::State, Error> {
        let mut items = inner.unwrap_or_else(&self.new);
        let mut index = 0;
        let count = self.per.map_or(usize::MAX, NonZeroUsize::get);
        let error = loop {
            if index >= count {
                break None;
            }
            let inner = match self.parse.initialize(state.own()) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let inner = match self.parse.parse((inner, state.own())) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let item = match self.parse.finalize((inner, state.own())) {
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
                None => Err(state.missing_option()),
            }
        } else {
            Ok(Some(items))
        }
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

macro_rules! at {
    ($($name: ident, $index: tt),*) => {
        impl<$($name: Parse,)*> Parse for At<($($name,)*)> {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, mut _state: State) -> Result<Self::State, Error> {
                Ok(($(self.0.$index.initialize(_state.own())?,)*))
            }

            fn parse(&self, (mut _inner, mut _state): (Self::State, State)) -> Result<Self::State, Error> {
                let Some(index) = _state.index else { return Err(Error::MissingIndex); };
                match index & MASK {
                    $($index => _inner.$index = self.0.$index.parse((_inner.$index, _state.with(None, None, None, Some(index >> SHIFT))))?,)*
                    index => return Err(Error::InvalidIndex(index)),
                };
                #[allow(unreachable_code)]
                Ok(_inner)
            }

            fn finalize(&self, (_inner, mut _state): (Self::State, State)) -> Result<Self::Value, Error> {
                Ok(($(self.0.$index.finalize((_inner.$index, _state.own()))?,)*))
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
