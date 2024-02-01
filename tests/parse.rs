use checkito::*;
use scalp::{Builder, Error};
use std::{error, result};

type Result = result::Result<(), Box<dyn error::Error>>;
const COUNT: usize = 100;

#[test]
fn empty_parser_builds() -> Result {
    Builder::new().build()?;
    Ok(())
}

#[test]
fn empty_parser_with_name_builds() -> Result {
    String::generator().check(COUNT, |name| Builder::new().name(name.clone()).build())?;
    Ok(())
}

#[test]
fn empty_parser_with_help_builds() -> Result {
    String::generator().check(COUNT, |name| Builder::new().help(name.clone()).build())?;
    Ok(())
}

#[test]
fn missing_option_value() -> Result {
    (("~", "_", "±", "@1", ";123").any().map(Unify::<&str>::unify), regex!("[a-zA-Z]")).check(COUNT, |(short, name)| {
        let parser = Builder::new()
            .short(*short)
            .option::<usize, _>(|option| option.name(name.clone()))
            .build()
            .unwrap();
        let argument = format!("{short}{name}");
        let error = parser.parse_with([argument.clone()], [("", "")]).unwrap_err();
        prove!(
            matches!(error, Error::MissingOptionValue(type_name, option) if type_name == Some("integer".into()) && option == Some(argument.into()))
        )
    })?;
    Ok(())
}

#[test]
fn fails_to_parse_invalid_value() -> Result {
    let parser = Builder::new()
        .option::<usize, _>(|option| option.name("a"))
        .build()?;
    let error = parser.parse_with(["-a", "-1"], [("", "")]).unwrap_err();
    assert!(
        matches!(error, Error::FailedToParseOptionValue(value, type_name, option) if value == "-1" && type_name == Some("integer".into()) && option == Some("-a".into()))
    );
    Ok(())
}

#[test]
fn verb_with_no_option_allows_for_root_options_before_and_after() -> Result {
    let parser = Builder::new()
        .option(|option| option.name("a").default(1))
        .option(|option| option.name("b").default(1))
        .verb(|verb| verb.name("c"))
        .build()?;
    let result = parser.parse_with(["-a", "1", "c", "-b", "2"], [("", "")])?;
    assert_eq!(result, (1, 2, Some(())));
    Ok(())
}

#[test]
fn boolean_option_swizzling() -> Result {
    let parser = Builder::new()
        .option(|option| option.name("a").default(false))
        .option(|option| option.name("b").default(false))
        .option(|option| option.name("c").default(false))
        .build()?;
    assert_eq!(parser.parse_with(["-a"], [("", "")])?, (true, false, false));
    assert_eq!(parser.parse_with(["-ab"], [("", "")])?, (true, true, false));
    assert_eq!(parser.parse_with(["-abc"], [("", "")])?, (true, true, true));
    assert_eq!(parser.parse_with(["-ca"], [("", "")])?, (true, false, true));
    assert_eq!(parser.parse_with(["-bca"], [("", "")])?, (true, true, true));
    Ok(())
}
