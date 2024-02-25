pub mod build;
mod case;
mod error;
mod help;
pub mod meta;
pub mod parse;
pub mod scope;
mod spell;
mod stack;
pub mod style;

pub use crate::{
    build::Builder,
    case::Case,
    error::Error,
    meta::Options,
    parse::{Parse, Parser},
    scope::Scope,
};

/*
    TODO:
    - Favor `Deserialize` over `FromStr`.
        - Define a 'MetaDeserializer' which will be used to collect meta data from a type `T: Deserialize` (including variant names and more).
        - For enums, build a map between case-converted keys and variant names.
    - Generate usage string automatically.
        - Usage: {verb (for root use the root name)} [position options (if any)] [named options (if any)] {sub-command (if any)}
    - Ensure that variables don't obscure the context variable.
    - Support for streamed arguments via stdin, file system, http.
    - Support for a value with --help
        - Allows to provide a help context when help becomes very large (ex: --help branch)
    - Autocomplete?
    - Simplify the 'Into<Cow<'static, str>>' all over the place, if possible.
        - There are probably some places where the `Cow` isn't useful.
    - Can I unify 'Builder' and 'Parser'?
    - Allow to rename '--help' and '--version'?
    - Support for json values.
*/

const HELP: usize = usize::MAX;
const VERSION: usize = usize::MAX - 1;
const LICENSE: usize = usize::MAX - 2;
const AUTHOR: usize = usize::MAX - 3;
const BREAK: usize = usize::MAX - 4;

const SHIFT: u32 = 5;
const MASK: usize = (1 << SHIFT) - 1;
const MAXIMUM: u32 = usize::BITS - 14;

#[macro_export]
macro_rules! header {
    () => {
        |builder: $crate::Builder<$crate::scope::Root, _>| $crate::header!(builder)
    };
    ($builder: expr) => {
        $builder.pipe(|mut builder| {
            builder = builder.name(env!("CARGO_BIN_NAME").trim());
            builder = builder.version(env!("CARGO_PKG_VERSION").trim());
            builder = builder.license(
                env!("CARGO_PKG_LICENSE").trim(),
                env!("CARGO_PKG_LICENSE_FILE").trim(),
            );
            builder = env!("CARGO_PKG_AUTHORS")
                .trim()
                .split(':')
                .fold(builder, |builder, author| builder.author(author.trim()));
            builder = builder.summary(env!("CARGO_PKG_DESCRIPTION").trim());
            builder = builder.home(env!("CARGO_PKG_HOMEPAGE").trim());
            builder = builder.repository(env!("CARGO_PKG_REPOSITORY").trim());
            builder.line()
        })
    };
}
