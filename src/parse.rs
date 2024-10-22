use crate::{
    Options,
    case::Case,
    error::Error,
    help,
    meta::{Meta, Text},
    spell::Spell,
    stack::Stack,
    style::{Format, Style, Termion},
};
use core::{cmp::min, marker::PhantomData, num::NonZeroUsize};
use orn::*;
use regex::Regex;
use std::{
    any::{self, TypeId},
    collections::{HashMap, VecDeque},
    fmt::{self, Write},
    mem::take,
    ops::ControlFlow,
    str::FromStr,
    sync::Arc,
};

pub struct Context<'a> {
    arguments: &'a mut VecDeque<Argument>,
    environment: &'a mut HashMap<Text, Text>,
    path: &'a mut Vec<Key>,
    names: &'a mut Vec<Name>,
    indices: &'a mut Vec<usize>,
    root: Arc<Meta>,
    meta: Arc<Meta>,
    style: Arc<dyn Style>,
    case: Case,
    prefix: char,
}

#[derive(Clone)]
pub struct Parser<P> {
    pub(crate) parse: P,
    pub(crate) meta: Arc<Meta>,
    pub(crate) prefix: Option<char>,
    pub(crate) case: Option<Case>,
    pub(crate) style: Option<Arc<dyn Style>>,
}

#[derive(Clone)]
pub struct Node<P>(pub(crate) P);

#[derive(Default)]
pub struct Value<T>(pub(crate) PhantomData<T>);

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
pub struct TryMap<P, F>(pub(crate) P, pub(crate) F);
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
    Position(usize),
    Name(Name),
}

enum Argument {
    String(Text),
    Swizzle(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Name {
    value: Text,
    prefixes: (Option<char>, Option<char>),
    case: Option<Case>,
}

struct KeyFinder<'a> {
    argument: &'a Argument,
    arguments: &'a mut VecDeque<Argument>,
    metas: &'a [Meta],
    names: &'a mut Vec<Name>,
    indices: &'a mut Vec<usize>,
    count: &'a mut usize,
    prefix: char,
    case: Case,
    verb: bool,
    option: bool,
    position: usize,
    swizzle: bool,
    root: bool,
}

enum KeyResult {
    Help,
    Version,
    Author,
    License,
    Position(usize),
    Name(Name),
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

impl Name {
    pub fn len(&self, prefix: bool, case: bool) -> usize {
        self.chars(prefix, case).count()
    }

    pub fn chars(&self, prefix: bool, case: bool) -> impl Iterator<Item = char> + '_ {
        self.case.filter(|_| case).unwrap_or(Case::Same).convert(
            self.prefixes
                .0
                .filter(|_| prefix)
                .into_iter()
                .chain(self.prefixes.1.filter(|_| prefix))
                .chain(self.value.chars()),
        )
    }
}

impl Argument {
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        use or2::Or;
        match self {
            Argument::String(key) => Or::T0(key.chars()),
            Argument::Swizzle(key) => Or::T1([*key]),
        }
        .into_iter()
        .map(Or::into)
    }
}

impl Format for Name {
    fn width(&self) -> usize {
        self.len(true, true)
    }

    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for character in self.chars(true, true) {
            f.write_char(character)?;
        }
        Ok(())
    }
}

impl Format for Key {
    fn width(&self) -> usize {
        match self {
            Key::Position(position) if *position < 10 => 3,
            Key::Position(_) => 4,
            Key::Name(name) => name.width(),
        }
    }

    fn format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Position(position) => write!(f, "[{}]", position),
            Key::Name(name) => write!(f, "{}", name),
        }
    }
}

impl<T: Stack> Stack for Node<T> {
    type Clear = Node<T::Clear>;
    type Item = T::Item;
    type Pop = Node<T::Pop>;
    type Push<U> = Node<T::Push<U>>;

    const COUNT: usize = T::COUNT;

    fn push<U>(self, item: U) -> Self::Push<U> {
        Node(self.0.push(item))
    }

    fn pop(self) -> (Self::Item, Self::Pop) {
        let pair = self.0.pop();
        (pair.0, Node(pair.1))
    }

    fn clear(self) -> Self::Clear {
        Node(self.0.clear())
    }
}

impl<T: Stack> Stack for At<T> {
    type Clear = At<T::Clear>;
    type Item = T::Item;
    type Pop = At<T::Pop>;
    type Push<U> = At<T::Push<U>>;

    const COUNT: usize = T::COUNT;

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
            path: self.path,
            names: self.names,
            prefix: self.prefix,
            root: self.root.clone(),
            case: self.case,
            meta: self.meta.clone(),
            indices: self.indices,
            style: self.style.clone(),
        }
    }

    fn key(&mut self, position: &mut usize) -> Result<Option<Key>, Error> {
        let Some(argument) = self.arguments.pop_front() else {
            return Ok(None);
        };
        self.indices.clear();
        self.names.clear();
        let result = KeyFinder {
            argument: &argument,
            arguments: self.arguments,
            metas: self.meta.children(),
            names: self.names,
            indices: self.indices,
            count: &mut 0,
            prefix: self.prefix,
            case: self.case,
            verb: false,
            option: false,
            swizzle: false,
            position: *position,
            root: true,
        }
        .find();

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
            KeyResult::Name(name) => Ok(Some(Key::Name(name))),
            KeyResult::Position(index) => {
                self.arguments.push_front(argument);
                *position += 1;
                Ok(Some(Key::Position(index)))
            }
            KeyResult::Unrecognized => {
                let key = match argument {
                    Argument::String(key) => key,
                    Argument::Swizzle(key) => format!("{}{key}", self.prefix).into(),
                };
                let suggestions = Spell::new().suggest(
                    &key,
                    self.names
                        .iter()
                        .map(|name| name.chars(true, true).collect()),
                    min(key.len() / 3, 3),
                );
                Err(Error::UnrecognizedArgument(key, suggestions))
            }
        }
    }

    fn fill(self, error: Error) -> Error {
        match error {
            Error::Help(None) => Error::Help(Some(help::Help {
                root: self.root,
                meta: self.meta,
                style: self.style,
                path: take(self.path),
            })),
            Error::Version(None) => Error::Version(Some(help::Version(self.meta))),
            Error::License(None) => Error::License(Some(help::License(self.meta))),
            Error::Author(None) => Error::Author(Some(help::Author(self.meta))),
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
        for meta in self.meta.children() {
            state = descend(meta, state, &mut valid)?;
        }
        Ok(state)
    }

    fn missing_option(self) -> Error {
        Error::MissingOptionValue(self.type_name(), take(self.path))
    }

    fn missing_required(self) -> Error {
        let path = take(self.path);
        if let Meta::Option(_) = self.meta.as_ref() {
            Error::MissingRequiredOption(path)
        } else {
            Error::MissingRequiredValue(path, self.meta.require())
        }
    }

    fn duplicate_verb(self) -> Error {
        Error::DuplicateVerb(take(self.path))
    }

    fn duplicate_option(self) -> Error {
        Error::DuplicateOption(take(self.path))
    }

    fn invalid_option(self, value: Text) -> Error {
        Error::InvalidOptionValue(value, self.patterns(), take(self.path))
    }

    fn failed_parse(self, value: Text) -> Error {
        Error::FailedToParseOptionValue(value, self.type_name(), take(self.path))
    }

    fn type_name(&self) -> Option<Text> {
        let mut name = None;
        for meta in self.meta.children() {
            if let Meta::Type(value) = meta {
                name = Some(value);
            }
        }
        name.cloned()
    }

    fn with(
        &mut self,
        meta: Arc<Meta>,
        prefix: Option<char>,
        case: Option<Case>,
        style: Option<Arc<dyn Style>>,
    ) -> Context {
        let mut context = self.own();
        context.meta = meta;
        if let Some(prefix) = prefix {
            context.prefix = prefix;
        }
        if let Some(case) = case {
            context.case = case;
        }
        if let Some(style) = style {
            context.style = style;
        }
        context
    }
}

impl<P> Parser<P> {
    fn new(parse: P, meta: Meta) -> Self {
        Self {
            parse,
            meta: Arc::new(meta),
            prefix: None,
            case: None,
            style: None,
        }
    }

    pub const fn case(mut self, case: Case) -> Self {
        self.case = Some(case);
        self
    }

    pub const fn prefix(mut self, prefix: char) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn style<S: Style + 'static>(mut self, style: S) -> Self {
        self.style = Some(Arc::new(style));
        self
    }

    pub fn name(mut self, name: impl Into<Text>) -> Result<Self, Error> {
        self.with_meta(|meta| match meta {
            Meta::Option(metas) => {
                let name = name.into();
                if name
                    .chars()
                    .any(|letter| letter.is_whitespace() || !letter.is_ascii_alphanumeric())
                {
                    Err(Error::InvalidOptionName(name))
                } else {
                    metas.push(Meta::Name(name));
                    Ok(())
                }
            }
            Meta::Verb(metas) => {
                let name = name.into();
                if name
                    .chars()
                    .any(|letter| letter.is_whitespace() || !letter.is_ascii_alphanumeric())
                {
                    Err(Error::InvalidVerbName(name))
                } else {
                    metas.push(Meta::Name(name));
                    Ok(())
                }
            }
            Meta::Group(metas) => {
                let name = name.into();
                if !name.chars().all(char::is_whitespace) {
                    metas.push(Meta::Name(name));
                }
                Ok(())
            }
            meta => Err(Error::InvalidLeafMeta(Clone::clone(meta))),
        })?;
        Ok(self)
    }

    pub fn help(self, help: impl Into<Text>) -> Self {
        let help = help.into();
        if help.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Help(help))
        }
    }

    pub fn child<Q>(self, parser: Parser<Q>) -> Parser<P::Push<Parser<Q>>>
    where
        P: Stack,
    {
        self.meta(parser.meta.as_ref().clone(0))
            .map_parse(|parse| parse.push(parser))
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.with_metas(|metas| metas.push(meta));
        self
    }

    fn with_meta<T>(&mut self, with: impl FnOnce(&mut Meta) -> T) -> T {
        match Arc::get_mut(&mut self.meta) {
            Some(meta) => with(meta),
            None => {
                let mut meta = Clone::clone(self.meta.as_ref());
                let value = with(&mut meta);
                self.meta = Arc::new(meta);
                value
            }
        }
    }

    fn with_metas<T>(&mut self, with: impl FnOnce(&mut Vec<Meta>) -> T) -> Option<T> {
        self.with_meta(|meta| match meta {
            Meta::Verb(metas) | Meta::Group(metas) | Meta::Option(metas) => Some(with(metas)),
            _ => None,
        })
    }

    fn map_parse<Q>(self, map: impl FnOnce(P) -> Q) -> Parser<Q> {
        Parser {
            parse: map(self.parse),
            meta: self.meta,
            prefix: self.prefix,
            case: self.case,
            style: self.style,
        }
    }
}

impl<P: Parse> Parser<P> {
    pub fn parse(&self) -> Result<P::Value, Error> {
        self.parse_with(std::env::args().skip(1), std::env::vars())
    }

    pub fn parse_with<A: Into<Text>, K: Into<Text>, V: Into<Text>>(
        &self,
        arguments: impl IntoIterator<Item = A>,
        environment: impl IntoIterator<Item = (K, V)>,
    ) -> Result<P::Value, Error> {
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
            path: &mut Vec::new(),
            names: &mut Vec::new(),
            indices: &mut Vec::new(),
            prefix: self.prefix.unwrap_or('-'),
            case: self.case.unwrap_or(Case::Kebab { upper: false }),
            style: self.style.clone().unwrap_or_else(|| Arc::new(Termion)),
            root: self.meta.clone(),
            meta: self.meta.clone(),
        };
        let state = Parse::initialize(self, context.own())?;
        let state = Parse::parse(self, state, context.own())?;
        let value = Parse::finalize(self, state, context)?;
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

    pub fn map<T, F: Fn(P::Value) -> T>(self, map: F) -> Parser<Map<P, F>> {
        self.map_parse(|parse| Map(parse, map))
    }

    pub fn try_map<T, E: Into<Error>, F: Fn(P::Value) -> Result<T, E>>(
        self,
        map: F,
    ) -> Parser<TryMap<P, F>> {
        self.map_parse(|parse| TryMap(parse, map))
    }
}

impl Parser<Node<At>> {
    pub fn verb() -> Self {
        Parser::new(Node(At(())), Meta::Verb(Vec::new()))
    }
}

impl Parser<At> {
    pub fn group() -> Self {
        Parser::new(At(()), Meta::Group(Vec::new()))
    }
}

impl<T: FromStr + 'static> Parser<Value<T>> {
    pub fn option() -> Self {
        Parser::new(Value(PhantomData), Meta::Option(Vec::new()))
            .meta(Meta::Type(type_name::<T>().into()))
    }
}

impl<P: Parse + ?Sized> Parse for Box<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        P::finalize(self, state, context)
    }
}

impl<P: Parse + ?Sized> Parse for &P {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        P::finalize(self, state, context)
    }
}

impl<P: Parse + ?Sized> Parse for &mut P {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        P::initialize(self, context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        P::parse(self, state, context)
    }

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
        while let Some(key) = context.key(&mut position)? {
            context.path.push(key);
            outer = self.0.parse(outer, context.own())?;
            context.path.pop();
        }
        Ok(Some(self.0.finalize(outer, context.own())?))
    }

    fn finalize(&self, state: Self::State, _: Context) -> Result<Self::Value, Error> {
        Ok(state)
    }
}

impl<P: Parse> Parse for Parser<P> {
    type State = P::State;
    type Value = P::Value;

    fn initialize(&self, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(
            self.meta.clone(),
            self.prefix,
            self.case,
            self.style.clone(),
        );
        match self.parse.initialize(context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn parse(&self, state: Self::State, mut context: Context) -> Result<Self::State, Error> {
        let mut context = context.with(
            self.meta.clone(),
            self.prefix,
            self.case,
            self.style.clone(),
        );
        match self.parse.parse(state, context.own()) {
            Ok(state) => Ok(state),
            Err(error) => Err(context.fill(error)),
        }
    }

    fn finalize(&self, state: Self::State, mut context: Context) -> Result<Self::Value, Error> {
        let mut context = context.with(
            self.meta.clone(),
            self.prefix,
            self.case,
            self.style.clone(),
        );
        match self.parse.finalize(state, context.own()) {
            Ok(value) => Ok(value),
            Err(error) => Err(context.fill(error)),
        }
    }
}

impl<P: Parse, T, F: Fn(P::Value) -> T> Parse for Map<P, F> {
    type State = P::State;
    type Value = T;

    fn initialize(&self, context: Context) -> Result<Self::State, Error> {
        self.0.initialize(context)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        self.0.parse(state, context)
    }

    fn finalize(&self, state: Self::State, context: Context) -> Result<Self::Value, Error> {
        Ok(self.1(self.0.finalize(state, context)?))
    }
}

impl<P: Parse, T, E: Into<Error>, F: Fn(P::Value) -> Result<T, E>> Parse for TryMap<P, F> {
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
                        take(context.path),
                    )),
                },
                None => Ok(None),
            },
        }
    }
}

impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: FromStr + 'static> Parse for Value<T> {
    type State = Option<T>;
    type Value = Option<T>;

    fn initialize(&self, _: Context) -> Result<Self::State, Error> {
        Ok(None)
    }

    fn parse(&self, state: Self::State, context: Context) -> Result<Self::State, Error> {
        if state.is_some() {
            return Err(context.duplicate_option());
        }

        let argument = match context.arguments.pop_front() {
            Some(Argument::String(argument)) => argument,
            Some(Argument::Swizzle(argument)) => {
                return Err(Error::InvalidSwizzleOption(format!("{}", argument).into()));
            }
            None if TypeId::of::<bool>() == TypeId::of::<T>() => match "true".parse::<T>() {
                Ok(value) => return Ok(Some(value)),
                Err(_) => return Err(context.failed_parse("true".into())),
            },
            None => return Err(context.missing_option()),
        };
        match argument.parse::<T>() {
            Ok(value) => {
                if context.is_valid(&argument) {
                    Ok(Some(value))
                } else {
                    Err(context.invalid_option(argument))
                }
            }
            Err(_) if TypeId::of::<bool>() == TypeId::of::<T>() => match "true".parse::<T>() {
                Ok(value) => {
                    context.arguments.push_front(Argument::String(argument));
                    Ok(Some(value))
                }
                Err(_) => Err(context.failed_parse(argument)),
            },
            Err(_) => Err(context.failed_parse(argument)),
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

impl KeyFinder<'_> {
    fn find(mut self) -> KeyResult {
        let mut help = None;
        let mut version = None;
        if let ControlFlow::Break(result) = self
            .with(self.metas, false, false, true)
            .descend(&mut help, &mut version)
        {
            return result;
        }
        if self.option {
            if let Some(true) = help {
                if self.is_help(true, true) {
                    return KeyResult::Help;
                }
            }
            if let Some(true) = version {
                if self.is_version(true, true) {
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

    fn is_name(&mut self, name: Name, key: impl IntoIterator<Item = char>) -> Option<Name> {
        if name.chars(false, true).eq(key) {
            Some(name)
        } else {
            self.names.push(name);
            None
        }
    }

    fn is_short_option(
        &mut self,
        name: impl Into<Text>,
        key: impl IntoIterator<Item = char>,
    ) -> Option<Name> {
        let name = Name {
            value: name.into(),
            prefixes: (Some(self.prefix), None),
            case: None,
        };
        self.is_name(name, key)
    }

    fn is_long_option(
        &mut self,
        name: impl Into<Text>,
        key: impl IntoIterator<Item = char>,
    ) -> Option<Name> {
        let name = Name {
            value: name.into(),
            prefixes: (Some(self.prefix), Some(self.prefix)),
            case: Some(self.case),
        };
        self.is_name(name, key)
    }

    fn is_short_verb(
        &mut self,
        name: impl Into<Text>,
        key: impl IntoIterator<Item = char>,
    ) -> Option<Name> {
        let name = Name {
            value: name.into(),
            prefixes: (None, None),
            case: None,
        };
        self.is_name(name, key)
    }

    fn is_long_verb(
        &mut self,
        name: impl Into<Text>,
        key: impl IntoIterator<Item = char>,
    ) -> Option<Name> {
        let name = Name {
            value: name.into(),
            prefixes: (None, None),
            case: Some(self.case),
        };
        self.is_name(name, key)
    }

    fn is_options(
        &mut self,
        short: Option<impl Into<Text>>,
        long: Option<impl Into<Text>>,
    ) -> bool {
        if let Some(short) = short {
            if self.is_short_option(short, self.argument.chars()).is_some() {
                return true;
            }
        }
        if let Some(long) = long {
            if self.is_long_option(long, self.argument.chars()).is_some() {
                return true;
            }
        }
        false
    }

    fn is_help(&mut self, short: bool, long: bool) -> bool {
        self.is_options(Some("h").filter(|_| short), Some("help").filter(|_| long))
    }

    fn is_version(&mut self, short: bool, long: bool) -> bool {
        self.is_options(
            Some("v").filter(|_| short),
            Some("version").filter(|_| long),
        )
    }

    fn is_author(&mut self, short: bool, long: bool) -> bool {
        self.is_options(Some("a").filter(|_| short), Some("author").filter(|_| long))
    }

    fn is_license(&mut self, short: bool, long: bool) -> bool {
        self.is_options(
            Some("l").filter(|_| short),
            Some("license").filter(|_| long),
        )
    }

    fn with<'a>(
        &'a mut self,
        metas: &'a [Meta],
        option: bool,
        verb: bool,
        root: bool,
    ) -> KeyFinder<'a> {
        KeyFinder {
            argument: self.argument,
            arguments: self.arguments,
            names: self.names,
            indices: self.indices,
            prefix: self.prefix,
            case: self.case,
            position: self.position,
            swizzle: self.swizzle,
            count: self.count,
            verb,
            option,
            metas,
            root,
        }
    }

    fn swizzles(&self, index: usize) -> bool {
        self.swizzle
            || self.metas[index..]
                .iter()
                .any(|meta| matches!(meta, Meta::Swizzle))
    }

    fn descend(
        &mut self,
        help: &mut Option<bool>,
        version: &mut Option<bool>,
    ) -> ControlFlow<KeyResult> {
        use ControlFlow::*;

        let mut at = 0;
        for (i, meta) in self.metas.iter().enumerate() {
            match meta {
                Meta::Help(_)
                | Meta::Usage(_)
                | Meta::Note(_)
                | Meta::Repository(_)
                | Meta::Summary(_) => *help = help.or(Some(true)),
                Meta::Version(_) => *version = version.or(Some(true)),
                Meta::Position if !self.root => {
                    if *self.count == self.position {
                        return Break(KeyResult::Position(self.position));
                    } else {
                        *self.count += 1;
                    }
                }
                Meta::Name(name) if !self.root => {
                    match self.argument {
                        Argument::String(key) => {
                            let mut characters = key.chars();
                            match characters.next() {
                                Some(first) if self.option && first == self.prefix => {
                                    match characters.next() {
                                        Some(second) if second == self.prefix => {
                                            match characters.next() {
                                                // Long option.
                                                Some(third) => {
                                                    if let Some(name) = self.is_long_option(
                                                        name.clone(),
                                                        [third].into_iter().chain(characters),
                                                    ) {
                                                        return Break(KeyResult::Name(name));
                                                    }
                                                }
                                                // Break.
                                                // To produce `Break` from `--`, there must be at
                                                // least an option or verb with a name
                                                // defined so it is fine to make this check here.
                                                None => return Break(KeyResult::Break(false)),
                                            }
                                        }
                                        // Short option.
                                        Some(second) => {
                                            if let Some(name) =
                                                self.is_short_option(name.clone(), [second])
                                            {
                                                let mut single = true;
                                                for character in characters.rev() {
                                                    self.arguments
                                                        .push_front(Argument::Swizzle(character));
                                                    single = false;
                                                }
                                                if single || self.swizzles(i) {
                                                    return Break(KeyResult::Name(name));
                                                } else {
                                                    return Break(KeyResult::Error(
                                                        Error::InvalidSwizzleOption(
                                                            name.value.clone(),
                                                        ),
                                                    ));
                                                }
                                            }
                                        }
                                        None => return Break(KeyResult::Unrecognized),
                                    }
                                }
                                Some(first) if self.verb => match characters.next() {
                                    // Long verb.
                                    Some(second) => {
                                        if let Some(name) = self.is_long_verb(
                                            name.clone(),
                                            [first, second].into_iter().chain(characters),
                                        ) {
                                            return Break(KeyResult::Name(name));
                                        }
                                    }
                                    // Short verb.
                                    None => {
                                        if let Some(name) =
                                            self.is_short_verb(name.clone(), [first])
                                        {
                                            return Break(KeyResult::Name(name));
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                        Argument::Swizzle(key) => {
                            if let Some(name) = self.is_short_option(name.clone(), [*key]) {
                                if self.swizzles(i) {
                                    return Break(KeyResult::Name(name));
                                } else {
                                    return Break(KeyResult::Error(Error::InvalidSwizzleOption(
                                        name.value.clone(),
                                    )));
                                }
                            }
                        }
                    }
                }
                Meta::Swizzle => self.swizzle = true,
                Meta::Group(metas) => {
                    self.indices.push(at);
                    self.with(metas, self.option, self.verb, self.root)
                        .descend(help, version)?;
                    self.indices.pop();
                    at += 1;
                }
                Meta::Verb(metas) if self.root => {
                    if self.verb {
                        self.indices.push(at);
                        self.with(metas, false, true, false)
                            .descend(help, version)?;
                        self.indices.pop();
                    }
                    at += 1;
                    *help = help.or(Some(true));
                }
                Meta::Option(metas) if self.root => {
                    if self.option {
                        self.indices.push(at);
                        self.with(metas, true, false, false)
                            .descend(help, version)?;
                        self.indices.pop();
                    }
                    at += 1;
                    *help = help.or(Some(true));
                }
                Meta::Options(options) if self.root => {
                    match options {
                        Options::Help { short, long } if self.is_help(*short, *long) => {
                            return Break(KeyResult::Help);
                        }
                        Options::Version { short, long } if self.is_version(*short, *long) => {
                            return Break(KeyResult::Version);
                        }
                        Options::Author { short, long } if self.is_author(*short, *long) => {
                            return Break(KeyResult::Author);
                        }
                        Options::License { short, long } if self.is_license(*short, *long) => {
                            return Break(KeyResult::License);
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

fn trim_pattern(pattern: &Regex) -> &str {
    pattern
        .as_str()
        .trim_start_matches('^')
        .trim_end_matches('$')
}

fn type_name<T: 'static>() -> &'static str {
    macro_rules! is {
        ($left: expr $(, $rights: ident)+) => {
            $($left == TypeId::of::<$rights>() || $left == TypeId::of::<Option<$rights>>() ||)+ false
        };
    }

    let identifier = TypeId::of::<T>();
    if is!(identifier, bool) {
        "boolean"
    } else if is!(identifier, u8, u16, u32, u64, u128, usize) {
        "natural number"
    } else if is!(identifier, i8, i16, i32, i64, i128, isize) {
        "integer number"
    } else if is!(identifier, f32, f64) {
        "rational number"
    } else {
        let mut name = any::type_name::<T>();
        if let Some(split) = name.split('<').next() {
            name = split;
        }
        if let Some(split) = name.split(':').last() {
            name = split;
        }
        name
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
                let Some(index) = _context.indices.pop() else { return Err(Error::MissingIndex); };
                match index {
                    $($index => { _state.$index = self.0.$index.parse(_state.$index, _context.own())?; })*
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
at!(
    Or9, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8
);
at!(
    Or10, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9
);
at!(
    Or11, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10
);
at!(
    Or12, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11
);
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
