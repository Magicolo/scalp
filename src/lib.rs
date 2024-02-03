mod build;
mod case;
mod error;
mod help;
pub mod meta;
mod parse;
pub mod scope;
mod spell;
mod stack;

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
    - Offset long names by the short name length if there are any short names.
    - Column for types before summary.
    - Specify license.
    - Could I use the same 'Parse' trait to generate an API?
    - Generate usage string automatically.
        - Usage: {verb (for root use the root name)} [position options (if any)] [named options (if any)] {sub-command (if any)}
    - Support for styled formatting out of the box; use a feature?
    - Parse with graceful handling of 'Error::Help' and 'Error::Version'.
    - Support for indexed arguments.
    - Ensure that variables don't obscure the context variable.
    - Support for streamed arguments via stdin, file system, http.
    - Ability to overwrite the type name.
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

const HELP: usize = usize::MAX;
const VERSION: usize = usize::MAX - 1;
const LICENSE: usize = usize::MAX - 2;
const AUTHOR: usize = usize::MAX - 3;
const BREAK: usize = usize::MAX - 4;

const SHIFT: u32 = 5;
const MASK: usize = (1 << SHIFT) - 1;
const MAXIMUM: u32 = usize::BITS - 14;

#[macro_export]
macro_rules! cargo {
    () => {
        |builder| {
            builder
                .name(env!("CARGO_BIN_NAME").trim())
                .version(env!("CARGO_PKG_VERSION").trim())
                .license(
                    env!("CARGO_PKG_LICENSE").trim(),
                    env!("CARGO_PKG_LICENSE_FILE").trim(),
                )
                .pipe(|builder| {
                    env!("CARGO_PKG_AUTHORS")
                        .split(':')
                        .fold(builder, |builder, author| builder.author(author.trim()))
                })
                .help("")
                .help(env!("CARGO_PKG_DESCRIPTION"))
                .help("")
                .note(env!("CARGO_PKG_HOMEPAGE"))
                .note(env!("CARGO_PKG_REPOSITORY"))
                .help("")
        }
    };
}
