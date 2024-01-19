use crate::{
    error::Error,
    parse::{Any, At, Default, Environment, Many, Map, Node, Parse, Parser, Require, Value},
    scope::{self, Scope},
    stack::{Count, Push},
    Meta, Options, BREAK, HELP, MAXIMUM, SHIFT,
};
use scalp_core::case::Case;
use std::{
    any::type_name,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    default,
    fmt::Display,
    marker::PhantomData,
    str::FromStr,
};

pub struct Builder<S, P> {
    case: Case,
    short: Cow<'static, str>,
    long: Cow<'static, str>,
    buffer: String,
    parse: P,
    scope: S,
}

impl default::Default for Builder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder<(), ()> {
    pub const fn new() -> Self {
        Self {
            case: Case::Kebab,
            short: Cow::Borrowed("-"),
            long: Cow::Borrowed("--"),
            buffer: String::new(),
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
        B: FnOnce(Builder<scope::Root, At>) -> Result<Builder<scope::Root, Q>, Error>,
    >(
        self,
        build: B,
    ) -> Result<Builder<(), Node<Q>>, Error> {
        let (mut root, mut builder) =
            build(self.map_both(|_| scope::Root::new(), |_| At(())))?.swap_scope(());
        let mut indices = HashMap::new();
        builder.descend(&mut root, 0, 0, &mut indices, true)?;
        Ok(builder.map_parse(|parse| Node {
            parse,
            indices,
            meta: root.into(),
        }))
    }
}

impl<S, P> Builder<S, P> {
    fn descend(
        &mut self,
        metas: &mut Vec<Meta>,
        mask: usize,
        shift: u32,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        top: bool,
    ) -> Result<(bool, bool), Error> {
        let mut index = 0;
        let mut version = true;
        let mut help = true;
        for i in 0..metas.len() {
            let value = (index << shift) | mask;
            match metas.get_mut(i) {
                Some(Meta::Option(metas)) => {
                    self.descend_option(metas, indices, value)?;
                    index += 1;
                }
                Some(Meta::Verb(metas)) => {
                    self.descend_verb(metas, indices, value)?;
                    index += 1;
                }
                Some(Meta::Group(_)) if shift > MAXIMUM => {
                    return Err(Error::GroupNestingLimitOverflow)
                }
                Some(Meta::Group(metas)) => {
                    let pair = self.descend(metas, value, shift + SHIFT, indices, false)?;
                    version &= pair.0;
                    help &= pair.1;
                    index += 1;
                }
                Some(Meta::Options(Options::Help)) => {
                    self.insert_help(metas, indices)?;
                    help = false;
                }
                Some(Meta::Options(Options::Version)) => {
                    self.insert_version(metas, indices)?;
                    version = false;
                }
                None => break,
                _ => {}
            }
        }

        if top {
            if version {
                self.insert_version(metas, indices)?;
            }
            if help {
                self.insert_help(metas, indices)?;
            }
            Self::insert_key(self.long.clone(), indices, BREAK)?;
        }
        Ok((version, help))
    }

    fn descend_verb(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
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
        indices: &mut HashMap<Cow<'static, str>, usize>,
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

    fn insert_version(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
    ) -> Result<(), Error> {
        let names = (self.option_name("version"), self.option_name("v"));
        Self::insert_key(names.0.clone(), indices, HELP)?;
        Self::insert_key(names.1.clone(), indices, HELP)?;
        metas.push(Meta::Option(vec![
            Meta::Help(Cow::Borrowed("Displays version information.")),
            Meta::Name(names.0),
            Meta::Name(names.1),
        ]));
        Ok(())
    }

    fn insert_help(
        &mut self,
        metas: &mut Vec<Meta>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
    ) -> Result<(), Error> {
        let names = (self.option_name("help"), self.option_name("h"));
        Self::insert_key(names.0.clone(), indices, HELP)?;
        Self::insert_key(names.1.clone(), indices, HELP)?;
        metas.push(Meta::Option(vec![
            Meta::Help(Cow::Borrowed("Displays this help message.")),
            Meta::Name(names.0),
            Meta::Name(names.1),
        ]));
        Ok(())
    }

    fn insert_key(
        key: Cow<'static, str>,
        indices: &mut HashMap<Cow<'static, str>, usize>,
        index: usize,
    ) -> Result<(), Error> {
        match indices.entry(key) {
            Entry::Occupied(entry) => Err(Error::DuplicateName {
                name: entry.key().clone(),
            }),
            Entry::Vacant(entry) => {
                entry.insert(index);
                Ok(())
            }
        }
    }

    fn option_name(&mut self, name: impl Into<Cow<'static, str>>) -> Cow<'static, str> {
        let mut outer = name.into();
        let name = outer.trim();
        match name.len() {
            0 => return outer,
            1 => {
                self.buffer.clear();
                self.buffer.push_str(&self.short);
                self.buffer.push_str(name);
            }
            2.. => {
                self.buffer.clear();
                self.buffer.push_str(&self.long);
                if self.case.convert_in(name, &mut self.buffer).is_err() {
                    return outer;
                }
            }
        }
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        outer
    }

    fn verb_name(&mut self, name: impl Into<Cow<'static, str>>) -> Cow<'static, str> {
        let mut outer = name.into();
        let name = outer.trim();
        match name.len() {
            0 => return outer,
            1 => {
                self.buffer.clear();
                self.buffer.push_str(name);
            }
            2.. => {
                self.buffer.clear();
                if self.case.convert_in(name, &mut self.buffer).is_err() {
                    return outer;
                }
            }
        }
        let inner = outer.to_mut();
        inner.clear();
        inner.push_str(&self.buffer);
        outer
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
                scope,
                parse,
            },
        )
    }
}

impl<S: Scope, P> Builder<S, P> {
    pub fn help(self, help: impl Into<Cow<'static, str>>) -> Self {
        self.meta(Meta::Help(help.into()))
    }

    pub fn hide(self) -> Self {
        self.meta(Meta::Hide)
    }

    fn meta(mut self, meta: Meta) -> Self {
        self.scope.push(meta);
        self
    }

    fn type_name<T>(mut self) -> Self {
        let Some(name) = type_name::<T>().split("::").last() else {
            return self;
        };
        self.buffer.clear();
        let Ok(_) = self.case.convert_in(name, &mut self.buffer) else {
            return self;
        };
        let value = self.buffer.clone();
        self.meta(Meta::Type(Cow::Owned(value)))
    }
}

impl<S: scope::Node, P> Builder<S, P> {
    pub fn group<Q>(
        self,
        build: impl FnOnce(Builder<scope::Group, At>) -> Result<Builder<scope::Group, Q>, Error>,
    ) -> Result<Builder<S, P::Output>, Error>
    where
        P: Push<Q>,
    {
        let (scope, old, group) = self.swap_both(scope::Group::new(), At(()));
        let (scope, mut builder) = build(group)?
            .map_parse(|new| old.push(new))
            .swap_scope(scope);
        builder.scope.push(scope.into());
        Ok(builder)
    }

    pub fn verb<Q>(
        self,
        build: impl FnOnce(Builder<scope::Verb, At>) -> Result<Builder<scope::Verb, Q>, Error>,
    ) -> Result<Builder<S, P::Output>, Error>
    where
        P: Push<Node<Q>>,
    {
        let (scope, old, builder) = self.swap_both(scope::Verb::new(), At(()));
        let (mut verb, mut builder) = build(builder)?.swap_scope(scope);
        let mut indices = HashMap::new();
        builder.descend(&mut verb, 0, 0, &mut indices, true)?;
        let meta = Meta::from(verb);
        builder.scope.push(meta.clone(1));
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
        let (option, mut builder) = build(option.type_name::<T>())
            .map_parse(|new| old.push(new))
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

impl<P> Builder<scope::Verb, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = self.verb_name(name);
        self.meta(Meta::Name(name))
    }
}

impl<P> Builder<scope::Option, P> {
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        let name = self.option_name(name);
        self.meta(Meta::Name(name))
    }

    pub fn default<T: Clone + Display>(self, default: T) -> Builder<scope::Option, Default<P, T>>
    where
        P: Parse<Value = Option<T>>,
    {
        self.meta(Meta::Default(Cow::Owned(format!("{default}"))))
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
        self.meta(Meta::Many)
            .map_parse(|parse| Many(parse, per, PhantomData))
    }
}
