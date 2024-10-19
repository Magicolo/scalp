use crate::{
    case::Case,
    error::Error,
    meta::{Meta, Options, Text},
    parse::{self, Any, At, Default, Environment, Many, Map, Node, Parse, Parser, Require, Value},
    scope::{self, Scope},
    stack::Stack,
    style,
};
use core::{any::TypeId, default, fmt, marker::PhantomData, num::NonZeroUsize, str::FromStr};
use regex::Regex;
use std::{any, convert::Infallible, sync::Arc};

pub struct Builder<S, P = At<()>> {
    case: Case,
    tag: Text,
    prefix: char,
    parse: Result<P, Error>,
    scope: S,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    fn try_swap_map<Q>(
        self,
        prefix: char,
        case: Case,
        map: impl FnOnce(P, char, Case) -> Result<Q, Error>,
    ) -> Builder<S, Q> {
        Builder {
            case,
            tag: self.tag,
            prefix,
            scope: self.scope,
            parse: self
                .parse
                .and_then(|parse| map(parse, self.prefix, self.case)),
        }
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
            prefix: self.prefix,
            scope: scope(self.scope),
            parse: self.parse.and_then(parse),
        }
    }

    fn swap_scope<T>(self, scope: T) -> (S, Builder<T, P>) {
        (self.scope, Builder {
            case: self.case,
            tag: self.tag,
            prefix: self.prefix,
            scope,
            parse: self.parse,
        })
    }

    fn swap_both<T, Q>(self, scope: T, parse: Q) -> (S, Result<P, Error>, Builder<T, Q>) {
        (self.scope, self.parse, Builder {
            case: self.case,
            tag: self.tag,
            prefix: self.prefix,
            scope,
            parse: Ok(parse),
        })
    }

    fn convert(&mut self, format: impl Into<Text>) -> Text {
        self.case.convert(format.into().chars()).collect()
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
        format: impl Into<Text>,
    ) -> Builder<S, Default<P, F>>
    where
        P: Parse<Value = Option<T>>,
    {
        let format = self.convert(format);
        self.meta(Meta::Default(format))
            .map_parse(|parse| Default(parse, default))
    }

    pub fn environment<T: FromStr>(self, variable: impl Into<Text>) -> Builder<S, Environment<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        let variable = variable.into();
        self.meta(Meta::Environment(variable.clone()))
            .map_parse(|inner| Environment(inner, variable))
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

    pub fn require_with<T>(mut self, format: impl Into<Text>) -> Builder<S, Require<P>>
    where
        P: Parse<Value = Option<T>>,
    {
        let format = self.convert(format);
        self.meta(Meta::Require(format))
            .map_parse(|parse| Require(parse))
    }

    pub fn help(self, help: impl Into<Text>) -> Self {
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

    pub fn note(self, note: impl Into<Text>) -> Self {
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

    pub fn style<T: style::Style + 'static>(self, style: T) -> Self {
        self.meta(Meta::Style(Arc::new(style)))
    }

    fn meta(self, meta: Meta) -> Self {
        self.try_meta(Ok(meta))
    }
}

impl<S: scope::Node, P> Builder<S, P> {
    pub fn prefix(mut self, prefix: char) -> Self {
        if prefix.is_alphanumeric() || prefix.is_whitespace() || prefix.is_control() {
            self.try_map_parse(|_| Err(Error::InvalidPrefix(prefix)))
        } else {
            self.prefix = prefix;
            self.meta(Meta::Prefix(prefix))
        }
    }

    pub fn usage(self, usage: impl Into<Text>) -> Self {
        let usage = usage.into();
        if usage.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Usage(usage))
        }
    }

    #[inline]
    pub fn group<Q>(
        self,
        build: impl FnOnce(Builder<scope::Group, At>) -> Builder<scope::Group, Q>,
    ) -> Builder<S, P::Push<Parser<Q>>>
    where
        P: Stack,
    {
        group(self, build)
    }

    #[inline]
    pub fn verb<Q>(
        self,
        build: impl FnOnce(Builder<scope::Verb, At>) -> Builder<scope::Verb, Q>,
    ) -> Builder<S, P::Push<Parser<Node<Q>>>>
    where
        P: Stack,
    {
        verb(self, build)
    }

    #[inline]
    pub fn option<T: FromStr + 'static, Q>(
        self,
        build: impl FnOnce(Builder<scope::Option, Value<T>>) -> Builder<scope::Option, Q>,
    ) -> Builder<S, P::Push<Parser<Q>>>
    where
        P: Stack,
    {
        option(self, build)
    }

    pub fn options(self, options: impl IntoIterator<Item = Options>) -> Self {
        options
            .into_iter()
            .map(Meta::Options)
            .fold(self, Builder::meta)
    }
}

// impl<P: Parse> Parser<Node<P>> {
//     #[inline]
//     pub fn verb(
//         build: impl FnOnce(Builder<scope::Verb, At>) -> Builder<scope::Verb,
// P>,     ) -> Result<Self, Error> {
//         let builder = verb(Builder::new(), build);
//         Ok(builder.parse?.0)
//     }
// }

// impl<P: Parse> Parser<P> {
//     #[inline]
//     pub fn group(
//         self,
//         build: impl FnOnce(Builder<scope::Group, At>) ->
// Builder<scope::Group, P>,     ) -> Result<Self, Error> {
//         let builder = group(Builder::new(), build);
//         Ok(builder.parse?.0)
//     }

//     #[inline]
//     pub fn option<T: FromStr + 'static>(
//         build: impl FnOnce(Builder<scope::Option, Value<T>>) ->
// Builder<scope::Option, P>,     ) -> Result<Self, Error> {
//         let builder = option(Builder::new(), build);
//         Ok(builder.parse?.0)
//     }
// }

// impl Builder<scope::Root, ()> {
//     #[inline]
//     const fn new() -> Self {
//         Self {
//             case: Case::Kebab { upper: false },
//             tag: Text::Static(parse::TAG),
//             prefix: parse::PREFIX,
//             parse: Ok(()),
//             scope: scope::Root::new(),
//         }
//     }
// }

impl<P> Builder<scope::Group, P> {
    pub fn name(self, name: impl Into<Text>) -> Self {
        let name = name.into();
        if name.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Name(name))
        }
    }
}

impl<P> Builder<scope::Verb, P> {
    pub fn case(mut self, case: Case) -> Self {
        self.case = case;
        self.tag = case.convert(parse::TAG.chars()).collect();
        self
    }

    pub fn name(self, name: impl Into<Text>) -> Self {
        let name = name.into();
        self.try_meta(
            if name
                .chars()
                .any(|letter| letter.is_whitespace() || !letter.is_ascii())
            {
                Err(Error::InvalidVerbName(name))
            } else {
                Ok(Meta::Name(name))
            },
        )
    }

    pub fn version(self, version: impl Into<Text>) -> Self {
        self.meta(Meta::Version(version.into()))
    }

    pub fn summary(self, summary: impl Into<Text>) -> Self {
        let summary = summary.into();
        if summary.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Summary(summary))
        }
    }

    pub fn license(self, name: impl Into<Text>, file: impl Into<Text>) -> Self {
        let name = name.into();
        let content = file.into();
        if name.chars().all(char::is_whitespace) && content.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::License(name, content))
        }
    }

    pub fn author(self, author: impl Into<Text>) -> Self {
        let author = author.into();
        if author.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Author(author))
        }
    }

    pub fn repository(self, repository: impl Into<Text>) -> Self {
        let repository = repository.into();
        if repository.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Repository(repository))
        }
    }

    pub fn home(self, home: impl Into<Text>) -> Self {
        let home = home.into();
        if home.chars().all(char::is_whitespace) {
            self
        } else {
            self.meta(Meta::Home(home))
        }
    }
}

impl Builder<scope::Option, Value<Unit>> {
    pub fn parse<T: FromStr + 'static>(self) -> Builder<scope::Option, Value<T>> {
        self.parse_with(type_name::<T>())
    }

    pub fn parse_with<T: FromStr>(
        mut self,
        format: impl Into<Text>,
    ) -> Builder<scope::Option, Value<T>> {
        let format = self.convert(format);
        self.meta(Meta::Type(format))
            .map_parse(|_| Value(PhantomData))
    }
}

impl<P> Builder<scope::Option, P> {
    pub fn name(self, name: impl Into<Text>) -> Self {
        let name = name.into();
        self.try_meta(
            if name
                .chars()
                .any(|letter| letter.is_whitespace() || !letter.is_ascii())
            {
                Err(Error::InvalidOptionName(name))
            } else {
                Ok(Meta::Name(name))
            },
        )
    }

    pub fn position(self) -> Self {
        self.meta(Meta::Position)
    }

    pub fn swizzle(self) -> Self
    where
        P: Parse,
        <P as Parse>::Value: Flag,
    {
        self.meta(Meta::Swizzle)
    }

    pub fn valid(self, pattern: impl Into<Text>) -> Self {
        let pattern = pattern.into();
        self.try_meta(match Regex::new(&format!("^{pattern}$")) {
            Ok(regex) => Ok(Meta::Valid(regex)),
            Err(error) => Err(Error::Regex(error)),
        })
    }
}

fn group<S: scope::Scope, P, Q>(
    builder: Builder<S, P>,
    build: impl FnOnce(Builder<scope::Group, At>) -> Builder<scope::Group, Q>,
) -> Builder<S, P::Push<Parser<Q>>>
where
    P: Stack,
{
    let prefix = builder.prefix;
    let case = builder.case;
    let (scope, old, builder) = builder.swap_both(scope::Group::new(), At(()));
    let (group, mut builder) = build(builder).swap_scope(scope);
    let meta = Meta::from(group);
    builder.scope.push(meta.clone(usize::MAX));
    builder.try_swap_map(prefix, case, |new, prefix, case| {
        Ok(old?.push(Parser {
            parse: new,
            meta: Arc::new(meta),
            prefix: Some(prefix),
            case: Some(case),
        }))
    })
}

fn verb<S: scope::Scope, P, Q>(
    builder: Builder<S, P>,
    build: impl FnOnce(Builder<scope::Verb, At>) -> Builder<scope::Verb, Q>,
) -> Builder<S, P::Push<Parser<Node<Q>>>>
where
    P: Stack,
{
    let prefix = builder.prefix;
    let case = builder.case;
    let (scope, old, builder) = builder.swap_both(scope::Verb::new(), At(()));
    let (verb, mut builder) = build(builder).swap_scope(scope);
    let meta = Meta::from(verb);
    builder.scope.push(meta.clone(1));
    builder.try_swap_map(prefix, case, |new, prefix, case| {
        Ok(old?.push(Parser {
            parse: Node(new),
            meta: Arc::new(meta),
            prefix: Some(prefix),
            case: Some(case),
        }))
    })
}

fn option<T: FromStr + 'static, S: scope::Scope, P, Q>(
    builder: Builder<S, P>,
    build: impl FnOnce(Builder<scope::Option, Value<T>>) -> Builder<scope::Option, Q>,
) -> Builder<S, P::Push<Parser<Q>>>
where
    P: Stack,
{
    let prefix = builder.prefix;
    let case = builder.case;
    let (scope, old, builder) = builder.swap_both(scope::Option::new(), Value::default());
    let (option, mut builder) = build(builder.parse::<T>()).swap_scope(scope);
    let meta = Meta::from(option);
    builder.scope.push(meta.clone(1));
    builder.try_swap_map(prefix, case, |new, prefix, case| {
        Ok(old?.push(Parser {
            parse: new,
            meta: Arc::new(meta),
            prefix: Some(prefix),
            case: Some(case),
        }))
    })
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
