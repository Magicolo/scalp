mod build;
mod case;
mod error;
mod help;
mod parse;
mod scope;
mod spell;
mod stack;

pub use crate::{build::Builder, error::Error, parse::Parser, scope::Scope};
use std::borrow::Cow;

/*
    TODO:
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
    Type(Cow<'static, str>),
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
    Version,
    Help,
}

impl Meta {
    pub fn clone(&self, depth: usize) -> Self {
        match self {
            Meta::Name(value) => Meta::Name(value.clone()),
            Meta::Position => Meta::Position,
            Meta::Version(value) => Meta::Version(value.clone()),
            Meta::Help(value) => Meta::Help(value.clone()),
            Meta::Type(value) => Meta::Type(value.clone()),
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
