use crate::{
    error::Error,
    help::{help, version},
    stack::{Count, Pop, Push},
    Meta, BREAK, HELP, MASK, SHIFT, VERSION,
};
use std::{
    any::TypeId,
    borrow::Cow,
    collections::{HashMap, VecDeque},
    default,
    marker::PhantomData,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};

pub struct State<'a> {
    arguments: &'a mut VecDeque<Cow<'static, str>>,
    environment: &'a HashMap<Cow<'static, str>, Cow<'static, str>>,
    short: &'a str,
    long: &'a str,
    index: usize,
}

pub struct Parser<P> {
    pub(crate) short: Cow<'static, str>,
    pub(crate) long: Cow<'static, str>,
    pub(crate) parse: P,
}

pub struct Node<P> {
    pub(crate) indices: HashMap<Cow<'static, str>, usize>,
    pub(crate) meta: Meta,
    pub(crate) parse: P,
}
pub struct Map<P, F>(pub(crate) P, pub(crate) F);
pub struct Value<T>(pub(crate) PhantomData<T>);
pub struct Many<P, I>(
    pub(crate) P,
    pub(crate) Option<usize>,
    pub(crate) PhantomData<I>,
);
pub struct Require<P>(pub(crate) P);
pub struct Default<P, F>(pub(crate) P, pub(crate) F);
pub struct Environment<P>(pub(crate) P, pub(crate) Cow<'static, str>);
pub struct At<P = ()>(pub(crate) P);

pub trait Parse {
    type State;
    type Value;
    fn initialize(&self, state: &State) -> Result<Self::State, Error>;
    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error>;
    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error>;
}

pub trait Any<T> {
    fn any(self) -> Option<T>;
}

impl<T, P: Push<T>> Push<T> for At<P> {
    type Output = At<P::Output>;

    fn push(self, item: T) -> Self::Output {
        At(self.0.push(item))
    }
}

impl<T: Count> Count for At<T> {
    const COUNT: usize = T::COUNT;
}

impl<T: Pop> Pop for At<T> {
    type Item = T::Item;
    type Output = T::Output;

    fn pop(self) -> (Self::Item, Self::Output) {
        self.0.pop()
    }
}

impl<'a> State<'a> {
    pub fn key(&mut self) -> Option<Cow<'static, str>> {
        let Some(key) = self.arguments.pop_front() else {
            return None;
        };
        if key.starts_with(self.short) && !key.starts_with(self.long) {
            for key in key.chars().skip(self.short.len() + 1) {
                self.arguments
                    .push_front(Cow::Owned(format!("{}{key}", self.short)));
            }
        }
        self.index = 0;
        Some(key)
    }

    pub fn value<T: FromStr + 'static>(&mut self) -> Result<Option<T>, Error> {
        match self.arguments.pop_front() {
            Some(value) => match value.parse() {
                Ok(value) => Ok(Some(value)),
                Err(_) if TypeId::of::<bool>() == TypeId::of::<T>() && self.index == 0 => {
                    self.index += 1;
                    self.arguments.push_front(value);
                    Ok("true".parse().ok())
                }
                Err(_) => Err(Error::MissingOptionValue),
            },
            None if TypeId::of::<bool>() == TypeId::of::<T>() && self.index == 0 => {
                self.index += 1;
                Ok("true".parse().ok())
            }
            None => Err(Error::MissingOptionValue),
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn variable<T: FromStr>(&self, key: &str) -> Result<Option<T>, Error> {
        match self.environment.get(key) {
            Some(value) => match value.parse() {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(Error::FailedToParseVariable(value.clone())),
            },
            None => Ok(None),
        }
    }

    pub fn with(&mut self, index: usize) -> State {
        State {
            arguments: self.arguments,
            environment: self.environment,
            short: self.short,
            long: self.long,
            index,
        }
    }
}

impl<P: Parse> Parser<P> {
    pub fn parse(&self) -> Result<P::Value, Error> {
        self.parse_with(std::env::args(), std::env::vars())
    }

    pub fn parse_with(
        &self,
        arguments: impl IntoIterator<Item = impl Into<Cow<'static, str>>>,
        environment: impl IntoIterator<
            Item = (impl Into<Cow<'static, str>>, impl Into<Cow<'static, str>>),
        >,
    ) -> Result<P::Value, Error> {
        let mut arguments = arguments.into_iter().map(Into::into).collect();
        let mut environment = environment
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        let mut state = State {
            arguments: &mut arguments,
            environment: &mut environment,
            short: &self.short,
            long: &self.long,
            index: 0,
        };
        let states = (self.parse.initialize(&state)?, &mut state);
        let states = (self.parse.parse(states)?, &state);
        let value = self.parse.finalize(states)?;
        if arguments.is_empty() {
            Ok(value)
        } else {
            Err(Error::ExcessArguments(arguments))
        }
    }
}

impl Parse for () {
    type State = ();
    type Value = ();

    #[inline]
    fn initialize(&self, _: &State) -> Result<Self::State, Error> {
        Ok(())
    }

    #[inline]
    fn parse(&self, _: (Self::State, &mut State)) -> Result<Self::State, Error> {
        Ok(())
    }

    #[inline]
    fn finalize(&self, _: (Self::State, &State)) -> Result<Self::Value, Error> {
        Ok(())
    }
}

impl<P: Parse + ?Sized> Parse for Box<P> {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse + ?Sized> Parse for Rc<P> {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse + ?Sized> Parse for Arc<P> {
    type State = P::State;
    type Value = P::Value;

    #[inline]
    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        P::initialize(self, state)
    }

    #[inline]
    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        P::parse(self, states)
    }

    #[inline]
    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        P::finalize(self, states)
    }
}

impl<P: Parse> Parse for Node<P> {
    type State = Option<P::Value>;
    type Value = Option<P::Value>;

    fn initialize(&self, _: &State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        if states.0.is_some() {
            return Err(Error::DuplicateNode);
        }
        let mut run = || {
            let mut state = self.parse.initialize(states.1)?;
            while let Some(key) = states.1.key() {
                match self.indices.get(&key).copied() {
                    Some(HELP) => return Err(Error::Help(None)),
                    Some(VERSION) => return Err(Error::Version(None)),
                    Some(BREAK) => break,
                    Some(index) => state = self.parse.parse((state, &mut states.1.with(index)))?,
                    None => return Err(Error::UnrecognizedArgument { name: key }),
                };
            }
            self.parse.finalize((state, states.1))
        };
        match run() {
            Ok(values) => Ok(Some(values)),
            Err(Error::Help(None)) => Err(Error::Help(help(&self.meta).map(Cow::Owned))),
            Err(Error::Version(None)) => Err(Error::Version(version(&self.meta, 1).cloned())),
            Err(error) => Err(error),
        }
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

impl<P: Parse, T, F: Fn(P::Value) -> Result<T, Error>> Parse for Map<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        self.1(self.0.finalize(states)?).map_err(Into::into)
    }
}

impl<T, P: Parse<Value = Option<T>>> Parse for Require<P> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        match self.0.finalize(states)? {
            Some(value) => Ok(value),
            None => Err(Error::MissingRequiredValue),
        }
    }
}

impl<T: Clone, P: Parse<Value = Option<T>>> Parse for Default<P, T> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        match self.0.finalize(states)? {
            Some(value) => Ok(value),
            None => Ok(self.1.clone()),
        }
    }
}

impl<T: FromStr, P: Parse<Value = Option<T>>> Parse for Environment<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, state: &State) -> Result<Self::State, Error> {
        self.0.initialize(state)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        self.0.parse(states)
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        match self.0.finalize((states.0, states.1))? {
            Some(value) => Ok(Some(value)),
            None => states.1.variable(&self.1),
        }
    }
}

impl<T: FromStr + 'static> Parse for Value<T> {
    type State = Option<T>;
    type Value = Option<T>;

    fn initialize(&self, _: &State) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        match states.0 {
            Some(_) => Err(Error::DuplicateOptionValue),
            None => states.1.value(),
        }
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

impl<T, P: Parse<Value = Option<T>>, I: default::Default + Extend<T>> Parse for Many<P, I> {
    type State = I;
    type Value = I;

    fn initialize(&self, _: &State) -> Result<Self::State, Error> {
        Ok(I::default())
    }

    fn parse(&self, mut states: (Self::State, &mut State)) -> Result<Self::State, Error> {
        let mut index = 0;
        let count = self.1.unwrap_or(usize::MAX);
        let error = loop {
            if index >= count {
                break None;
            }
            let state = match self.0.initialize(states.1) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let state = match self.0.parse((state, states.1)) {
                Ok(state) => state,
                Err(error) => break Some(error),
            };
            let item = match self.0.finalize((state, states.1)) {
                Ok(Some(item)) => item,
                Ok(None) => break None,
                Err(error) => break Some(error),
            };
            states.0.extend([item]);
            index += 1;
        };
        match (error, index) {
            (_, 1..) => Ok(states.0),
            (None, 0) => Err(Error::MissingOptionValue),
            (Some(error), 0) => Err(error),
        }
    }

    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error> {
        Ok(states.0)
    }
}

macro_rules! at {
    ($($name: ident, $index: tt),*) => {
        impl<$($name: Parse,)*> Parse for At<($($name,)*)> {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, _state: &State) -> Result<Self::State, Error> {
                Ok(($(self.0.$index.initialize(_state)?,)*))
            }

            fn parse(&self, mut _states: (Self::State, &mut State)) -> Result<Self::State, Error> {
                let index = _states.1.index();
                match index & MASK {
                    $($index => _states.0.$index = self.0.$index.parse((_states.0.$index, &mut _states.1.with(index >> SHIFT)))?,)*
                    _ => {},
                };
                #[allow(unreachable_code)]
                Ok(_states.0)
            }

            fn finalize(&self, _states: (Self::State, &State)) -> Result<Self::Value, Error> {
                Ok(($(self.0.$index.finalize((_states.0.$index, _states.1))?,)*))
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
