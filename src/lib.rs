mod build;
mod case;
mod error;
mod help;
mod parse;
pub mod scope;
mod spell;
mod stack;

pub use crate::{
    build::Builder,
    error::Error,
    parse::{Parse, Parser},
    scope::Scope,
};
use std::{any::TypeId, borrow::Cow};

/*
    TODO:
    - Could I use the same 'Parse' trait to generate an API?
    - Generate usage string automatically.
        - Usage: {verb (for root use the root name)} [position options (if any)] [named options (if any)] {sub-command (if any)}
    - Support for styled formatting out of the box; use a feature?
    - Parse with graceful handling of 'Error::Help' and 'Error::Version'.
    - Support for indexed arguments.
    - Ensure that variables don't obscure the context variable.
    - Support for streamed arguments via stdin, file system, http.
    - Support for a value with --help
        - Allows to provide a help context when help becomes very large (ex: --help branch)
    - Autocomplete?
    - Simplify the 'Into<Cow<'static, str>>' all over the place, if possible.
        - There are probably some places where the `Cow` isn't useful.
    - Add support for combined flags using the short names when possible.
        - Short names must be of length 1.
        - ex: ls -l -a -r -t => ls -lart
    - Can I unify 'Builder' and 'Parser'?
    - Allow to rename '--help' and '--version'?
    - Support for json values.
    - Find a way to get rid of the '.ok()'. It is very confusing.
    - What if an option has an child that is an option/verb/Group?
    - Different kinds of 'help' such as 'usage', 'summary', 'detail'; that will be displayed in different contexts.
        - The motivation comes from differentiating the 'summary' help and the 'detail' help.
        - Summaries will be shown from the parent node.
        - Details will be shown only for the specific node.
        - Maybe show the help only at the current node level and require a parameter to show from the parent.
    - Display the valid values for enums.
    - Format default enum values with proper casing when using '.default()'.
*/

#[derive(Debug)]
pub enum Meta {
    Name(Cow<'static, str>),
    Position,
    Version(Cow<'static, str>),
    Help(Cow<'static, str>),
    Usage(Cow<'static, str>),
    Note(Cow<'static, str>),
    Type(Cow<'static, str>, TypeId),
    Required,
    Many(Option<usize>),
    Default(Cow<'static, str>),
    Environment(Cow<'static, str>),
    Show, // TODO: Add Show?
    Hide, // TODO: Add Show?
    Root(Vec<Meta>),
    Option(Vec<Meta>),
    Options(Options),
    Verb(Vec<Meta>),
    Group(Vec<Meta>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Options {
    Version { short: bool, long: bool },
    Help { short: bool, long: bool },
}

impl Options {
    pub const fn version(short: bool, long: bool) -> Self {
        Self::Version { short, long }
    }

    pub const fn help(short: bool, long: bool) -> Self {
        Self::Help { short, long }
    }
}

impl Meta {
    pub fn clone(&self, depth: usize) -> Self {
        match self {
            Meta::Name(value) => Meta::Name(value.clone()),
            Meta::Position => Meta::Position,
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Usage(value) => Meta::Usage(value.clone()),
            Meta::Note(value) => Meta::Note(value.clone()),
            Meta::Type(value, identifier) => Meta::Type(value.clone(), *identifier),
            Meta::Required => Meta::Required,
            Meta::Many(value) => Meta::Many(*value),
            Meta::Default(value) => Meta::Default(value.clone()),
            Meta::Environment(value) => Meta::Environment(value.clone()),
            Meta::Hide => Meta::Hide,
            Meta::Show => Meta::Show,
            Meta::Root(metas) if depth > 0 => {
                Meta::Root(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Root(_) => Meta::Root(Vec::new()),
            Meta::Option(metas) if depth > 0 => {
                Meta::Option(metas.iter().map(|meta| meta.clone(depth - 1)).collect())
            }
            Meta::Option(_) => Meta::Option(Vec::new()),
            Meta::Options(options) => Meta::Options(*options),
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

const HELP: usize = usize::MAX;
const VERSION: usize = usize::MAX - 1;
const BREAK: usize = usize::MAX - 2;
const SHIFT: u32 = 5;
const MASK: usize = (1 << SHIFT) - 1;
const MAXIMUM: u32 = usize::BITS - 14;

// mod boba {
//     use crate::{
//         error::Error,
//         parse::Parse,
//         parse::{At, Value},
//         scope::{self, Scope},
//         stack::Stack,
//     };
//     use std::{marker::PhantomData, str::FromStr};

//     pub struct Builder<S, P> {
//         scope: S,
//         parse: P,
//     }

//     pub struct Invalid<S, P>(S, P);

//     pub fn root() -> impl Build<Scope = scope::Root, Parse = At<()>> {
//         Builder {
//             scope: scope::Root::new(),
//             parse: Value(PhantomData),
//         }
//     }
//     pub fn group() -> impl Build<Scope = scope::Group, Parse = At<()>> {
//         Builder {
//             scope: scope::Group::new(),
//             parse: Value(PhantomData),
//         }
//     }
//     pub fn verb() -> impl Build<Scope = scope::Verb, Parse = At<()>> {
//         Builder {
//             scope: scope::Verb::new(),
//             parse: Value(PhantomData),
//         }
//     }
    
//     pub fn option<T: FromStr + 'static>() -> impl Build<Scope = scope::Option, Parse = Value<T>> {
//         Builder {
//             scope: scope::Option::new(),
//             parse: Value(PhantomData),
//         }
//     }

//     pub trait Build {
//         type Scope: Scope;
//         type Parse: Parse;
//         type Next<P>: Build<Scope = Self::Scope>;

//         fn parts(self) -> (Self::Scope, Self::Parse);
//         fn build(self) -> Result<Self::Parse, Error>;
//         fn child<B: Build>(self, child: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child;
//     }

//     impl<S: Scope, P: Parse> Build for Invalid<S, P> {
//         type Scope = S;
//         type Parse = P;
//         type Next<Q> = Self;

//         fn parts(self) -> (Self::Scope, Self::Parse) {
//             (self.0, self.1)
//         }

//         fn build(self) -> Result<Self::Parse, Error> {
//             todo!()
//         }

//         fn child<B: Build>(self, child: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child,
//         {
//             self
//         }
//     }

//     impl<P: Parse + Stack> Build for Builder<scope::Root, P> {
//         type Scope = scope::Root;
//         type Parse = P;
//         type Next<Q> = Builder<scope::Root, P::Push<Q>>;

//         fn parts(self) -> (Self::Scope, Self::Parse) {
//             (self.scope, self.parse)
//         }

//         fn build(self) -> Result<Self::Parse, Error> {
//             todo!()
//         }

//         fn child<B: Build>(mut self, child: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child,
//         {
//             let pair = child.parts();
//             self.scope.push(pair.0.into());
//             Builder {
//                 scope: self.scope,
//                 parse: self.parse.push(pair.1),
//             }
//         }
//     }

//     impl<P: Parse + Stack> Build for Builder<scope::Group, P> {
//         type Scope = scope::Group;
//         type Parse = P;
//         type Next<Q> = Builder<scope::Group, P::Push<Q>>;

//         fn parts(self) -> (Self::Scope, Self::Parse) {
//             (self.scope, self.parse)
//         }

//         fn build(self) -> Result<Self::Parse, Error> {
//             todo!()
//         }

//         fn child<B: Build>(mut self, child: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child,
//         {
//             let pair = child.parts();
//             self.scope.push(pair.0.into());
//             Builder {
//                 scope: self.scope,
//                 parse: self.parse.push(pair.1),
//             }
//         }
//     }

//     impl<P: Parse + Stack> Build for Builder<scope::Verb, P> {
//         type Scope = scope::Verb;
//         type Parse = P;
//         type Next<Q> = Builder<scope::Verb, P::Push<Q>>;

//         fn parts(self) -> (Self::Scope, Self::Parse) {
//             (self.scope, self.parse)
//         }

//         fn build(self) -> Result<Self::Parse, Error> {
//             todo!()
//         }

//         fn child<B: Build>(mut self, child: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child,
//         {
//             let pair = child.parts();
//             self.scope.push(pair.0.into());
//             Builder {
//                 scope: self.scope,
//                 parse: self.parse.push(pair.1),
//             }
//         }
//     }

//     impl<P: Parse> Build for Builder<scope::Option, P> {
//         type Scope = scope::Option;
//         type Parse = P;
//         type Next<Q> = Invalid<scope::Option, P>;

//         fn parts(self) -> (Self::Scope, Self::Parse) {
//             (self.scope, self.parse)
//         }

//         fn build(self) -> Result<Self::Parse, Error> {
//             todo!()
//         }

//         fn child<B: Build>(self, _: B) -> Self::Next<B::Parse>
//         where
//             Self::Scope: scope::Parent,
//             B::Scope: scope::Child,
//         {
//             Invalid(self.scope, self.parse)
//         }
//     }

//     fn karl() {
//         root()
//             .child(verb().child(group()).child(option::<u8>()))
//             .child(group().child(group()).child(verb()).child(option::<u8>()))
//             .child(option::<u8>());
//     }
// }
