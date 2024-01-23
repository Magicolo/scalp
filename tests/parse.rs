use anyhow::Result;
use scalp::Builder;

#[test]
fn verb_with_no_option_allows_for_root_options_before_and_after() -> Result<()> {
    let result = Builder::new()
        .root(|build| {
            build
                .option(|build| build.name("a").default(1))
                .option(|build| build.name("b").default(1))
                .verb(|build| build.name("c"))
        })
        .build()?
        .parse_with(["-a", "1", "c", "-b", "2"], [("", "")])?;
    assert_eq!(result, Some((1, 2, Some(()))));
    Ok(())
}

#[test]
fn boolean_option_swizzling() -> Result<()> {
    let parser = Builder::new()
        .root(|build| {
            build
                .option(|build| build.name("a").default(false))
                .option(|build| build.name("b").default(false))
                .option(|build| build.name("c").default(false))
        })
        .build()?;
    assert_eq!(
        parser.parse_with(["-a"], [("", "")])?,
        Some((true, false, false))
    );
    assert_eq!(
        parser.parse_with(["-ab"], [("", "")])?,
        Some((true, true, false))
    );
    assert_eq!(
        parser.parse_with(["-abc"], [("", "")])?,
        Some((true, true, true))
    );
    assert_eq!(
        parser.parse_with(["-ca"], [("", "")])?,
        Some((true, false, true))
    );
    assert_eq!(
        parser.parse_with(["-bca"], [("", "")])?,
        Some((true, true, true))
    );
    Ok(())
}
