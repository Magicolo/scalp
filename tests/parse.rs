use scalp::{Builder, Error};

#[test]
fn verb_with_no_option_allows_for_root_options_before_and_after() -> Result<(), Error> {
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
fn boolean_option_swizzling() -> Result<(), Error> {
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
