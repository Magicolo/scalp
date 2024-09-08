use crate::{
    case::Case,
    error::Error,
    help,
    meta::{Meta, Text},
    spell::Spell,
    stack::Stack,
    style::Format,
    Options, MASK, SHIFT,
};
use core::{cmp::min, marker::PhantomData, num::NonZeroUsize};
use orn::*;
use regex::Regex;
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    ops::ControlFlow,
    str::FromStr,
};

pub(crate) const PREFIX: char = '-';
pub(crate) const TAG: &str = "true";

// TODO: Allow for non-static Cows by carrying another lifetime parameter.
pub struct Context<'a> {
    arguments: &'a mut VecDeque<Argument>,
    environment: &'a mut HashMap<Text, Text>,
    swizzles: &'a mut Vec<char>,
    path: &'a mut Vec<Key>,
    names: &'a mut Vec<Name>,
    root: Option<&'a Meta>,
    meta: Option<&'a Meta>,
    case: Case,
    prefix: char,
    index: Option<usize>,
}

#[derive(Clone)]
pub struct Parser<P>(pub(crate) P);

#[derive(Clone)]
pub struct Node<P>(pub(crate) P);

#[derive(Clone)]
pub struct With<P> {
    pub(crate) parse: P,
    pub(crate) meta: Meta,
    pub(crate) prefix: char,
    pub(crate) case: Case,
}

#[derive(Default)]
pub struct Value<T> {
    pub(crate) tag: Option<Text>,
    pub(crate) _marker: PhantomData<T>,
}

pub struct Many<P, I, N, F> {
    pub(crate) parse: P,
    pub(crate) per: Option<NonZeroUsize>,
    pub(crate) new: N,
    pub(crate) add: F,
    pub(crate) _marker: PhantomData<I>,
}

#[derive(Clone)]
pub struct Map<P, F>(pub(crate) P, pub(crate) F);
#[derive(Clone)]
pub struct Require<P>(pub(crate) P);
#[derive(Clone)]
pub struct Default<P, T>(pub(crate) P, pub(crate) T);
#[derive(Clone)]
pub struct Environment<P>(pub(crate) P, pub(crate) Text);
#[derive(Clone)]
pub struct At<P = ()>(pub(crate) P);

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Index(usize),
    Name(Text),
}

enum Argument {
    String(Text),
    Swizzle(char),
}

struct Name(Text, Option<Case>);

struct KeyFinder<'a> {
    key: &'a str,
    metas: &'a [Meta],
    names: &'a mut Vec<Name>,
    case: Option<Case>,
    verb: bool,
    option: bool,
    r#break: bool,
    swizzle: bool,
    position: usize,
    root: bool,
}

enum KeyResult {
    Help,
    Version,
    Author,
    License,
    Index(usize),
    Name(Text, usize),
    Break(bool),
    Error(Error),
    Unrecognized,
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

impl Format for Key {
    fn width(&self) -> usize {
        match self {
            Key::Index(position) if *position < 10 => 3,
            Key::Index(_) => 4,
            Key::Name(name) => name.len(),
        }
    }

    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
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
            swizzles: self.swizzles,
            path: self.path,
            names: self.names,
            prefix: self.prefix,
            root: self.root,
            case: self.case,
            meta: self.meta,
            index: self.index,
        }
    }

    fn key(&mut self, position: usize) -> Result<Option<(Key, usize)>, Error> {
        self.index = None;
        self.names.clear();
        let Some(argument) = self.arguments.pop_front() else {
            return Ok(None);
        };
        let metas = self.meta.map_or([].as_ref(), Meta::children);
        let result = match &argument {
            Argument::String(key) => {
                let mut characters = key.chars();
                match characters.next() {
                    Some(first) if first == self.prefix => match characters.next() {
                        Some(second) if second == self.prefix => KeyFinder {
                            key: &key[first.len_utf8() + second.len_utf8()..],
                            names: self.names,
                            case: Some(self.case),
                            verb: false,
                            option: true,
                            r#break: true,
                            metas,
                            position: usize::MAX,
                            swizzle: false,
                            root: true,
                        }
                        .find(),
                        Some(second) => {
                            let mut swizzle = false;
                            for character in characters.rev() {
                                self.arguments.push_front(Argument::Swizzle(character));
                                swizzle = true;
                            }
                            KeyFinder {
                                key: &key[first.len_utf8()..first.len_utf8() + second.len_utf8()],
                                names: self.names,
                                case: None,
                                verb: false,
                                option: true,
                                r#break: false,
                                metas,
                                position: usize::MAX,
                                swizzle,
                                root: true,
                            }
                            .find()
                        }
                        None => KeyResult::Error(Error::EmptyShortOption),
                    },
                    Some(_) => match characters.next() {
                        Some(_) => KeyFinder {
                            key,
                            names: self.names,
                            case: Some(self.case),
                            verb: true,
                            option: false,
                            r#break: false,
                            metas,
                            position,
                            swizzle: false,
                            root: true,
                        }
                        .find(),
                        None => KeyFinder {
                            key,
                            names: self.names,
                            case: None,
                            verb: true,
                            option: false,
                            r#break: false,
                            metas,
                            position,
                            swizzle: false,
                            root: true,
                        }
                        .find(),
                    },
                    None => KeyResult::Error(Error::EmptyArgument),
                }
            }
            Argument::Swizzle(key) => KeyFinder {
                key: key.encode_utf8(&mut [0; 4]),
                names: self.names,
                case: None,
                verb: false,
                option: true,
                r#break: false,
                metas,
                position: usize::MAX,
                swizzle: true,
                root: true,
            }
            .find(),
        };

        match result {
            KeyResult::Help => Err(Error::Help(None)),
            KeyResult::Version => Err(Error::Version(None)),
            KeyResult::Author => Err(Error::Author(None)),
            KeyResult::License => Err(Error::License(None)),
            KeyResult::Break(restore) => {
                if restore {
                    self.arguments.push_front(argument);
                }
                Ok(None)
            }
            KeyResult::Error(error) => Err(error),
            KeyResult::Name(name, index) => Ok(Some((Key::Name(name), index))),
            KeyResult::Index(index) => {
                self.arguments.push_front(argument);
                Ok(Some((Key::Index(index), index)))
            }
            KeyResult::Unrecognized => {
                let key = match argument {
                    Argument::String(key) => key,
                    Argument::Swizzle(key) => format!("{}{key}", self.prefix).into(),
                };
                let suggestions = Spell::new().suggest(
                    &key,
                    self.names.iter().map(|name| match name.1 {
                        Some(case) => [self.prefix, self.prefix]
                            .into_iter()
                            .chain(case.convert(name.0.chars()))
                            .collect(),
                        None => [self.prefix].into_iter().chain(name.0.chars()).collect(),
                    }),
                    min(key.len() / 3, 3),
                );
                Err(Error::UnrecognizedArgument(key, suggestions))
            }
        }
    }

    fn fill(&self, error: Error) -> Error {
        let Some(meta) = self.meta else {
            return error;
        };
        match error {
            Error::Help(None) => {
                Error::Help(help::help(self.root.unwrap_or(meta), meta, self.path))
            }
            Error::Version(None) => Error::Version(help::version(meta, 1)),
            Error::License(None) => Error::License(help::license(meta, 1)),
            Error::Author(None) => Error::Author(help::author(meta, 1)),
            _ => error,
        }
    }

    fn is_valid(&self, key: &Text) -> bool {
        let result = self.fold_patterns(true, |_, pattern| {
            if pattern.is_match(key) {
                Err(true)
            } else {
                Ok(false)
            }
        });
        match result {
            Ok(value) | Err(value) => value,
        }
    }

    fn patterns(&self) -> Vec<String> {
        self.map_patterns(|pattern| trim_pattern(pattern).to_string())
    }

    fn map_patterns<'b, T>(&'b self, mut map: impl FnMut(&'b Regex) -> T) -> Vec<T> {
        let mut patterns = Vec::new();
        let _ = self.fold_patterns((), |_, pattern| {
            patterns.push(map(pattern));
            Ok::<(), ()>(())
        });
        patterns
    }

    fn fold_patterns<'b, T, E>(
        &'b self,
        mut state: T,
        mut valid: impl FnMut(T, &'b Regex) -> Result<T, E>,
    ) -> Result<T, E> {
        fn descend<'b, T, E>(
            meta: &'b Meta,
            mut state: T,
            valid: &mut impl FnMut(T, &'b Regex) -> Result<T, E>,
        ) -> Result<T, E> {
            match meta {
                Meta::Valid(pattern) => valid(state, pattern),
                Meta::Group(metas) => {
                    for meta in metas {
                        state = descend(meta, state, valid)?;
                    }
                    Ok(state)
                }
                _ => Ok(state),
            }
        }
        for meta in self.meta.map_or([].as_ref(), Meta::children) {
            state = descend(meta, state, &mut valid)?;
        }
        Ok(state)
    }

    fn missing_option(&self) -> Error {
        Error::MissingOptionValue(self.type_name(), self.path.clone())
    }

    fn missing_required(&self) -> Error {
        let path = self.path.clone();
        match self.meta {
            Some(Meta::Option(_)) => {
                Error::MissingRequiredOption(path, self.meta.and_then(Meta::key))
            }
            _ => Error::MissingRequiredValue(path, self.meta.and_then(Meta::require)),
        }
    }

    fn duplicate_verb(&self) -> Error {
        Error::DuplicateVerb(self.path.clone())
    }

    fn duplicate_option(&self) -> Error {
        Error::DuplicateOption(self.path.clone())
    }

    fn invalid_option(&self, value: Text) -> Error {
        Error::InvalidOptionValue(value, self.patterns(), self.path.clone())
    }

    fn failed_parse(&self, value: Text) -> Error {
        Error::FailedToParseOptionValue(value, self.type_name(), self.path.clone())
    }

    // fn restore(&mut self, key: Text) {
    //     self.arguments.push_front(key)
    // }

    fn type_name(&self) -> Option<Text> {
        let meta = self.meta?;
        let mut name = None;
        for meta in Meta::visible(meta.children()) {
            if let Meta::Type(value) = meta {
                name = Some(value);
            }
        }
        name.cloned()
    }

    fn at(&mut self, index: usize) -> Context {
        let mut state = self.own();
        state.index = Some(index);
        state
    }

    fn with<'b>(&'b mut self, meta: &'b Meta, prefix: char, case: Case) -> Context<'b> {
        let mut context = self.own();
        context.root = context.root.or(Some(meta));
        context.meta = Some(meta);
        context.prefix = prefix;
        context.case = case;
        context
    }
}

impl<T, P: Parse<Value = Option<T>>> Parser<P> {
    pub fn parse(&self) -> Result<T, Error> {
        self.parse_with(std::env::args().skip(1), std::env::vars())
    }

    pub fn parse_with<A: Into<Text>, K: Into<Text>, V: Into<Text>>(
        &self,
        arguments: impl IntoIterator<Item = A>,
        environment: impl IntoIterator<Item = (K, V)>,
    ) -> Result<T, Error> {
        let mut arguments = arguments
            .into_iter()
            .map(Into::into)
            .filter(|argument| !argument.chars().all(char::is_whitespace))
            .map(Argument::String)
            .collect();
        let mut environment = environment
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .filter(|(key, _)| !key.chars().all(char::is_whitespace))
            .collect();
        let mut context = Context {
            arguments: &mut arguments,
            environment: &mut environment,
            swizzles: &mut Vec::new(),
            path: &mut Vec::new(),
            names: &mut Vec::new(),
            prefix: PREFIX,
            case: Case::Kebab { upper: false },
            index: None,
            root: None,
            meta: None,
        };
        let state = self.0.initialize(context.own())?;
        let state = self.0.parse(state, context.own())?;
        let value = self
            .0
            .finalize(state, context)?
            .ok_or(Error::FailedToParseArguments)?;
        if arguments.is_empty() {
            Ok(value)
        } else {
            Err(Error::ExcessArguments(
                arguments
                    .into_iter()
                    .filter_map(|argument| match argument {
                        Argument::String(argument) => Some(argument),
                        Argument::Swizzle(_) => None,
                    })
                    .collect(),
            ))
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
            return Err(context.duplicate_verb());
        }

        let mut outer: <P as Parse>::State = self.0.initialize(context.own())?;
        let mut position = 0;
        while let Some((key, index)) = context.key(position)? {
            if let Key::Index(_) = key {
                position += 1;
            }
            context.path.push(key);
            outer = self.0.parse(outer, context.at(index))?;
            context.path.pop();
        }
        Ok(Some(self.0.finalize(outer, context.own())?))
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

fn trim_pattern(pattern: &Regex) -> &str {
    pattern
        .as_str()
        .trim_start_matches('^')
        .trim_end_matches('$')
}

impl KeyFinder<'_> {
    fn find(mut self) -> KeyResult {
        let mut index = 0;
        let mut help = None;
        let mut version = None;
        let mut position = Err(self.position);
        if let ControlFlow::Break(result) =
            self.with(self.metas, true)
                .descend(&mut index, &mut help, &mut version, &mut position)
        {
            return result;
        }
        if let Ok(position) = position {
            return KeyResult::Index(position);
        }
        if self.r#break && self.key.is_empty() && index > 0 {
            return KeyResult::Break(false);
        }
        if self.option {
            if let Some(true) = help {
                if self.is("h", None) || self.is("help", self.case) {
                    return KeyResult::Help;
                }
            }
            if let Some(true) = version {
                if self.is("v", None) || self.is("version", self.case) {
                    return KeyResult::Version;
                }
            }
        }
        if self.names.is_empty() {
            KeyResult::Break(true)
        } else {
            KeyResult::Unrecognized
        }
    }

    #[inline]
    fn is(&mut self, name: impl Into<Text>, case: Option<Case>) -> bool {
        let name = name.into();
        let is = match case {
            Some(case) => self.key.chars().eq(case.convert(name.chars())),
            None => self.key == name.as_str(),
        };
        self.names.push(Name(name, case));
        is
    }

    fn with<'a>(&'a mut self, metas: &'a [Meta], root: bool) -> KeyFinder<'a> {
        KeyFinder {
            key: self.key,
            names: self.names,
            case: self.case,
            verb: self.verb,
            option: self.option,
            r#break: self.r#break,
            metas,
            position: self.position,
            swizzle: self.swizzle,
            root,
        }
    }

    fn descend(
        mut self,
        index: &mut usize,
        help: &mut Option<bool>,
        version: &mut Option<bool>,
        position: &mut Result<usize, usize>,
    ) -> ControlFlow<KeyResult> {
        for (i, meta) in self.metas.iter().enumerate() {
            match (meta, self.root) {
                (Meta::Position, false, ..) if *position == Err(*index) => *position = Ok(*index),
                (Meta::Help(_), ..)
                | (Meta::Usage(_), ..)
                | (Meta::Note(_), ..)
                | (Meta::Repository(_), ..)
                | (Meta::Summary(_), ..) => *help = help.or(Some(true)),
                (Meta::Version(_), ..) => *version = version.or(Some(true)),
                (Meta::Name(name), false) if self.is(name.clone(), self.case) => {
                    if self.swizzle {
                        let has = self.metas[i + 1..]
                            .iter()
                            .any(|meta| matches!(meta, Meta::Swizzle));
                        if !has {
                            return ControlFlow::Break(KeyResult::Error(
                                Error::InvalidSwizzleOption(name.clone()),
                            ));
                        }
                    }
                    return ControlFlow::Break(KeyResult::Name(name.clone(), *index));
                }
                (Meta::Swizzle, ..) => self.swizzle = false,
                (Meta::Group(metas), true) => {
                    self.with(metas, self.root)
                        .descend(index, help, version, position)?;
                }
                (Meta::Verb(metas), true) => {
                    if self.verb {
                        self.with(metas, false)
                            .descend(index, help, version, position)?;
                    }
                    *index += 1;
                    *help = help.or(Some(true));
                }
                (Meta::Option(metas), true) => {
                    if self.option {
                        self.with(metas, false)
                            .descend(index, help, version, position)?;
                    }
                    *index += 1;
                    *help = help.or(Some(true));
                }
                (Meta::Options(options), true) if self.option => {
                    match options {
                        Options::Help { short: true, .. } if self.is("h", None) => {
                            return ControlFlow::Break(KeyResult::Help);
                        }
                        Options::Help { long: true, .. } if self.is("help", self.case) => {
                            return ControlFlow::Break(KeyResult::Help);
                        }
                        Options::Version { short: true, .. } if self.is("v", None) => {
                            return ControlFlow::Break(KeyResult::Version);
                        }
                        Options::Version { long: true, .. } if self.is("version", self.case) => {
                            return ControlFlow::Break(KeyResult::Version);
                        }
                        Options::Author { short: true, .. } if self.is("a", None) => {
                            return ControlFlow::Break(KeyResult::Author);
                        }
                        Options::Author { long: true, .. } if self.is("author", self.case) => {
                            return ControlFlow::Break(KeyResult::Author);
                        }
                        Options::License { short: true, .. } if self.is("l", None) => {
                            return ControlFlow::Break(KeyResult::License);
                        }
                        Options::License { long: true, .. } if self.is("license", self.case) => {
                            return ControlFlow::Break(KeyResult::License);
                        }
                        _ => {}
                    }
                    *help = Some(false);
                    *version = Some(false);
                }
                _ => {}
            }
        }
        ControlFlow::Continue(())
    }
}

impl<P: Parse> Parse for With<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(&self.meta, self.prefix, self.case);
        match self.parse.initialize(context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(&self.meta, self.prefix, self.case);
        match self.parse.parse(state, context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        let mut context = context.with(&self.meta, self.prefix, self.case);
        match self.parse.finalize(state, context.own()) {
            Ok(value) => Ok(value),
            Err(error) => Err(context.fill(error)),
        }
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

impl<T: FromStr, P: Parse<Value = Option<T>>> Parse for Environment<P> {
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
                Some(value) => match value.parse::<T>() {
                    Ok(value) => Ok(Some(value)),
                    Err(_) => Err(Error::FailedToParseEnvironmentVariable(
                        self.1.clone(),
                        value.clone(),
                        context.type_name(),
                        context.path.clone(),
                        context.meta.and_then(Meta::key),
                    )),
                },
                None => Ok(None),
            },
        }
    }
}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self {
            tag: self.tag.clone(),
            _marker: PhantomData,
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
            (Some(Argument::String(argument)), _, _) => argument,
            (argument, Some(tag), Some(index)) if *index == 0 => match tag.parse::<T>() {
                Ok(value) => {
                    if let Some(argument) = argument {
                        context.arguments.push_front(argument);
                    }
                    *index += 1;
                    return Ok(Some(value));
                }
                Err(_) => return Err(context.failed_parse(tag.clone())),
            },
            (Some(Argument::Swizzle(argument)), _, _) => {
                return Err(Error::InvalidSwizzleOption(format!("{}", argument).into()))
            }
            _ => return Err(context.missing_option()),
        };
        match (argument.parse::<T>(), &self.tag, &mut context.index) {
            (Ok(value), _, _) => {
                if context.is_valid(&argument) {
                    Ok(Some(value))
                } else {
                    Err(context.invalid_option(argument))
                }
            }
            (Err(_), Some(tag), Some(index)) if *index == 0 => {
                context.arguments.push_front(Argument::String(argument));
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

impl<P: Clone, I, N: Clone, F: Clone> Clone for Many<P, I, N, F> {
    fn clone(&self) -> Self {
        Self {
            parse: self.parse.clone(),
            per: self.per,
            new: self.new.clone(),
            add: self.add.clone(),
            _marker: PhantomData,
        }
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
    ($or: ident $(, $name: ident, $index: tt)*) => {
        impl<$($name: Parse,)*> Parse for At<($($name,)*)> {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, context: Context) -> Result<Self::State, Error> {
                self.0.initialize(context)
            }

            fn parse(&self, mut _state: Self::State, mut _context: Context) -> Result<Self::State, Error> {
                let Some(index) = _context.index else { return Err(Error::MissingIndex); };
                match index & MASK {
                    $($index => _state.$index = self.0.$index.parse(_state.$index, _context.at(index >> SHIFT))?,)*
                    index => return Err(Error::InvalidIndex(index)),
                };
                #[allow(unreachable_code)]
                Ok(_state)
            }

            fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
                self.0.finalize(state, context)
            }
        }

        impl<$($name: Parse,)*> Parse for ($($name,)*) {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, mut _context: Context) -> Result<Self::State, Error> {
                Ok(($(self.$index.initialize(_context.own())?,)*))
            }

            fn parse(&self, _state: Self::State, mut _context: Context) -> Result<Self::State, Error> {
                Ok(($(self.$index.parse(_state.$index, _context.own())?,)*))
            }

            fn finalize(&self, _state: Self::State, mut _context: Context) -> Result<Self::Value, Error> {
                Ok(($(self.$index.finalize(_state.$index, _context.own())?,)*))
            }
        }

        impl<$($name: Parse,)*> Parse for $or<$($name),*> {
            type State = $or<$($name::State),*>;
            type Value = $or<$($name::Value),*>;

            fn initialize(&self, _context: Context) -> Result<Self::State, Error> {
                match self {
                    $($or::$name(value) => Ok($or::$name(value.initialize(_context)?)),)*
                    #[allow(unreachable_patterns)]
                    _ => Err(Error::InvalidParseState),
                }
            }

            fn parse(&self, state: Self::State, _context: Context) -> Result<Self::State, Error> {
                match (self, state) {
                    $(($or::$name(value), $or::$name(state)) => Ok($or::$name(value.parse(state, _context)?)),)*
                    #[allow(unreachable_patterns)]
                    _ => Err(Error::InvalidParseState),
                }
            }

            fn finalize(&self, state: Self::State, _context: Context) -> Result<Self::Value, Error> {
                match (self, state) {
                    $(($or::$name(value), $or::$name(state)) => Ok($or::$name(value.finalize(state, _context)?)),)*
                    #[allow(unreachable_patterns)]
                    _ => Err(Error::InvalidParseState),
                }
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

at!(Or0);
at!(Or1, T0, 0);
at!(Or2, T0, 0, T1, 1);
at!(Or3, T0, 0, T1, 1, T2, 2);
at!(Or4, T0, 0, T1, 1, T2, 2, T3, 3);
at!(Or5, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4);
at!(Or6, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5);
at!(Or7, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6);
at!(Or8, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7);
at!(Or9, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8);
at!(Or10, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9);
at!(Or11, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10);
at!(Or12, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11);
at!(
    Or13, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12
);
at!(
    Or14, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13
);
at!(
    Or15, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14
);
at!(
    Or16, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15
);
at!(
    Or17, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16
);
at!(
    Or18, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17
);
at!(
    Or19, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18
);
at!(
    Or20, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19
);
at!(
    Or21, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20
);
at!(
    Or22, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21
);
at!(
    Or23, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22
);
at!(
    Or24, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23
);
at!(
    Or25, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24
);
at!(
    Or26, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25
);
at!(
    Or27, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26
);
at!(
    Or28, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27
);
at!(
    Or29, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27, T28, 28
);
at!(
    Or30, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27, T28, 28, T29, 29
);
at!(
    Or31, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27, T28, 28, T29, 29, T30, 30
);
at!(
    Or32, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11,
    T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17, T18, 18, T19, 19, T20, 20, T21, 21, T22,
    22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27, T28, 28, T29, 29, T30, 30, T31, 31
);
