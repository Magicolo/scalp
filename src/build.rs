use crate::{
    case::Case,
    error::Error,
    parse::{
        Any, At, Default, Environment, Indices, Many, Map, Node, Parse, Parser, Require, Value,
    },
    scope::{self, Scope},
    stack::Stack,
    Meta, Options, BREAK, HELP, MAXIMUM, SHIFT, VERSION,
};
use core::fmt;
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::hash_map::Entry,
    default,
    marker::PhantomData,
    str::FromStr,
};

pub struct Builder<S, P = At<()>> {
    case: Case,
    short: Cow<'static, str>,
    long: Cow<'static, str>,
    buffer: String,
    parse: Result<P, Error>,
    scope: S,
}

impl default::Default for Builder<scope::Root> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, P> Builder<S, P> {
    fn descend(&mut self, meta: &mut Meta, indices: &mut Indices) -> Result<(), Error> {
        let (version, help, metas) = match meta {
            Meta::Root(metas) | Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas) => {
                let pair = self.descend_node(metas, 0, 0, indices)?;
                (pair.0, pair.1, metas)
            }
            _ => return Ok(()),
        };
        if let Some(true) = version {
            self.insert_version(metas, indices, true, true)?;
        }
        if let None | Some(true) = help {
            self.insert_help(metas, indices, true, true)?;
        }
        Self::insert_key(self.long.clone(), indices, BREAK)?;
        Ok(())
    }

    fn descend_node(
        &mut self,
        metas: &mut Vec<Meta>,
        mask: usize,
        shift: u32,
        indices: &mut Indices,
    ) -> Result<(Option<bool>, Option<bool>), Error> {
        let mut index = 0;
        let mut version = None;
        let mut help = None;
        let mut hide = 0;
        for i in 0..metas.len() {
            let value = (index << shift) | mask;
            match metas.get_mut(i) {
                Some(Meta::Version(_)) => {
                    if hide == 0 {
                        version = version.or(Some(true))
                    }
                }
                Some(Meta::Help(_) | Meta::Name(_) | Meta::Usage(_) | Meta::Note(_)) => {
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Some(Meta::Hide) => hide += 1,
                Some(Meta::Show) => hide = usize::saturating_sub(hide, 1),
                Some(Meta::Option(metas)) => {
                    self.descend_option(metas, indices, value)?;
                    index += 1;
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Some(Meta::Verb(metas)) => {
                    self.descend_verb(metas, indices, value)?;
                    index += 1;
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Some(Meta::Group(_)) if shift > MAXIMUM => {
                    return Err(Error::GroupNestingLimitOverflow)
                }
                Some(Meta::Group(metas)) => {
                    let pair = self.descend_node(metas, value, shift + SHIFT, indices)?;
                    version = match (version, pair.0) {
                        (None, None) => None,
                        (None, Some(right)) => Some(right),
                        (Some(left), None) => Some(left),
                        (Some(left), Some(right)) => Some(left && right),
                    };
                    help = match (help, pair.0) {
                        (None, None) => None,
                        (None, Some(right)) => Some(right),
                        (Some(left), None) => Some(left),
                        (Some(left), Some(right)) => Some(left && right),
                    };
                    index += 1;
                }
                Some(Meta::Options(Options::Version { short, long })) => {
                    let (short, long) = (*short, *long);
                    self.insert_version(metas, indices, short, long)?;
                    version = Some(false);
                    if hide == 0 {
                        help = help.or(Some(true))
                    }
                }
                Some(Meta::Options(Options::Help { short, long })) => {
                    let (short, long) = (*short, *long);
                    self.insert_help(metas, indices, short, long)?;
                    help = Some(false);
                }
                None => break,
                _ => {}
            }
        }
        Ok((version, help))
    }

    fn descend_verb(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(name)) => Self::insert_key(name.clone(), indices, index)?,
                None => break,
                _ => {}
            };
        }
        Ok(())
    }

    fn descend_option(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        for i in 0..metas.len() {
            match metas.get(i) {
                Some(Meta::Name(name)) => Self::insert_key(name.clone(), indices, index)?,
                Some(Meta::Position) => indices.1.push(index),
                None => break,
                _ => {}
            };
        }
        Ok(())
    }

    fn insert_version(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<(), Error> {
        let mut option = vec![Meta::Help(Cow::Borrowed("Displays version information."))];
        if short {
            let name = self.option_name("v")?;
            if Self::insert_key(name.clone(), indices, VERSION).is_ok() {
                option.push(Meta::Name(name));
            }
        }
        if long {
            let name = self.option_name("version")?;
            if Self::insert_key(name.clone(), indices, VERSION).is_ok() {
                option.push(Meta::Name(name));
            }
        }
        if option.len() > 1 {
            metas.push(Meta::Option(option));
        }
        Ok(())
    }

    fn insert_help(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut Indices,
        short: bool,
        long: bool,
    ) -> Result<(), Error> {
        let mut option = vec![Meta::Help(Cow::Borrowed("Displays this help message."))];
        if short {
            let name = self.option_name("h")?;
            if Self::insert_key(name.clone(), indices, HELP).is_ok() {
                option.push(Meta::Name(name));
            }
        }
        if long {
            let name = self.option_name("help")?;
            if Self::insert_key(name.clone(), indices, HELP).is_ok() {
                option.push(Meta::Name(name));
            }
        }
        if option.len() > 1 {
            metas.push(Meta::Option(option));
        }
        Ok(())
    }

    fn insert_key(
        key: Cow<'static, str>,
        indices: &mut Indices,
        index: usize,
    ) -> Result<(), Error> {
        match indices.0.entry(key) {
            Entry::Occupied(entry) => Err(Error::DuplicateName {
                name: entry.key().clone(),
            }),
            Entry::Vacant(entry) => {
                entry.insert(index);
                Ok(())
            }
        }
    }

    fn option_name(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Cow<'static, str>, Error> {
        let mut outer = name.into();
        let name = outer.trim();
        match name.len() {
            0 => return Err(Error::InvalidName { name: outer }),
            1 => {
                self.buffer.clear();
                self.buffer.push_str(&self.short);
                self.buffer.push_str(name);
            }
            2.. => {
                self.buffer.clear();
                self.buffer.push_str(&self.long);
                self.case.convert_in(name, &mut self.buffer)?;
            }
        }
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        Ok(outer)
    }

    fn verb_name(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Cow<'static, str>, Error> {
        let mut outer = name.into();
        let name = outer.trim();
        match name.len() {
            0 => return Err(Error::InvalidName { name: outer }),
            1 => {
                self.buffer.clear();
                self.buffer.push_str(name);
            }
            2.. => {
                self.buffer.clear();
                self.case.convert_in(name, &mut self.buffer)?;
            }
        }
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        Ok(outer)
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

    pub fn try_map<T, F: Fn(P::Value) -> Result<T, Error>>(self, map: F) -> Builder<S, Map<P, F>>
    where
        P: Parse,
    {
        self.map_parse(|parse| Map(parse, map))
    }

    pub fn boxed(self) -> Builder<S, Box<dyn Parse<Value = P::Value, State = P::State>>>
    where
        P: Parse + 'static,
    {
        self.map_parse(|parse| Box::new(parse) as _)
    }

    pub fn any<T>(self) -> Builder<S, Map<P, impl Fn(P::Value) -> Result<Option<T>, Error>>>
    where
        P: Parse,
        P::Value: Any<T>,
    {
        self.map(Any::any)
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
            short: self.short,
            long: self.long,
            buffer: self.buffer,
            scope: scope(self.scope),
            parse: self.parse.and_then(parse),
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
                scope,
                parse: self.parse,
            },
        )
    }

    fn swap_both<T, Q>(self, scope: T, parse: Q) -> (S, Result<P, Error>, Builder<T, Q>) {
        (
            self.scope,
            self.parse,
            Builder {
                case: self.case,
                short: self.short,
                long: self.long,
                buffer: self.buffer,
                scope,
                parse: Ok(parse),
            },
        )
    }
}

macro_rules! is {
    ($left: expr $(, $rights: ident)+) => {
        $($left == TypeId::of::<$rights>() || $left == TypeId::of::<Option<$rights>>() ||)+ false
    };
}

impl<S: Scope, P> Builder<S, P> {
    pub fn help(self, help: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Help(help.into()))
    }

    pub fn note(self, help: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Note(help.into()))
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

    fn type_name<T: 'static>(mut self) -> Self {
        let name = type_name::<T>();
        let Some(name) = name.split('<').next() else {
            return self;
        };
        let Some(name) = name.split(':').last() else {
            return self;
        };
        let identifier = TypeId::of::<T>();
        let name = if is!(identifier, bool) {
            Cow::Borrowed("boolean")
        } else if is!(identifier, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize) {
            Cow::Borrowed("integer")
        } else if is!(identifier, f32, f64) {
            Cow::Borrowed("number")
        } else {
            self.buffer.clear();
            let Ok(_) = self.case.convert_in(name, &mut self.buffer) else {
                return self;
            };
            Cow::Owned(self.buffer.clone())
        };
        self.meta(Meta::Type(name, identifier))
    }
}

impl<S: scope::Parent, P> Builder<S, P> {
    pub fn usage(self, help: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Usage(help.into()))
    }

    pub fn group<Q>(
        self,
        build: impl FnOnce(Builder<scope::Group, At>) -> Builder<scope::Group, Q>,
    ) -> Builder<S, P::Push<Q>>
    where
        P: Stack,
    {
        let (scope, old, group) = self.swap_both(scope::Group::new(), At(()));
        let (scope, mut builder) = build(group)
            .try_map_parse(|new| Ok(old?.push(new)))
            .swap_scope(scope);
        builder.scope.push(scope.into());
        builder
    }

    pub fn verb<Q>(
        self,
        build: impl FnOnce(Builder<scope::Verb, At>) -> Builder<scope::Verb, Q>,
    ) -> Builder<S, P::Push<Node<Q>>>
    where
        P: Stack,
    {
        let (scope, old, builder) = self.swap_both(scope::Verb::new(), At(()));
        let (verb, mut builder) = build(builder).swap_scope(scope);
        let mut indices = Indices::default();
        let mut meta = Meta::from(verb);
        let result = builder.descend(&mut meta, &mut indices);
        builder.scope.push(meta.clone(1));
        builder.try_map_parse(|new| {
            result?;
            Ok(old?.push(Node {
                parse: new,
                indices,
                meta,
            }))
        })
    }

    pub fn option<T: FromStr + 'static, Q>(
        self,
        build: impl FnOnce(Builder<scope::Option, Value<T>>) -> Builder<scope::Option, Q>,
    ) -> Builder<S, P::Push<Q>>
    where
        P: Stack,
    {
        let (scope, old, option) = self.swap_both(scope::Option::new(), Value(PhantomData));
        let (option, mut builder) = build(option.type_name::<T>())
            .try_map_parse(|new| Ok(old?.push(new)))
            .swap_scope(scope);
        builder.scope.push(option.into());
        builder
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
}

impl Builder<scope::Root> {
    pub const fn new() -> Self {
        Self {
            case: Case::Kebab,
            short: Cow::Borrowed("-"),
            long: Cow::Borrowed("--"),
            buffer: String::new(),
            parse: Ok(At(())),
            scope: scope::Root::new(),
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
}

impl<P> Builder<scope::Root, P> {
    pub fn build(self) -> Result<Parser<Node<P>>, Error>
    where
        P: Parse,
    {
        let (root, mut builder) = self.swap_scope(());
        let mut indices = Indices::default();
        let mut meta = Meta::from(root);
        builder.descend(&mut meta, &mut indices)?;
        Ok(Parser {
            short: builder.short,
            long: builder.long,
            parse: Node {
                indices,
                meta,
                parse: builder.parse?,
            },
        })
    }

    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Name(name.into()))
    }
}

impl<P> Builder<scope::Group, P> {
    pub fn name(self, name: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Name(name.into()))
    }
}

impl<P> Builder<scope::Verb, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let meta = self.verb_name(name).map(Meta::Name);
        self.try_meta(meta)
    }
}

impl<P> Builder<scope::Option, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let meta = self.option_name(name).map(Meta::Name);
        self.try_meta(meta)
    }

    pub fn position(self) -> Self {
        self.meta(Meta::Position)
    }

    pub fn default<T: Clone + fmt::Debug>(
        self,
        default: impl Into<T>,
    ) -> Builder<scope::Option, Default<P, T>>
    where
        P: Parse<Value = Option<T>>,
    {
        let default = default.into();
        let display = format!("{default:?}");
        self.default_with(default, display)
    }

    pub fn default_with<T: Clone>(
        self,
        default: T,
        debug: impl Into<Cow<'static, str>>,
    ) -> Builder<scope::Option, Default<P, T>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.meta(Meta::Default(debug.into()))
            .map_parse(|parse| Default(parse, default))
    }

    pub fn environment<T: FromStr>(
        self,
        variable: impl Into<Cow<'static, str>>,
    ) -> Builder<scope::Option, Environment<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        let variable = variable.into();
        self.meta(Meta::Environment(variable.clone()))
            .map_parse(|parse| Environment(parse, variable))
    }

    pub fn require(self) -> Builder<scope::Option, Require<P>> {
        self.meta(Meta::Required).map_parse(Require)
    }

    pub fn many<T, I: default::Default + Extend<T>>(
        self,
        per: Option<usize>,
    ) -> Builder<scope::Option, Many<P, I>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.meta(Meta::Many(per))
            .map_parse(|parse| Many(parse, per, PhantomData))
    }
}
