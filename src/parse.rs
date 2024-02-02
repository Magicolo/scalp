use crate::{
    error::Error,
    help::{help, version},
    meta::Meta,
    spell::Spell,
    stack::Stack,
    BREAK, HELP, MASK, SHIFT, VERSION,
};
use std::{
    any::TypeId,
    borrow::Cow,
    cmp::min,
    collections::{HashMap, VecDeque},
    default,
    marker::PhantomData,
    num::NonZeroUsize,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};

#[derive(Debug)]
pub struct State<'a> {
    arguments: &'a mut VecDeque<Cow<'static, str>>,
    environment: &'a HashMap<Cow<'static, str>, Cow<'static, str>>,
    short: &'a str,
    long: &'a str,
    key: Option<&'a Cow<'static, str>>,
    index: usize,
    meta: Option<&'a Meta>,
}

#[derive(Debug)]
pub struct Parser<P> {
    pub(crate) short: Cow<'static, str>,
    pub(crate) long: Cow<'static, str>,
    pub(crate) parse: P,
}

#[derive(Debug, Default)]
pub(crate) struct Indices(pub HashMap<Cow<'static, str>, usize>, pub Vec<usize>);

#[derive(Debug)]
pub struct Node<P> {
    pub(crate) indices: Indices,
    pub(crate) meta: Meta,
    pub(crate) parse: P,
}

#[derive(Debug)]
pub struct With<P> {
    pub(crate) meta: Meta,
    pub(crate) parse: P,
}

#[derive(Debug)]
pub struct Map<P, F>(pub(crate) P, pub(crate) F);

#[derive(Debug)]
pub struct Value<T>(pub(crate) PhantomData<T>);

#[derive(Debug)]
pub struct Function<F>(pub(crate) F);

#[derive(Debug)]
pub struct Many<P, I>(
    pub(crate) P,
    pub(crate) Option<NonZeroUsize>,
    pub(crate) PhantomData<I>,
);

#[derive(Debug)]
pub struct Require<P>(pub(crate) P);

#[derive(Debug)]
pub struct Default<P, T>(pub(crate) P, pub(crate) T);

#[derive(Debug)]
pub struct Environment<P, F>(pub(crate) P, pub(crate) Cow<'static, str>, pub(crate) F);

#[derive(Debug)]
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
            key: self.key,
            index: self.index,
            meta: self.meta,
        }
    }

    fn key(&mut self) -> Option<Cow<'static, str>> {
        let mut key = self.arguments.pop_front()?;
        self.index = 0;
        self.key = None;
        if key.len() > self.short.len() + 1 && key.starts_with(self.short) && !key.starts_with(self.long) {
            for key in key.chars().skip(self.short.len() + 1) {
                self.arguments
                    .push_front(Cow::Owned(format!("{}{key}", self.short)));
            }
            key.to_mut().truncate(2);
        }
        Some(key)
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

    fn restore(&mut self, key: Cow<'static, str>) {
        self.arguments.push_front(key)
    }

    fn type_name(&self) -> Option<&Cow<'static, str>> {
        self.meta.and_then(|meta| meta.type_name(1))
    }

    fn with<'b>(
        &'b mut self,
        meta: Option<&'b Meta>,
        key: Option<&'b Cow<'static, str>>,
        index: Option<usize>,
    ) -> State<'b> {
        let mut state = self.own();
        state.meta = meta.or(state.meta);
        state.key = key.or(state.key);
        state.index = index.unwrap_or(state.index);
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
            key: None,
            index: 0,
            meta: None,
        };
        let states = (self.parse.initialize(state.own())?, state.own());
        let states = (self.parse.parse(states)?, state.own());
        let value = self.parse.finalize(states)?.ok_or(Error::FailedToParseArguments)?;
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

impl<P: Parse + ?Sized> Parse for Rc<P> {
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

impl<P: Parse + ?Sized> Parse for Arc<P> {
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

    fn parse(&self, mut states: (Self::State, State)) -> Result<Self::State, Error> {
        if states.0.is_some() {
            return Err(Error::DuplicateNode);
        }

        let mut run = || {
            let state = self.parse.initialize(states.1.own())?;
            if self.indices.0.is_empty() && self.indices.1.is_empty() {
                return self.parse.finalize((state, states.1.own()));
            }

            let mut states = (state, states.1.own());
            let mut at = 0;
            while let Some(key) = states.1.key() {
                match self.indices.0.get(&key).copied() {
                    Some(HELP) => return Err(Error::Help(None)),
                    Some(VERSION) => return Err(Error::Version(None)),
                    Some(BREAK) => break,
                    Some(index) => {
                        states.0 = self.parse.parse((
                            states.0,
                            states.1.with(Some(&self.meta), Some(&key), Some(index)),
                        ))?
                    }
                    None => match self.indices.1.get(at).copied() {
                        Some(index) => {
                            states.1.restore(key);
                            states.0 = self.parse.parse((
                                states.0,
                                states.1.with(Some(&self.meta), None, Some(index)),
                            ))?;
                            at += 1;
                        }
                        None => {
                            let suggestions = Spell::new().suggest(
                                &key,
                                self.indices.0.keys().cloned(),
                                min(key.len() / 3, 3),
                            );
                            return Err(Error::UnrecognizedArgument(key, suggestions));
                        }
                    },
                };
            }
            self.parse.finalize(states)
        };
        match run() {
            Ok(values) => Ok(Some(values)),
            Err(Error::Help(None)) => Err(Error::Help(help(&self.meta))),
            Err(Error::Version(None)) => Err(Error::Version(version(&self.meta, 1).cloned())),
            Err(error) => Err(error),
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
            .initialize(state.with(Some(&self.meta), None, None))
            .map_err(|error| fill(error, &self.meta))
    }

    fn parse(&self, mut states: (Self::State, State)) -> Result<Self::State, Error> {
        self.parse
            .parse((states.0, states.1.with(Some(&self.meta), None, None)))
            .map_err(|error| fill(error, &self.meta))
    }

    fn finalize(&self, mut states: (Self::State, State)) -> Result<Self::Value, Error> {
        self.parse
            .finalize((states.0, states.1.with(Some(&self.meta), None, None)))
            .map_err(|error| fill(error, &self.meta))
    }
}

fn fill(error: Error, meta: &Meta) -> Error {
    match error {
        Error::Help(None) => Error::Help(help(meta)),
        Error::Version(None) => Error::Version(version(meta, 1).cloned()),
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

    fn finalize(&self, mut states: (Self::State, State)) -> Result<Self::Value, Error> {
        match self.0.finalize((states.0, states.1.own()))? {
            Some(value) => Ok(value),
            None => Err(states.1.missing_required()),
        }
    }
}

impl<T: Clone, P: Parse<Value = Option<T>>> Parse for Default<P, T> {
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
            None => Ok(self.1.clone()),
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

    fn finalize(&self, mut states: (Self::State, State)) -> Result<Self::Value, Error> {
        match self.0.finalize((states.0, states.1.own()))? {
            Some(value) => Ok(Some(value)),
            None => match states.1.environment.get(&self.1) {
                Some(value) => match self.2(value) {
                    Some(value) => Ok(Some(value)),
                    None => Err(Error::FailedToParseEnvironmentVariable(
                        self.1.clone(),
                        value.clone(),
                        states.1.type_name().cloned(),
                        states.1.key.cloned(),
                    )),
                },
                None => Ok(None),
            },
        }
    }
}

impl<T: FromStr + 'static> Parse for Value<T> {
    type State = Option<T>;
    type Value = Option<T>;

    fn initialize(&self, state: State) -> Result<Self::State, Error> {
        Function(|value: &str| T::from_str(value).ok()).initialize(state)
    }

    fn parse(&self, states: (Self::State, State)) -> Result<Self::State, Error> {
        Function(|value: &str| T::from_str(value).ok()).parse(states)
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        Function(|value: &str| T::from_str(value).ok()).finalize(states)
    }
}

impl<T: 'static, F: Fn(&str) -> Option<T>> Parse for Function<F> {
    type State = Option<T>;
    type Value = Option<T>;

    fn initialize(&self, _: State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, mut states: (Self::State, State)) -> Result<Self::State, Error> {
        match states.0 {
            Some(_) => Err(states.1.duplicate_option()),
            None => match states.1.arguments.pop_front() {
                Some(value) => match self.0(&value) {
                    Some(value) => Ok(Some(value)),
                    None if TypeId::of::<bool>() == TypeId::of::<T>() && states.1.index == 0 => {
                        states.1.index += 1;
                        states.1.arguments.push_front(value);
                        Ok(self.0("true"))
                    }
                    None => Err(Error::FailedToParseOptionValue(
                        value,
                        states.1.type_name().cloned(),
                        states.1.key.cloned(),
                    )),
                },
                None if TypeId::of::<bool>() == TypeId::of::<T>() && states.1.index == 0 => {
                    states.1.index += 1;
                    Ok(self.0("true"))
                }
                None => Err(states.1.missing_option()),
            },
        }
    }

    fn finalize(&self, states: (Self::State, State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

impl<T, P: Parse<Value = Option<T>>, I: default::Default + Extend<T>> Parse for Many<P, I> {
    type State = Option<I>;
    type Value = Option<I>;

    fn initialize(&self, _: State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, mut states: (Self::State, State)) -> Result<Self::State, Error> {
        let mut items = states.0.unwrap_or_default();
        let mut index = 0;
        let count = self.1.map_or(usize::MAX, NonZeroUsize::get);
        let error = loop {
            if index >= count {
                break None;
            }
            let state = match self.0.initialize(states.1.own()) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let state = match self.0.parse((state, states.1.own())) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let item = match self.0.finalize((state, states.1.own())) {
                Ok(Some(item)) => item,
                Ok(None) => break None,
                Err(error) => break Some(error),
            };
            items.extend([item]);
            index += 1;
        };
        match (error, index) {
            (_, 1..) => Ok(Some(items)),
            (None, 0) => Err(states.1.missing_option()),
            (Some(error), 0) => Err(error),
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

            fn parse(&self, mut _states: (Self::State, State)) -> Result<Self::State, Error> {
                let index = _states.1.index;
                match index & MASK {
                    $($index => _states.0.$index = self.0.$index.parse((_states.0.$index, _states.1.with(None, None, Some(index >> SHIFT))))?,)*
                    index => return Err(Error::InvalidIndex(index)),
                };
                #[allow(unreachable_code)]
                Ok(_states.0)
            }

            fn finalize(&self, mut _states: (Self::State, State)) -> Result<Self::Value, Error> {
                Ok(($(self.0.$index.finalize((_states.0.$index, _states.1.own()))?,)*))
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
