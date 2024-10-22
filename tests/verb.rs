pub mod common;
use common::*;

#[check]
fn empty_parses() {
    verb().parse_with([""], [("", "")]).unwrap();
}

#[check(regex!("[0-9a-zA-Z]+"))]
fn alpha_numeric_name_is_valid(name: String) {
    verb().name(name).unwrap();
}

#[check(_)]
#[should_panic]
fn other_name_is_invalid(name: String) {
    verb().name(name).unwrap();
}

#[check(_)]
fn help_is_valid(help: String) {
    verb().help(help);
}

#[check(_)]
fn prefix_is_valid(prefix: char) {
    verb().prefix(prefix);
}

#[check(case())]
fn case_is_valid(case: Case) {
    verb().case(case);
}
