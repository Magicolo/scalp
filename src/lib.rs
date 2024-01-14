pub mod error;
pub mod scope;
pub mod stack;

pub use crate::{
    error::{Error, Ok},
    scope::Scope,
};

use crate::stack::{Count, Pop, Push};
use scalp_core::case::Case;
use std::{
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap, VecDeque},
    default,
    fmt::{self, Write},
    marker::PhantomData,
    mem::replace,
    str::FromStr,
};

/*
    TODO:
    - If verb doesn't have sub options, allow options to be placed before or after the verb.
    - Ensure that variables don't obscure the context variable.
    - Support for streamed arguments via stdin, file system, http.
    - Support for help with #[help(text)].
        - Maybe the #[help] attribute generates a doc string?
        - Allow on fields and structs.
    - Support for a value with --help
        - Allows to provide a help context when help becomes very large (ex: --help branch)
    - Support aliases with #[alias(names...)].
        - Maybe the #[alias] attribute generates a doc string?
    - Support default values with #[default(value?)].
        - Maybe the #[default] attribute generates a doc string?
        - #[default] uses 'Default::default'.
        - #[default(value)] uses 'TryFrom::try_from(value)'.
    - Support environment variables with #[environment(variables...)].
        - Maybe the #[environment] attribute generates a doc string?
    - Support for #[omit(help)]
    - Support for #[version] (uses the cargo version) or #[version(version)] (explicit version).
        - Only add the version option if the version attribute is supplied.
    - Autocomplete?
    - Add support for combined flags using the short names when possible.
        - Short names must be of length 1.
        - ex: ls -l -a -r -t => ls -lart
    - Can I unify 'Builder' and 'Parser'?
    - Support for json values.
    - What if an option has an child that is an option/verb/Group?
    - Different kinds of 'help' such as 'usage', 'summary', 'detail'; that will be displayed in different contexts.
        - The motivation comes from differentiating the 'summary' help and the 'detail' help.
        - Summaries will be shown from the parent node.
        - Details will be shown only for the specific node.
        - Maybe show the help only at the current node level and require a parameter to show from the parent.
*/

#[derive(Debug)]
pub enum Meta {
    Name(Cow<'static, str>),
    Key(Cow<'static, str>),
    Version(Cow<'static, str>),
    Help(Cow<'static, str>),
    Type(Cow<'static, str>),
    Hide,
    Root(Vec<Meta>),
    Option(Vec<Meta>),
    Verb(Vec<Meta>),
    Group(Vec<Meta>),
}

pub struct State<'a> {
    arguments: &'a mut VecDeque<Cow<'static, str>>,
    environment: &'a HashMap<Cow<'static, str>, Cow<'static, str>>,
    short: &'a str,
    long: &'a str,
    index: usize,
}

pub struct Builder<S, P> {
    case: Case,
    short: Cow<'static, str>,
    long: Cow<'static, str>,
    buffer: String,
    count: usize,
    parse: P,
    scope: S,
}

pub struct Parser<P> {
    short: Cow<'static, str>,
    long: Cow<'static, str>,
    parse: P,
}

pub struct Map<P, F>(P, F);

pub struct Node<P> {
    indices: HashMap<Cow<'static, str>, usize>,
    meta: Meta,
    parse: P,
}

pub struct Value<T>(PhantomData<T>);
pub struct Many<P, I>(P, Option<usize>, PhantomData<I>);
pub struct Require<P>(P);
pub struct Default<P, F>(P, F);
pub struct Environment<P>(P, Cow<'static, str>);
pub struct At<P>(P, usize);

pub trait Parse {
    type State;
    type Value;
    fn initialize(&self, state: &State) -> Result<Self::State, Error>;
    fn parse(&self, states: (Self::State, &mut State)) -> Result<Self::State, Error>;
    fn finalize(&self, states: (Self::State, &State)) -> Result<Self::Value, Error>;
}

impl<T, P: Push<T>> Push<T> for At<P> {
    type Output = At<P::Output>;

    fn push(self, item: T) -> Self::Output {
        At(self.0.push(item), self.1)
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

impl Meta {
    pub fn clone(&self, depth: usize) -> Self {
        match self {
            Meta::Name(value) => Meta::Name(value.clone()),
            Meta::Key(value) => Meta::Key(value.clone()),
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Type(value) => Meta::Type(value.clone()),
            Meta::Hide => Meta::Hide,
            Meta::Root(metas) if depth > 0 => {
                Meta::Root(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Root(_) => Meta::Root(Vec::new()),
            Meta::Option(metas) if depth > 0 => {
                Meta::Option(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Option(_) => Meta::Option(Vec::new()),
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
}

impl Parse for () {
    type State = ();
    type Value = ();

    fn initialize(&self, _: &State) -> Result<Self::State, Error> {
        Ok(())
    }

    fn parse(&self, _: (Self::State, &mut State)) -> Result<Self::State, Error> {
        Ok(())
    }

    fn finalize(&self, _: (Self::State, &State)) -> Result<Self::Value, Error> {
        Ok(())
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

// TODO: A required option in a verb will return an error even if the verb was not specified.
macro_rules! at {
    ($($name: ident, $index: tt),*) => {
        impl<$($name: Parse,)*> Parse for At<($($name,)*)> {
            type State = ($($name::State,)*);
            type Value = ($($name::Value,)*);

            fn initialize(&self, _state: &State) -> Result<Self::State, Error> {
                Ok(($(self.0.$index.initialize(_state)?,)*))
            }

            fn parse(&self, mut _states: (Self::State, &mut State)) -> Result<Self::State, Error> {
                match _states.1.index().checked_sub(self.1) {
                    $(Some($index) => _states.0.$index = self.0.$index.parse((_states.0.$index, _states.1))?,)*
                    _ => {},
                };
                #[allow(unreachable_code)]
                Ok(_states.0)
            }

            fn finalize(&self, _states: (Self::State, &State)) -> Result<Self::Value, Error> {
                Ok(($(self.0.$index.finalize((_states.0.$index, _states.1))?,)*))
            }
        }
    };
}

// TODO: Implement up to 64.
at!();
at!(P0, 0);
at!(P0, 0, P1, 1);
at!(P0, 0, P1, 1, P2, 2);
at!(P0, 0, P1, 1, P2, 2, P3, 3);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6);
at!(P0, 0, P1, 1, P2, 2, P3, 3, P4, 4, P5, 5, P6, 6, P7, 7);

impl default::Default for Builder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

const HELP: usize = usize::MAX;
const VERSION: usize = usize::MAX - 1;
const BREAK: usize = usize::MAX - 2;

impl Builder<(), ()> {
    pub const fn new() -> Self {
        Self {
            case: Case::Kebab,
            short: Cow::Borrowed("-"),
            long: Cow::Borrowed("--"),
            buffer: String::new(),
            count: 0,
            scope: (),
            parse: (),
        }
    }

    pub fn case(mut self, case: Case) -> Self {
        self.case = case;
        self
    }

    pub fn short(mut self, prefix: impl Into<Cow<'static, str>>) -> Self {
        self.short = prefix.into();
        self
    }

    pub fn long(mut self, prefix: impl Into<Cow<'static, str>>) -> Self {
        self.long = prefix.into();
        self
    }

    pub fn root<
        Q,
        B: FnOnce(Builder<scope::Root, At<()>>) -> Result<Builder<scope::Root, Q>, Error>,
    >(
        self,
        build: B,
    ) -> Result<Builder<(), Node<Q>>, Error> {
        let (mut root, mut builder) =
            build(self.map_both(|_| scope::Root::new(), |_| At((), 0)))?.swap_scope(());
        let mut indices = HashMap::new();
        let mut index = 0;
        builder.descend_all(&mut root, &mut index, &mut indices, true)?;
        Ok(builder.map_parse(|parse| Node {
            parse,
            indices,
            meta: root.into(),
        }))
    }
}

impl<S, P> Builder<S, P> {
    fn descend_all(
        &mut self,
        metas: &mut Vec<Meta>,
        index: &mut usize,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        root: bool,
    ) -> Result<(), Error> {
        // TODO: Insert help and version as verb if root.
        for i in 0..metas.len() {
            match metas.get_mut(i) {
                Some(Meta::Version(_)) if root => {
                    let mut version =
                        vec![Meta::Help(Cow::Borrowed("Displays version information."))];
                    version.extend(self.insert_option("version", indices, VERSION)?);
                    version.extend(self.insert_option("v", indices, VERSION)?);
                    metas.push(Meta::Option(version));
                }
                Some(Meta::Option(metas)) => {
                    self.descend_option(metas, indices, *index)?;
                    *index += 1;
                }
                Some(Meta::Verb(metas)) => {
                    self.descend_verb(metas, indices, *index)?;
                    *index += 1;
                }
                Some(Meta::Group(metas)) => self.descend_all(metas, index, indices, false)?,
                None => break,
                _ => {}
            }
        }

        if root {
            let mut help = vec![Meta::Help(Cow::Borrowed("Displays this help message."))];
            help.extend(self.insert_option("help", indices, HELP)?);
            help.extend(self.insert_option("h", indices, HELP)?);
            metas.push(Meta::Option(help));
            Self::insert(self.long.clone(), indices, BREAK)?;
        }

        Ok(())
    }

    fn descend_verb(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<(), Error> {
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(name)) => metas.extend(self.insert_verb(name, indices, index)?),
                None => break,
                _ => {}
            };
        }
        Ok(())
    }

    fn descend_option(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<(), Error> {
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(name)) => metas.extend(self.insert_option(name, indices, index)?),
                None => break,
                _ => {}
            };
        }
        Ok(())
    }

    fn insert_option(
        &mut self,
        name: &str,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<Option<Meta>, Error> {
        let name = name.trim();
        match name.len() {
            0 => return Ok(None),
            1 => {
                self.buffer.clear();
                self.buffer.push_str(&self.short);
                self.buffer.push_str(name);
            }
            2.. => {
                self.buffer.clear();
                self.buffer.push_str(&self.long);
                self.case.convert_in(name, &mut self.buffer);
            }
        }
        Ok(Some(Self::insert(
            Cow::Owned(self.buffer.clone()),
            indices,
            index,
        )?))
    }

    fn insert_verb(
        &mut self,
        name: &str,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<Option<Meta>, Error> {
        let name = name.trim();
        match name.len() {
            0 => return Ok(None),
            1.. => {
                self.buffer.clear();
                self.buffer.push_str(name);
            }
        }
        Ok(Some(Self::insert(
            Cow::Owned(self.buffer.clone()),
            indices,
            index,
        )?))
    }

    fn insert(
        name: Cow<'static, str>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<Meta, Error> {
        match indices.entry(name.clone()) {
            Entry::Occupied(_) => Err(Error::DuplicateName { name }),
            Entry::Vacant(entry) => {
                entry.insert(index);
                Ok(Meta::Key(name))
            }
        }
    }
}

fn help_in<W: Write>(meta: &Meta, writer: &mut W) -> Result<(), fmt::Error> {
    match meta {
        Meta::Hide => return Ok(()),
        Meta::Root(metas) => help_root_in(metas, writer)?,
        Meta::Option(metas) => help_option_in(metas, writer)?,
        Meta::Verb(metas) => help_verb_in(metas, writer)?,
        Meta::Group(metas) => help_group_in(metas, writer)?,
        _ => {}
    }
    writer.write_char('\n')?;
    Ok(())
}

fn help_root_in<W: Write>(metas: &[Meta], writer: &mut W) -> Result<(), fmt::Error> {
    for meta in metas {
        match meta {
            Meta::Hide => break,
            _ => {}
        }
    }
    Ok(())
}

fn help_option_in<W: Write>(metas: &[Meta], writer: &mut W) -> Result<(), fmt::Error> {
    for meta in metas {
        match meta {
            Meta::Hide => break,
            _ => {}
        }
    }
    Ok(())
}

fn help_verb_in<W: Write>(metas: &[Meta], writer: &mut W) -> Result<(), fmt::Error> {
    for meta in metas {
        match meta {
            Meta::Hide => break,
            _ => {}
        }
    }
    Ok(())
}

fn help_group_in<W: Write>(metas: &[Meta], writer: &mut W) -> Result<(), fmt::Error> {
    for meta in metas {
        match meta {
            Meta::Hide => break,
            _ => {}
        }
    }
    Ok(())
}

fn help(meta: &Meta) -> Option<String> {
    let mut buffer = String::new();
    help_in(meta, &mut buffer).ok()?;
    Some(buffer)
}

fn version(meta: &Meta, depth: usize) -> Option<&Cow<'static, str>> {
    match meta {
        Meta::Version(version) => Some(version),
        Meta::Root(metas) | Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas)
            if depth > 0 =>
        {
            metas.iter().find_map(|meta| version(meta, depth - 1))
        }
        _ => None,
    }
}

impl<P: Parse> Builder<(), P> {
    pub fn build(self) -> Parser<P> {
        Parser {
            short: self.short,
            long: self.long,
            parse: self.parse,
        }
    }
}

impl<S, P> Builder<S, P> {
    pub fn map<T, F: Fn(P::Value) -> T>(
        self,
        map: F,
    ) -> Builder<S, Map<P, impl Fn(P::Value) -> Result<T, Error>>>
    where
        P: Parse,
    {
        self.try_map(move |value| Ok(map(value)))
    }

    pub fn map_some<T, U, F: Fn(T) -> U>(
        self,
        map: F,
    ) -> Builder<S, Map<P, impl Fn(Option<T>) -> Result<U, Error>>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.map_option(|| Error::MissingValue, map)
    }

    pub fn map_option<T, E: Into<Error>, U, F: Fn(T) -> U, G: Fn() -> E>(
        self,
        none: G,
        some: F,
    ) -> Builder<S, Map<P, impl Fn(Option<T>) -> Result<U, E>>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.try_map(move |value| match value {
            Some(value) => Ok(some(value)),
            None => Err(none()),
        })
    }

    pub fn try_map<T, E: Into<Error>, F: Fn(P::Value) -> Result<T, E>>(
        self,
        map: F,
    ) -> Builder<S, Map<P, F>>
    where
        P: Parse,
    {
        self.map_parse(|parse| Map(parse, map))
    }

    fn map_parse<Q>(self, map: impl FnOnce(P) -> Q) -> Builder<S, Q> {
        self.map_both(|scope| scope, map)
    }

    fn map_both<T, Q>(
        self,
        scope: impl FnOnce(S) -> T,
        parse: impl FnOnce(P) -> Q,
    ) -> Builder<T, Q> {
        Builder {
            case: self.case,
            short: self.short,
            long: self.long,
            buffer: self.buffer,
            count: self.count,
            scope: scope(self.scope),
            parse: parse(self.parse),
        }
    }

    fn swap_scope<T>(self, scope: T) -> (S, Builder<T, P>) {
        (
            self.scope,
            Builder {
                case: self.case,
                short: self.short,
                long: self.long,
                buffer: self.buffer,
                count: self.count,
                scope,
                parse: self.parse,
            },
        )
    }

    fn swap_both<T, Q>(self, scope: T, parse: Q) -> (S, P, Builder<T, Q>) {
        (
            self.scope,
            self.parse,
            Builder {
                case: self.case,
                short: self.short,
                long: self.long,
                buffer: self.buffer,
                count: self.count,
                scope,
                parse,
            },
        )
    }
}

impl<S: Scope, P> Builder<S, P> {
    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Name(name.into()))
    }

    pub fn help(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Help(name.into()))
    }

    pub fn version(self, version: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Version(version.into()))
    }

    pub fn hide(self) -> Self {
        self.meta(Meta::Hide)
    }

    pub fn group<Q>(
        self,
        build: impl FnOnce(Builder<scope::Group, At<()>>) -> Result<Builder<scope::Group, Q>, Error>,
    ) -> Result<Builder<S, P::Output>, Error>
    where
        P: Push<Q>,
    {
        let count = self.count;
        let (scope, old, group) = self.swap_both(scope::Group::new(), At((), count));
        let (scope, mut builder) = build(group)?
            .map_parse(|new| old.push(new))
            .swap_scope(scope);
        builder.scope.push(scope.into());
        Ok(builder)
    }

    pub fn verb<Q>(
        mut self,
        build: impl FnOnce(Builder<scope::Verb, At<()>>) -> Result<Builder<scope::Verb, Q>, Error>,
    ) -> Result<Builder<S, P::Output>, Error>
    where
        P: Push<Node<Q>>,
    {
        let count = replace(&mut self.count, 0);
        let (scope, old, builder) = self.swap_both(scope::Verb::new(), At((), 0));
        let (mut verb, mut builder) = build(builder)?.swap_scope(scope);
        let mut indices = HashMap::new();
        let mut index = 0;
        builder.descend_all(&mut verb, &mut index, &mut indices, true)?;
        let meta = Meta::from(verb);
        builder.scope.push(meta.clone(1));
        builder.count = count + 1;
        Ok(builder.map_parse(|new| {
            old.push(Node {
                parse: new,
                indices,
                meta,
            })
        }))
    }

    pub fn option<T: FromStr, Q>(
        self,
        build: impl FnOnce(Builder<scope::Option, Value<T>>) -> Builder<scope::Option, Q>,
    ) -> Builder<S, P::Output>
    where
        P: Count + Push<Q>,
    {
        let (scope, old, option) = self.swap_both(scope::Option::new(), Value(PhantomData));
        let (option, mut builder) = build(option)
            .map_parse(|new| old.push(new))
            .swap_scope(scope);
        builder.scope.push(option.into());
        builder.count += 1;
        builder
    }

    fn meta(mut self, meta: Meta) -> Self {
        self.scope.push(meta);
        self
    }
}

impl<P> Builder<scope::Option, P> {
    pub fn default<T, F: Fn() -> T>(self, default: F) -> Builder<scope::Option, Default<P, F>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.map_parse(|parse| Default(parse, default))
    }

    pub fn environment<T: FromStr>(
        self,
        variable: impl Into<Cow<'static, str>>,
    ) -> Builder<scope::Option, Environment<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.map_parse(|parse| Environment(parse, variable.into()))
    }

    pub fn require(self) -> Builder<scope::Option, Require<P>> {
        self.map_parse(Require)
    }

    pub fn many<T, I: default::Default + Extend<T>>(
        self,
        per: Option<usize>,
    ) -> Builder<scope::Option, Many<P, I>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.map_parse(|parse| Many(parse, per, PhantomData))
    }
}

impl<P: Parse, T, E: Into<Error>, F: Fn(P::Value) -> Result<T, E>> Parse for Map<P, F> {
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

impl<T, F: Fn() -> T, P: Parse<Value = Option<T>>> Parse for Default<P, F> {
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
            None => Ok(self.1()),
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
