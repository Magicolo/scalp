pub mod common;
use common::*;

#[check(regex!("[a-zA-Z]+"))]
fn string_option_parses(value: String) {
    let parsed = option::<String>()
        .parse_with([value.clone()], [("", "")])
        .unwrap()
        .unwrap();
    assert_eq!(value, parsed);
}
