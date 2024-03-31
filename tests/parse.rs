use checkito::*;
use scalp::{Case, Error, Parser};
use std::{error, result, str::FromStr};

type Result = result::Result<(), Box<dyn error::Error>>;
const COUNT: usize = 1000;

#[test]
fn empty_parser_builds() -> Result {
    Parser::builder().build()?;
    Ok(())
}

#[test]
fn empty_parser_with_name_builds() -> Result {
    String::generator().check(COUNT, |name| {
        Parser::builder().name(name.clone()).build().is_ok()
    })?;
    Ok(())
}

#[test]
fn empty_parser_with_help_builds() -> Result {
    String::generator().check(COUNT, |name| {
        Parser::builder().help(name.clone()).build().is_ok()
    })?;
    Ok(())
}

#[test]
fn missing_option_value_with_short() -> Result {
    (regex!("[~±@£¢¤¬¦²³¼½¾`^¯_]+"), regex!("[a-zA-Z]")).check(COUNT, |(short, name)| {
        let parser = Parser::builder()
            .case(Case::Same)
            .prefix(short.clone(), "--")
            .option::<usize, _>(|option| option.name(name.clone()))
            .build()
            .unwrap();
        let argument = format!("{short}{name}");
        let error = parser.parse_with([argument.clone()], [("", "")]).unwrap_err();
        prove!(matches!(error, Error::MissingOptionValue(type_name, path) if type_name == Some("natural number".into()) && path == vec![argument.into()]))
    })?;
    Ok(())
}

#[test]
fn missing_option_value_with_long() -> Result {
    (regex!("[~±@£¢¤¬¦²³¼½¾`^¯_]+"), regex!("[a-zA-Z]{2,}")).check(COUNT, |(long, name)| {
        let parser = Parser::builder()
            .case(Case::Same)
            .prefix("-", long.clone())
            .option::<isize, _>(|option| option.name(name.clone()))
            .build()
            .unwrap();
        let argument = format!("{long}{name}");
        let error = parser.parse_with([argument.clone()], [("", "")]).unwrap_err();
        prove!(matches!(error, Error::MissingOptionValue(type_name, path) if type_name == Some("integer number".into()) && path == vec![argument.into()]))
    })?;
    Ok(())
}

#[test]
fn fails_to_parse_invalid_value() -> Result {
    (regex!("[a-zA-Z]{2,}"), ..-1).check(COUNT, |(name, value)| {
        let parser = Parser::builder()
            .case(Case::Same)
            .option::<usize, _>(|option| option.name(name.clone()))
            .build()
            .unwrap();
        let arguments = (format!("--{name}"), format!("{value}"));
        let error = parser
            .parse_with([arguments.0.clone(), arguments.1.clone()], [("", "")])
            .unwrap_err();
        prove!(matches!(error, Error::FailedToParseOptionValue(value, type_name, path) if value == arguments.1 && type_name == Some("natural number".into()) && path == vec!(arguments.0.into())))
    })?;
    Ok(())
}

#[test]
fn verb_with_no_option_allows_for_root_options_before_and_after() -> Result {
    (
        regex!("[a-z]{2,}")
            .array::<3>()
            .filter(|[a, b, c]| a != b && a != c && b != c),
        u8::generator(),
        u8::generator(),
    )
        .check(COUNT, |(values, v, u)| {
            let Some([a, b, c]) = values else {
                return Ok(true);
            };

            let (v, u) = (*v, *u);
            let parser = Parser::builder()
                .case(Case::Same)
                .option(|option| option.name(a.clone()).default(v))
                .option(|option| option.name(b.clone()).default(u))
                .verb(|verb| verb.name(c.clone()))
                .build()
                .unwrap();
            let result = parser.parse_with(
                [
                    format!("--{a}"),
                    format!("{v}"),
                    c.clone(),
                    format!("--{b}"),
                    format!("{u}"),
                ],
                [("", "")],
            );
            prove!(result == Ok((v, u, Some(()))))
        })?;
    Ok(())
}

#[test]
fn boolean_option_swizzling() -> Result {
    let parser = Parser::builder()
        .option(|option| option.name("a").swizzle().default(false))
        .option(|option| option.name("b").swizzle().default(false))
        .option(|option| option.name("c").swizzle().default(false))
        .build()?;
    assert_eq!(parser.parse_with(["-a"], [("", "")])?, (true, false, false));
    assert_eq!(parser.parse_with(["-ab"], [("", "")])?, (true, true, false));
    assert_eq!(parser.parse_with(["-abc"], [("", "")])?, (true, true, true));
    assert_eq!(parser.parse_with(["-ca"], [("", "")])?, (true, false, true));
    assert_eq!(parser.parse_with(["-bca"], [("", "")])?, (true, true, true));
    Ok(())
}

#[test]
fn invalid_swizzling() -> Result {
    let parser = Parser::builder()
        .option(|option| option.name("a").swizzle().default(false))
        .option(|option| option.name("b").default(false))
        .build()?;
    assert_eq!(parser.parse_with(["-a"], [("", "")]), Ok((true, false)));
    assert_eq!(parser.parse_with(["-b"], [("", "")]), Ok((false, true)));
    assert_eq!(
        parser.parse_with(["-ab"], [("", "")]),
        Err(Error::InvalidSwizzleOption('b'))
    );
    assert_eq!(
        parser.parse_with(["-ba"], [("", "")]),
        Err(Error::InvalidSwizzleOption('b'))
    );
    Ok(())
}

#[test]
fn parses_enum_value() -> Result {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, PartialEq)]
    enum Casing {
        Same,
        camelCase,
        PascalCase,
        snake_case,
    }

    impl FromStr for Casing {
        type Err = &'static str;

        fn from_str(s: &str) -> result::Result<Self, Self::Err> {
            match s {
                "same" => Ok(Casing::Same),
                "c" | "camel-case" => Ok(Casing::camelCase),
                "p" | "pascal-case" => Ok(Casing::PascalCase),
                "s" | "snake-case" => Ok(Casing::snake_case),
                _ => Err("Failed to parse."),
            }
        }
    }

    let parser = Parser::builder()
        .option::<Casing, _>(|option| {
            option
                .name("c")
                .valid("c(amel-case)?")
                .valid("p(ascal-case)?")
                .valid("s(nake-case)?")
                .default(Casing::Same)
        })
        .map(|(case,)| case)
        .build()?;
    assert_eq!(
        parser.parse_with(["-c", "camel-case"], [("", "")]),
        Ok(Casing::camelCase)
    );
    assert_eq!(
        parser.parse_with(["-c", "c"], [("", "")]),
        Ok(Casing::camelCase)
    );
    assert_eq!(
        parser.parse_with(["-c", "pascal-case"], [("", "")]),
        Ok(Casing::PascalCase)
    );
    assert_eq!(
        parser.parse_with(["-c", "p"], [("", "")]),
        Ok(Casing::PascalCase)
    );
    assert_eq!(
        parser.parse_with(["-c", "snake-case"], [("", "")]),
        Ok(Casing::snake_case)
    );
    assert_eq!(
        parser.parse_with(["-c", "s"], [("", "")]),
        Ok(Casing::snake_case)
    );
    assert_eq!(
        parser.parse_with(["-c", "same"], [("", "")]),
        Err(Error::InvalidOptionValue(
            "same".into(),
            ["c(amel-case)?", "p(ascal-case)?", "s(nake-case)?"]
                .map(ToString::to_string)
                .to_vec(),
            vec!["-c".into()]
        ))
    );
    Ok(())
}
