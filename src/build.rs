use regex::RegexSet;

use crate::{
    case::Case,
    error::Error,
    meta::{Meta, Name, Options},
    parse::{
        Any, At, Default, Environment, Indices, Many, Map, Node, Parse, Parser, Require, Value,
        With,
    },
    scope::{self, Scope},
    stack::Stack,
    style, AUTHOR, BREAK, HELP, LICENSE, MAXIMUM, SHIFT, VERSION,
};
use core::{any::TypeId, default, fmt, marker::PhantomData, num::NonZeroUsize, str::FromStr};
use std::{any, borrow::Cow, collections::hash_map::Entry, convert::Infallible, mem::replace};

pub struct Builder<S, P = At<()>> {
    case: Case,
    tag: Cow<'static, str>,
    short: Cow<'static, str>,
    long: Cow<'static, str>,
    buffer: String,
    parse: Result<P, Error>,
    scope: S,
    style: Box<dyn style::Style>,
    position: usize,
}

pub struct Unit;

pub trait Flag {}

impl Flag for Option<bool> {}
impl Flag for bool {}

impl FromStr for Unit {
    type Err = Infallible;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Ok(Unit)
    }
}

impl default::Default for Builder<scope::Root> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, P> Builder<S, P> {
    pub fn pipe<Q>(self, pipe: impl FnOnce(Self) -> Builder<S, Q>) -> Builder<S, Q> {
        pipe(self)
    }

    pub fn map<T, F: Fn(P::Value) -> T>(self, map: F) -> Builder<S, impl Parse<Value = T>>
    where
        P: Parse,
    {
        self.try_map(move |value| Ok(map(value)))
    }

    pub fn try_map<T, F: Fn(P::Value) -> Result<T, Error>>(self, map: F) -> Builder<S, Map<P, F>>
    where
        P: Parse,
    {
        self.map_parse(|parse| Map(parse, map))
    }

    pub fn filter(
        self,
        filter: impl Fn(&P::Value) -> bool,
    ) -> Builder<S, impl Parse<Value = Option<P::Value>>>
    where
        P: Parse,
    {
        self.map(move |value| if filter(&value) { Some(value) } else { None })
    }

    pub fn any<T>(self) -> Builder<S, impl Parse<Value = Option<T>>>
    where
        P: Parse,
        P::Value: Any<T>,
    {
        self.map(Any::any)
    }

    pub fn or<T>(self, error: impl Into<Error>) -> Builder<S, impl Parse<Value = T>>
    where
        P: Parse<Value = Option<T>>,
    {
        let error = error.into();
        self.try_map(move |value| value.ok_or(error.clone()))
    }

    pub fn boxed(self) -> Builder<S, Box<dyn Parse<Value = P::Value, State = P::State>>>
    where
        P: Parse + 'static,
    {
        self.map_parse(|parse| Box::new(parse) as _)
    }

    fn map_parse<Q>(self, map: impl FnOnce(P) -> Q) -> Builder<S, Q> {
        self.map_both(|scope| scope, map)
    }

    fn try_map_parse<Q>(self, map: impl FnOnce(P) -> Result<Q, Error>) -> Builder<S, Q> {
        self.try_map_both(|scope| scope, map)
    }

    fn map_both<T, Q>(
        self,
        scope: impl FnOnce(S) -> T,
        parse: impl FnOnce(P) -> Q,
    ) -> Builder<T, Q> {
        self.try_map_both(scope, |old| Ok(parse(old)))
    }

    fn try_map_both<T, Q>(
        self,
        scope: impl FnOnce(S) -> T,
        parse: impl FnOnce(P) -> Result<Q, Error>,
    ) -> Builder<T, Q> {
        Builder {
            case: self.case,
            tag: self.tag,
            short: self.short,
            long: self.long,
            buffer: self.buffer,
            style: self.style,
            scope: scope(self.scope),
            parse: self.parse.and_then(parse),
            position: self.position,
        }
    }

    fn swap_scope<T>(self, scope: T) -> (S, Builder<T, P>) {
        (
            self.scope,
            Builder {
                case: self.case,
                tag: self.tag,
                short: self.short,
                long: self.long,
                buffer: self.buffer,
                style: self.style,
                scope,
                parse: self.parse,
                position: self.position,
            },
        )
    }

    fn swap_both<T, Q>(self, scope: T, parse: Q) -> (S, Result<P, Error>, Builder<T, Q>) {
        (
            self.scope,
            self.parse,
            Builder {
                case: self.case,
                tag: self.tag,
                short: self.short,
                long: self.long,
                buffer: self.buffer,
                style: self.style,
                scope,
                parse: Ok(parse),
                position: self.position,
            },
        )
    }

    fn descend(&mut self, meta: &mut Meta) -> Result<(Indices, RegexSet), Error> {
        let mut indices = Indices::default();
        let mut patterns = Vec::new();
        let (version, help, metas) = match meta {
            Meta::Root(metas) | Meta::Verb(metas) | Meta::Group(metas) => {
                let tuple = self.descend_node(metas, 0, 0, &mut indices, &mut patterns)?;
                (tuple.0, tuple.1, metas)
            }
            Meta::Option(metas) => {
                self.descend_node(metas, 0, 0, &mut indices, &mut patterns)?;
                (None, None, metas)
            }
            _ => return Ok((indices, RegexSet::new(patterns)?)),
        };
        if let Some(true) = version {
            metas.extend(self.insert_version(&mut indices, true, true)?);
        }
        if let Some(true) = help {
            metas.extend(self.insert_help(&mut indices, true, true)?);
        }
        if version.is_some() || help.is_some() {
            Self::insert_key(self.long.clone(), &mut indices, BREAK)?;
        }
        Ok((indices, RegexSet::new(patterns)?))
    }

    fn descend_node(
        &mut self,
        metas: &mut [Meta],
        mask: usize,
        shift: u32,
        indices: &mut Indices,
        patterns: &mut Vec<String>,
    ) -> Result<(Option<bool>, Option<bool>), Error> {
        let mut index = 0;
        let mut version = None;
        let mut help = None;
        let mut hide = 0;
        for i in 0..metas.len() {
            let value = (index << shift) | mask;
            let Some(meta) = metas.get_mut(i) else {
                break;
            };
            match meta {
                Meta::Version(_) if hide == 0 => version = version.or(Some(true)),
                Meta::Valid(value) => patterns.push(format!("^{value}$")),
                Meta::Help(_) | Meta::Usage(_) | Meta::Note(_) if hide == 0 => {
                    help = help.or(Some(true))
                }
                Meta::Hide => hide += 1,
                Meta::Show => hide = usize::saturating_sub(hide, 1),
                Meta::Option(metas) => {
                    self.descend_option(metas, indices, value)?;
                    index += 1;
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Meta::Verb(metas) => {
                    self.descend_verb(metas, indices, value)?;
                    index += 1;
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Meta::Group(_) if shift > MAXIMUM => return Err(Error::GroupNestingLimitOverflow),
                Meta::Group(metas) => {
                    let tuple =
                        self.descend_node(metas, value, shift + SHIFT, indices, patterns)?;
                    version = merge(version, tuple.0, |left, right| left && right);
                    help = merge(help, tuple.1, |left, right| left && right);
                    index += 1;
                }
                Meta::Options(options) => {
                    let option = match *options {
                        Options::Help { short, long } => self.insert_help(indices, short, long)?,
                        Options::Version { short, long } => {
                            self.insert_version(indices, short, long)?
                        }
                        Options::License { short, long } => {
                            self.insert_license(indices, short, long)?
                        }
                        Options::Author { short, long } => {
                            self.insert_author(indices, short, long)?
                        }
                    };
                    if let Some(option) = option {
                        *meta = option;
                    }
                    help = Some(false);
                    version = Some(false);
                }
                _ => {}
            }
        }
        Ok((version, help))
    }

    fn descend_verb(
        &mut self,
        metas: &[Meta],
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        let mut has = false;
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(_, value)) => {
                    Self::insert_key(value.clone(), indices, index)?;
                    has = true;
                }
                None => break,
                _ => {}
            };
        }
        if has {
            Ok(())
        } else {
            Err(Error::MissingVerbName)
        }
    }

    fn descend_option(
        &mut self,
        metas: &[Meta],
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        let mut has = false;
        let mut shorts = Vec::new();
        let mut swizzle = false;
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(name, value)) => {
                    Self::insert_key(value.clone(), indices, index)?;
                    has = true;
                    if let Name::Short = name {
                        shorts.extend(value.chars().nth(self.short.len()));
                    }
                }
                Some(Meta::Swizzle) => swizzle = true,
                Some(Meta::Position(_)) => {
                    indices.positions.push(index);
                    has = true;
                }
                None => break,
                _ => {}
            };
        }
        if swizzle {
            if shorts.is_empty() {
                return Err(Error::MissingShortOptionNameForSwizzling);
            } else {
                indices.swizzles.extend(shorts);
            }
        }
        if has {
            Ok(())
        } else {
            Err(Error::MissingOptionNameOrPosition)
        }
    }

    fn insert_version(
        &mut self,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<Option<Meta>, Error> {
        self.insert_option(
            indices,
            if short { Some("v") } else { None },
            if long { Some("version") } else { None },
            "Displays version information.",
            VERSION,
        )
    }

    fn insert_license(
        &mut self,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<Option<Meta>, Error> {
        self.insert_option(
            indices,
            if short { Some("l") } else { None },
            if long { Some("license") } else { None },
            "Displays license information.",
            LICENSE,
        )
    }

    fn insert_author(
        &mut self,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<Option<Meta>, Error> {
        self.insert_option(
            indices,
            if short { Some("a") } else { None },
            if long { Some("author") } else { None },
            "Displays author information.",
            AUTHOR,
        )
    }

    fn insert_help(
        &mut self,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<Option<Meta>, Error> {
        self.insert_option(
            indices,
            if short { Some("h") } else { None },
            if long { Some("help") } else { None },
            "Displays this help message.",
            HELP,
        )
    }

    fn insert_option(
        &mut self,
        indices: &mut Indices,
        short: Option<&'static str>,
        long: Option<&'static str>,
        help: &'static str,
        index: usize,
    ) -> Result<Option<Meta>, Error> {
        let mut option = vec![Meta::Help(Cow::Borrowed(help))];
        if let Some(short) = short {
            let (name, value) = self.option_name(short)?;
            if Self::insert_key(value.clone(), indices, index).is_ok() {
                option.push(Meta::Name(name, value));
            }
        }
        if let Some(long) = long {
            let (name, value) = self.option_name(long)?;
            if Self::insert_key(value.clone(), indices, index).is_ok() {
                option.push(Meta::Name(name, value));
            }
        }
        if option.len() > 1 {
            Ok(Some(Meta::Option(option)))
        } else {
            Ok(None)
        }
    }

    fn insert_key(
        key: Cow<'static, str>,
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        match indices.indices.entry(key) {
            Entry::Occupied(entry) => {
                let _ = 1;
                Err(Error::DuplicateName(entry.key().to_string()))
            }
            Entry::Vacant(entry) => {
                entry.insert(index);
                Ok(())
            }
        }
    }

    fn extend_name(&mut self, name: &str, prefix: bool) -> Option<Name> {
        self.buffer.clear();
        if name.len() == 1 {
            if prefix {
                self.buffer.push_str(&self.short);
            }
            self.extend_letters(name.chars())
        } else {
            if prefix {
                self.buffer.push_str(&self.long);
            }
            self.extend_letters(self.case.convert(name))
        }
    }

    fn extend_letters(&mut self, letters: impl IntoIterator<Item = char>) -> Option<Name> {
        let start = self.buffer.len();
        for letter in letters {
            if letter.is_whitespace() || !letter.is_ascii() {
                return None;
            } else {
                self.buffer.push(letter);
            }
        }
        match self.buffer.len() - start {
            0 => None,
            1 => Some(Name::Short),
            _ => Some(Name::Long),
        }
    }

    fn option_name(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(Name, Cow<'static, str>), Error> {
        let mut outer = name.into();
        let Some(name) = self.extend_name(&outer, true) else {
            return Err(Error::InvalidOptionName(outer));
        };
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        Ok((name, outer))
    }

    fn verb_name(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(Name, Cow<'static, str>), Error> {
        let mut outer = name.into();
        let Some(name) = self.extend_name(&outer, false) else {
            return Err(Error::InvalidVerbName(outer));
        };
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        Ok((name, outer))
    }

    fn convert<'a>(&mut self, format: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        let mut format = format.into();
        self.buffer.clear();
        self.buffer.push_str(&format);
        let inner = format.to_mut();
        inner.clear();
        inner.extend(self.case.convert(&self.buffer));
        format
    }
}

impl<S: Scope, P> Builder<S, P> {
    pub fn default<T: Clone + fmt::Debug>(
        self,
        default: impl Into<T>,
    ) -> Builder<S, Default<P, impl Fn() -> T>>
    where
        P: Parse<Value = Option<T>>,
    {
        let default = default.into();
        let format = format!("{default:?}");
        self.default_with(move || default.clone(), format)
    }

    pub fn default_with<T, F: Fn() -> T>(
        mut self,
        default: F,
        format: impl Into<Cow<'static, str>>,
    ) -> Builder<S, Default<P, F>>
    where
        P: Parse<Value = Option<T>>,
    {
        let format = self.convert(format);
        self.meta(Meta::Default(format))
            .map_parse(|parse| Default(parse, default))
    }

    pub fn environment<T: FromStr>(
        self,
        variable: impl Into<Cow<'static, str>>,
    ) -> Builder<S, Environment<P, impl Fn(&str) -> Option<T>>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.environment_with(variable, |value| value.parse().ok())
    }

    pub fn environment_with<T, F: Fn(&str) -> Option<T>>(
        self,
        variable: impl Into<Cow<'static, str>>,
        parse: F,
    ) -> Builder<S, Environment<P, F>>
    where
        P: Parse<Value = Option<T>>,
    {
        let variable = variable.into();
        self.meta(Meta::Environment(variable.clone()))
            .map_parse(|inner| Environment(inner, variable, parse))
    }

    pub fn many<T, I: default::Default + Extend<T>>(
        self,
    ) -> Builder<S, Many<P, I, impl Fn() -> I, impl Fn(&mut I, T)>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.many_with(Some(NonZeroUsize::MIN), I::default, |items, item| {
            items.extend([item])
        })
    }

    pub fn many_with<T, I, N: Fn() -> I, F: Fn(&mut I, T)>(
        self,
        per: Option<NonZeroUsize>,
        new: N,
        add: F,
    ) -> Builder<S, Many<P, I, N, F>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.meta(Meta::Many(per)).map_parse(|parse| Many {
            parse,
            per,
            new,
            add,
            _marker: PhantomData,
        })
    }

    pub fn require<T: 'static>(self) -> Builder<S, Require<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.require_with(type_name::<T>())
    }

    pub fn require_with<T>(mut self, format: impl Into<Cow<'static, str>>) -> Builder<S, Require<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        let format = self.convert(format);
        self.meta(Meta::Require(format))
            .map_parse(|parse| Require(parse))
    }

    pub fn help(self, help: impl Into<Cow<'static, str>>) -> Self {
        let help = help.into();
        if help.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Help(help))
        }
    }

    pub fn line(self) -> Self {
        self.meta(Meta::Line)
    }

    pub fn note(self, note: impl Into<Cow<'static, str>>) -> Self {
        let note = note.into();
        if note.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Note(note))
        }
    }

    pub fn hide(self) -> Self {
        self.meta(Meta::Hide)
    }

    pub fn show(self) -> Self {
        self.meta(Meta::Show)
    }

    fn try_meta(mut self, meta: Result<Meta, Error>) -> Self {
        match meta {
            Ok(meta) => {
                self.scope.push(meta);
                self
            }
            Err(error) => {
                self.parse = Err(error);
                self
            }
        }
    }

    fn meta(self, meta: Meta) -> Self {
        self.try_meta(Ok(meta))
    }
}

impl<S: scope::Node, P> Builder<S, P> {
    pub fn usage(self, usage: impl Into<Cow<'static, str>>) -> Self {
        let usage = usage.into();
        if usage.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Usage(usage))
        }
    }

    pub fn group<Q>(
        self,
        build: impl FnOnce(Builder<scope::Group, At>) -> Builder<scope::Group, Q>,
    ) -> Builder<S, P::Push<Q>>
    where
        P: Stack,
    {
        let (scope, old, builder) = self.swap_both(scope::Group::new(), At(()));
        let (scope, mut builder) = build(builder).swap_scope(scope);
        builder.scope.push(Meta::from(scope));
        builder.try_map_parse(|new| Ok(old?.push(new)))
    }

    pub fn verb<Q>(
        self,
        build: impl FnOnce(Builder<scope::Verb, At>) -> Builder<scope::Verb, Q>,
    ) -> Builder<S, P::Push<With<Node<Q>>>>
    where
        P: Stack,
    {
        let (scope, old, mut builder) = self.swap_both(scope::Verb::new(), At(()));
        let position = replace(&mut builder.position, 0);
        let (verb, mut builder) = build(builder).swap_scope(scope);
        let mut meta = Meta::from(verb);
        let pair = builder.descend(&mut meta);
        builder.position = position;
        builder.scope.push(meta.clone(1));
        builder.try_map_parse(|new| {
            let (indices, set) = pair?;
            Ok(old?.push(With {
                parse: Node {
                    parse: new,
                    indices,
                },
                meta,
                set,
            }))
        })
    }

    pub fn option<T: FromStr + 'static, Q>(
        self,
        build: impl FnOnce(Builder<scope::Option, Value<T>>) -> Builder<scope::Option, Q>,
    ) -> Builder<S, P::Push<With<Q>>>
    where
        P: Stack,
    {
        let tag = self.tag.clone();
        let (scope, old, builder) = self.swap_both(
            scope::Option::new(),
            Value {
                tag: if TypeId::of::<T>() == TypeId::of::<bool>() {
                    Some(tag)
                } else {
                    None
                },
                _marker: PhantomData,
            },
        );
        let (option, mut builder) = build(builder.parse::<T>()).swap_scope(scope);
        let mut meta = Meta::from(option);
        let pair = builder.descend(&mut meta);
        builder.scope.push(meta.clone(1));
        builder.try_map_parse(|new| {
            let (_, set) = pair?;
            Ok(old?.push(With {
                parse: new,
                set,
                meta,
            }))
        })
    }

    pub fn options(self, options: impl IntoIterator<Item = Options>) -> Self {
        options
            .into_iter()
            .map(Meta::Options)
            .fold(self, Builder::meta)
    }
}

impl<S: scope::Version, P> Builder<S, P> {
    pub fn version(self, version: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Version(version.into()))
    }

    pub fn summary(self, summary: impl Into<Cow<'static, str>>) -> Self {
        let summary = summary.into();
        if summary.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Summary(summary))
        }
    }
}

impl Parser<()> {
    pub fn builder() -> Builder<scope::Root> {
        Builder::new()
    }
}

impl Builder<scope::Root> {
    fn new() -> Self {
        let case = Case::Kebab { upper: false };
        Self {
            case,
            tag: case.convert("true").collect(),
            short: Cow::Borrowed("-"),
            long: Cow::Borrowed("--"),
            buffer: String::new(),
            parse: Ok(At(())),
            scope: scope::Root::new(),
            style: Box::new(style::Termion),
            position: 0,
        }
    }

    pub fn case(mut self, case: Case) -> Self {
        self.case = case;
        self.tag = case.convert("true").collect();
        self
    }

    pub fn prefix(
        mut self,
        short: impl Into<Cow<'static, str>>,
        long: impl Into<Cow<'static, str>>,
    ) -> Self {
        let short = short.into();
        let long = long.into();
        if short == long
            || short.is_empty()
            || short.chars().any(|letter| letter.is_ascii_alphanumeric())
            || long.is_empty()
            || long.chars().any(|letter| letter.is_ascii_alphanumeric())
        {
            self.try_map_parse(|_| Err(Error::InvalidPrefix(short, long)))
        } else {
            self.short = short;
            self.long = long;
            self
        }
    }
}

impl<P> Builder<scope::Root, P> {
    pub fn build(self) -> Result<Parser<With<Node<P>>>, Error> {
        let (root, mut builder) = self.swap_scope(());
        let mut meta = Meta::from(root);
        let (indices, set) = builder.descend(&mut meta)?;
        Ok(Parser {
            short: builder.short,
            long: builder.long,
            style: builder.style,
            parse: With {
                parse: Node {
                    indices,
                    parse: builder.parse?,
                },
                meta,
                set,
            },
        })
    }

    pub fn style<S: style::Style + 'static>(mut self, style: S) -> Self {
        self.style = Box::new(style);
        self
    }

    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        if name.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Name(Name::Plain, name))
        }
    }

    pub fn license(
        self,
        name: impl Into<Cow<'static, str>>,
        file: impl Into<Cow<'static, str>>,
    ) -> Self {
        let name = name.into();
        let content = file.into();
        if name.chars().all(char::is_whitespace) && content.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::License(name, content))
        }
    }

    pub fn author(self, author: impl Into<Cow<'static, str>>) -> Self {
        let author = author.into();
        if author.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Author(author))
        }
    }

    pub fn repository(self, repository: impl Into<Cow<'static, str>>) -> Self {
        let repository = repository.into();
        if repository.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Repository(repository))
        }
    }

    pub fn home(self, home: impl Into<Cow<'static, str>>) -> Self {
        let home = home.into();
        if home.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Home(home))
        }
    }
}

impl<P> Builder<scope::Group, P> {
    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        if name.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Name(Name::Plain, name))
        }
    }
}

impl<P> Builder<scope::Verb, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let meta = self.verb_name(name).map(|pair| Meta::Name(pair.0, pair.1));
        self.try_meta(meta)
    }
}

impl Builder<scope::Option, Value<Unit>> {
    pub fn parse<T: FromStr + 'static>(self) -> Builder<scope::Option, Value<T>> {
        self.parse_with(
            if TypeId::of::<T>() == TypeId::of::<bool>() {
                Some("true")
            } else {
                None
            },
            type_name::<T>(),
        )
    }

    pub fn parse_with<T: FromStr>(
        mut self,
        tag: Option<impl Into<Cow<'static, str>>>,
        format: impl Into<Cow<'static, str>>,
    ) -> Builder<scope::Option, Value<T>> {
        let format = self.convert(format);
        self.meta(Meta::Type(format)).map_parse(|_| Value {
            tag: tag.map(Into::into),
            _marker: PhantomData,
        })
    }
}

impl<P> Builder<scope::Option, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let meta = self
            .option_name(name)
            .map(|pair| Meta::Name(pair.0, pair.1));
        self.try_meta(meta)
    }

    pub fn position(mut self) -> Self {
        let position = self.position;
        self.position += 1;
        self.meta(Meta::Position(position))
    }

    pub fn swizzle(self) -> Self
    where
        P: Parse,
        <P as Parse>::Value: Flag,
    {
        self.meta(Meta::Swizzle)
    }

    pub fn valid(self, pattern: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Valid(pattern.into()))
    }
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

fn merge<T>(left: Option<T>, right: Option<T>, merge: impl FnOnce(T, T) -> T) -> Option<T> {
    match (left, right) {
        (None, None) => None,
        (None, Some(right)) => Some(right),
        (Some(left), None) => Some(left),
        (Some(left), Some(right)) => Some(merge(left, right)),
    }
}
