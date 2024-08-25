use crate::{
    case::Case,
    error::Error,
    help,
    meta::{Meta, Prefix},
    spell::Spell,
    stack::Stack,
    style::Format,
    Options, AUTHOR, BREAK, HELP, LICENSE, MASK, SHIFT, VERSION,
};
use core::{cmp::min, marker::PhantomData, num::NonZeroUsize};
use orn::*;
use regex::RegexSet;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    ops::ControlFlow,
    str::FromStr,
};

pub(crate) const PREFIX: char = '-';
pub(crate) const TAG: &str = "true";

// TODO: Allow for non-static Cows by carrying another lifetime parameter.
pub struct Context<'a> {
    arguments: &'a mut VecDeque<Argument>,
    environment: &'a mut HashMap<Cow<'static, str>, Cow<'static, str>>,
    swizzles: &'a mut Vec<char>,
    path: &'a mut Vec<Key>,
    set: &'a RegexSet,
    root: Option<&'a Meta>,
    meta: Option<&'a Meta>,
    case: Case,
    prefix: char,
    index: Option<usize>,
}

pub struct Parser<P>(pub(crate) P);

#[derive(Default)]
pub(crate) struct Indices {
    pub indices: HashMap<Cow<'static, str>, usize>,
    pub positions: Vec<usize>,
    pub swizzles: HashSet<char>,
}

pub struct Node<P> {
    pub(crate) indices: Indices,
    pub(crate) parse: P,
}

pub struct With<P> {
    pub(crate) parse: P,
    pub(crate) set: RegexSet,
    pub(crate) meta: Meta,
    pub(crate) prefix: char,
}

#[derive(Default)]
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
pub struct Environment<P>(pub(crate) P, pub(crate) Cow<'static, str>);
pub struct At<P = ()>(pub(crate) P);

#[derive(Clone, PartialEq)]
pub enum Key {
    Index(usize),
    Name(Cow<'static, str>),
}

enum Argument {
    String(Cow<'static, str>),
    Swizzle(char),
}

enum KeyResult {
    Help,
    Version,
    Author,
    License,
    Index(usize),
    Name(Cow<'static, str>, usize),
    Break,
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

impl KeyResult {
    fn or(self, error: Error) -> KeyResult {
        match self {
            KeyResult::Unrecognized => KeyResult::Error(error),
            result => result,
        }
    }
}

impl<'a> Context<'a> {
    fn own(&mut self) -> Context {
        Context {
            arguments: self.arguments,
            environment: self.environment,
            swizzles: self.swizzles,
            path: self.path,
            prefix: self.prefix,
            set: self.set,
            root: self.root,
            case: self.case,
            meta: self.meta,
            index: self.index,
        }
    }

    fn key(&mut self, position: usize) -> Result<Option<(Key, usize)>, Error> {
        self.index = None;
        let Some(argument) = self.arguments.pop_front() else {
            return Ok(None);
        };
        let metas = self.meta.map_or([].as_ref(), Meta::children);
        let result = match argument {
            Argument::String(key) => {
                let mut characters = key.chars();
                match characters.next() {
                    Some(first) if first == self.prefix => match characters.next() {
                        Some(second) if second == self.prefix => find_key_with(
                            &key[first.len_utf8() + second.len_utf8()..],
                            metas,
                            Some(self.case),
                            usize::MAX,
                            false,
                            true,
                            false,
                            true,
                        ),
                        Some(second) => match characters.next() {
                            Some(third) => {
                                self.arguments.push_front(Argument::Swizzle(third));
                                for character in characters {
                                    self.arguments.push_front(Argument::Swizzle(character));
                                }
                                find_key_with(
                                    &key[first.len_utf8()..first.len_utf8() + second.len_utf8()],
                                    metas,
                                    None,
                                    usize::MAX,
                                    true,
                                    false,
                                    false,
                                    true,
                                )
                            }
                            None => find_key_with(
                                &key[first.len_utf8()..first.len_utf8() + second.len_utf8()],
                                metas,
                                None,
                                usize::MAX,
                                false,
                                false,
                                false,
                                true,
                            ),
                        },
                        None => KeyResult::Error(Error::EmptyShortOption),
                    },
                    Some(_) => match characters.next() {
                        Some(_) => find_key_with(
                            &key,
                            metas,
                            Some(self.case),
                            position,
                            false,
                            false,
                            true,
                            false,
                        ),
                        None => {
                            find_key_with(&key, metas, None, position, false, false, true, false)
                        }
                    },
                    None => KeyResult::Error(Error::EmptyArgument),
                }
            }
            Argument::Swizzle(key) => find_key_with(
                key.encode_utf8(&mut [0; 4]),
                metas,
                None,
                usize::MAX,
                true,
                false,
                false,
                true,
            ),
        };

        match result {
            KeyResult::Help => Err(Error::Help(None)),
            KeyResult::Version => Err(Error::Version(None)),
            KeyResult::Author => Err(Error::Author(None)),
            KeyResult::License => Err(Error::License(None)),
            KeyResult::Break => Ok(None),
            KeyResult::Error(error) => Err(error),
            KeyResult::Name(name, index) => Ok(Some((Key::Name(name), index))),
            KeyResult::Index(index) => {
                self.arguments.push_front(argument);
                Ok(Some((Key::Index(index), index)))
            }
            KeyResult::Unrecognized => {
                let key = match argument {
                    Argument::String(key) => key,
                    Argument::Swizzle(key) => Cow::Owned(format!("{}{key}", self.prefix)),
                };
                let suggestions = Spell::new().suggest(
                    &key,
                    // TODO: Collect valid names. Don't forget to add the prefix/es, convert the case, include implicit options and 'break'.
                    self.indices.indices.keys().cloned(),
                    min(key.len() / 3, 3),
                );
                return Err(Error::UnrecognizedArgument(key, suggestions));
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

    fn invalid_argument(&self, key: Cow<'static, str>) -> Error {
        Error::InvalidArgument(
            key,
            self.set
                .patterns()
                .iter()
                .map(|pattern| {
                    pattern
                        .trim_start_matches('^')
                        .trim_end_matches('$')
                        .to_string()
                })
                .collect(),
            self.path.clone(),
        )
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

    fn invalid_option(&self, value: Cow<'static, str>) -> Error {
        Error::InvalidOptionValue(
            value,
            self.set
                .patterns()
                .iter()
                .map(|pattern| pattern.trim_matches(['$', '^']).to_string())
                .collect(),
            self.path.clone(),
        )
    }

    fn failed_parse(&self, value: Cow<'static, str>) -> Error {
        Error::FailedToParseOptionValue(value, self.type_name(), self.path.clone())
    }

    // fn restore(&mut self, key: Cow<'static, str>) {
    //     self.arguments.push_front(key)
    // }

    fn type_name(&self) -> Option<Cow<'static, str>> {
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

    fn with<'b>(&'b mut self, meta: &'b Meta, set: &'b RegexSet, prefix: char) -> Context<'b> {
        let mut context = self.own();
        context.root = context.root.or(Some(meta));
        context.meta = Some(meta);
        context.set = set;
        context.prefix = prefix;
        context
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
            prefix: PREFIX,
            set: &RegexSet::empty(),
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

        let mut outer = self.parse.initialize(context.own())?;
        if self.indices.indices.is_empty() && self.indices.positions.is_empty() {
            return Ok(Some(self.parse.finalize(outer, context)?));
        }

        let mut position = 0;
        while let Some((key, index)) = context.key(position)? {
            if let Key::Index(_) = key {
                position += 1;
            }
            context.path.push(key);
            outer = self.parse.parse(outer, context.at(index))?;
            context.path.pop();
        }
        // let mut positions = self.indices.positions.iter().copied().enumerate();
        // while let Some(key) = context.key(&self.indices.swizzles)? {
        //     let (key, index) = match self.indices.indices.get(&key).copied() {
        //         Some(HELP) => return Err(Error::Help(None)),
        //         Some(VERSION) => return Err(Error::Version(None)),
        //         Some(LICENSE) => return Err(Error::License(None)),
        //         Some(AUTHOR) => return Err(Error::Author(None)),
        //         Some(BREAK) => break,
        //         Some(index) => (Key::Name(key), index),
        //         None => match positions.next() {
        //             Some((i, index)) => {
        //                 context.restore(key);
        //                 (Key::Index(i), index)
        //             }
        //             None => {
        //                 let suggestions = Spell::new().suggest(
        //                     &key,
        //                     self.indices.indices.keys().cloned(),
        //                     min(key.len() / 3, 3),
        //                 );
        //                 return Err(Error::UnrecognizedArgument(key, suggestions));
        //             }
        //         },
        //     };
        //     context.path.push(key);
        //     outer = self.parse.parse(outer, context.at(index))?;
        //     context.path.pop();
        // }
        Ok(Some(self.parse.finalize(outer, context.own())?))
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

#[inline]
fn prefix(key: &str, prefix: char) -> Prefix {
    let mut characters = key.chars();
    if characters.next() == Some(prefix) {
        if characters.next() == Some(prefix) {
            Prefix::Long
        } else {
            Prefix::Short
        }
    } else {
        Prefix::None
    }
}

#[inline]
fn is(key: &str, name: &str, case: Option<Case>) -> bool {
    match case {
        Some(case) => key.chars().eq(case.convert(name.chars())),
        None => key == name,
    }
}

fn find_key_with(
    key: &str,
    metas: &[Meta],
    case: Option<Case>,
    position: usize,
    swizzle: bool,
    r#break: bool,
    verb: bool,
    option: bool,
) -> KeyResult {
    fn descend(
        key: &str,
        metas: &[Meta],
        index: &mut usize,
        help: &mut Option<bool>,
        version: &mut Option<bool>,
        position: &mut Result<usize, usize>,
        mut swizzle: bool,
        case: Option<Case>,
        root: bool,
        verb: bool,
        option: bool,
    ) -> ControlFlow<KeyResult> {
        for (i, meta) in metas.iter().enumerate() {
            match (meta, root, case) {
                (Meta::Position(_), false, ..) if *position == Err(*index) => {
                    *position = Ok(*index)
                }
                (Meta::Help(_), ..)
                | (Meta::Usage(_), ..)
                | (Meta::Note(_), ..)
                | (Meta::Repository(_), ..)
                | (Meta::Summary(_), ..) => *help = help.or(Some(true)),
                (Meta::Version(_), ..) => *version = version.or(Some(true)),
                (Meta::Name(_, name), false, _) if is(key, name, case) => {
                    if swizzle {
                        let has = metas[i + 1..]
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
                (Meta::Swizzle, ..) => swizzle = false,
                (Meta::Group(metas), true, case) => {
                    descend(
                        key, metas, index, help, version, position, swizzle, case, root, verb,
                        option,
                    )?;
                }
                (Meta::Verb(metas), true, case) => {
                    if verb {
                        descend(
                            key, metas, index, help, version, position, swizzle, case, false, verb,
                            option,
                        )?;
                    }
                    *index += 1;
                    *help = help.or(Some(true));
                }
                (Meta::Option(metas), true, case) => {
                    if option {
                        descend(
                            key, metas, index, help, version, position, swizzle, case, false, verb,
                            option,
                        )?;
                    }
                    *index += 1;
                    *help = help.or(Some(true));
                }
                (Meta::Options(options), true, case) if option => {
                    match options {
                        Options::Help { short: true, .. } if key == "h" => {
                            return ControlFlow::Break(KeyResult::Help);
                        }
                        Options::Help { long: true, .. } if is(key, "help", case) => {
                            return ControlFlow::Break(KeyResult::Help);
                        }
                        Options::Version { short: true, .. } if key == "v" => {
                            return ControlFlow::Break(KeyResult::Version);
                        }
                        Options::Version { long: true, .. } if is(key, "version", case) => {
                            return ControlFlow::Break(KeyResult::Version);
                        }
                        Options::Author { short: true, .. } if key == "a" => {
                            return ControlFlow::Break(KeyResult::Author);
                        }
                        Options::Author { long: true, .. } if is(key, "author", case) => {
                            return ControlFlow::Break(KeyResult::Author);
                        }
                        Options::License { short: true, .. } if key == "l" => {
                            return ControlFlow::Break(KeyResult::License);
                        }
                        Options::License { long: true, .. } if is(key, "license", case) => {
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

    let mut index = 0;
    let mut help = None;
    let mut version = None;
    let mut position = Err(position);
    if let ControlFlow::Break(result) = descend(
        key,
        metas,
        &mut index,
        &mut help,
        &mut version,
        &mut position,
        swizzle,
        case,
        true,
        verb,
        option,
    ) {
        return result;
    }
    if let Ok(position) = position {
        return KeyResult::Index(position);
    }
    if option {
        if let Some(true) = help {
            if key == "h" || is(key, "help", case) {
                return KeyResult::Help;
            }
        }
        if let Some(true) = version {
            if key == "v" || is(key, "version", case) {
                return KeyResult::Version;
            }
        }
        if r#break && index > 0 {
            return KeyResult::Break;
        }
    }
    KeyResult::Unrecognized
}

impl<P: Parse> Parse for With<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(&self.meta, &self.set, self.prefix);
        match self.parse.initialize(context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(&self.meta, &self.set, self.prefix);
        match self.parse.parse(state, context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        let mut context = context.with(&self.meta, &self.set, self.prefix);
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
            (Some(Argument::Swizzle(argument)), _, _) => {
                return Err(Error::InvalidSwizzleOption(argument))
            }
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
            (Ok(value), _, _) => {
                if context.set.is_empty() || context.set.is_match(&argument) {
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
