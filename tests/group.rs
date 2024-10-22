pub mod common;
use common::*;

#[check(_)]
fn name_is_valid(name: String) {
    group().name(name).unwrap();
}

#[check(_)]
fn help_is_valid(help: String) {
    group().help(help);
}
