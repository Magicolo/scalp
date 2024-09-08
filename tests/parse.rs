use checkito::*;
use scalp::{Case, Error, Parser};
use std::{error, result, str::FromStr};

type Result = result::Result<(), Box<dyn error::Error>>;
const COUNT: usize = 1000;

fn prefix() -> impl Generate<Item = char> {
    (
        '~', '±', '@', '£', '¢', '¤', '¬', '¦', '_', '-', ',', '.', '+', '=', '^', '¯',
    )
        .any()
        .map(|value| value.into::<char>())
}

#[test]
fn empty_verb_parser_builds() -> Result {
    Parser::verb(|verb| verb)?;
    Ok(())
}

#[test]
fn empty_option_parser_builds() -> Result {
    Parser::option::<String>(|option| option)?;
    Ok(())
}

#[test]
fn empty_parser_with_name_builds() -> Result {
    regex!("[a-zA-Z]+").check(COUNT, |name| {
        Parser::verb(|verb| verb.name(name.clone())).is_ok()
    })?;
    Ok(())
}

#[test]
fn empty_parser_with_help_builds() -> Result {
    String::generator().check(COUNT, |name| {
        Parser::verb(|verb| verb.help(name.clone())).is_ok()
    })?;
    Ok(())
}

#[test]
fn missing_option_value_with_short() -> Result {
    (prefix(), regex!("[a-zA-Z]")).check(COUNT, |(prefix, name)| {
        let parser = Parser::verb(|verb| {
            verb.case(Case::Same)
                .prefix(*prefix)
                .option::<usize, _>(|option| option.name(name.clone()))
        })
        .unwrap();
        let Err(Error::MissingOptionValue(type_name, path)) =
            parser.parse_with([format!("{prefix}{name}")], [("", "")])
        else {
            panic!()
        };
        assert_eq!(type_name, Some("natural number".into()));
        assert_eq!(path, vec!(name.clone().into()));
        Ok::<(), ()>(())
    })?;
    Ok(())
}

#[test]
fn missing_option_value_with_long() -> Result {
    (prefix(), regex!("[a-zA-Z]{2,}")).check(COUNT, |(prefix, name)| {
        let parser = Parser::verb(|verb| {
            verb.case(Case::Same)
                .prefix(*prefix)
                .option::<isize, _>(|option| option.name(name.clone()))
        })
        .unwrap();
        let Err(Error::MissingOptionValue(type_name, path)) =
            parser.parse_with([format!("{prefix}{prefix}{name}")], [("", "")])
        else {
            panic!()
        };
        assert_eq!(type_name, Some("integer number".into()));
        assert_eq!(path, vec!(name.clone().into()));
        Ok::<(), ()>(())
    })?;
    Ok(())
}

#[test]
fn fails_to_parse_invalid_value() -> Result {
    (regex!("[a-zA-Z]{2,}"), ..-1).check(COUNT, |(name, value)| {
        let parser = Parser::verb(|verb| {
            verb.case(Case::Same)
                .option::<usize, _>(|option| option.name(name.clone()))
        })
        .unwrap();
        let argument = format!("{value}");
        let Err(Error::FailedToParseOptionValue(value, type_name, path)) =
            parser.parse_with([format!("--{name}"), argument.clone()], [("", "")])
        else {
            panic!()
        };
        assert_eq!(value.as_str(), argument);
        assert_eq!(type_name, Some("natural number".into()));
        assert_eq!(path, vec!(name.clone().into()));
        Ok::<(), ()>(())
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
            let parser = Parser::verb(|root| {
                root.case(Case::Same)
                    .option(|option| option.name(a.clone()).default(v))
                    .option(|option| option.name(b.clone()).default(u))
                    .verb(|verb| verb.name(c.clone()))
            })
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
    let parser = Parser::verb(|verb| {
        verb.option(|option| option.name("a").swizzle().default(false))
            .option(|option| option.name("b").swizzle().default(false))
            .option(|option| option.name("c").swizzle().default(false))
    })?;
    assert_eq!(parser.parse_with(["-a"], [("", "")])?, (true, false, false));
    assert_eq!(parser.parse_with(["-ab"], [("", "")])?, (true, true, false));
    assert_eq!(parser.parse_with(["-abc"], [("", "")])?, (true, true, true));
    assert_eq!(parser.parse_with(["-ca"], [("", "")])?, (true, false, true));
    assert_eq!(parser.parse_with(["-bca"], [("", "")])?, (true, true, true));
    Ok(())
}

#[test]
fn invalid_swizzling() -> Result {
    let parser = Parser::verb(|verb| {
        verb.option(|option| option.name("a").swizzle().default(false))
            .option(|option| option.name("b").default(false))
    })?;
    assert_eq!(parser.parse_with(["-a"], [("", "")]), Ok((true, false)));
    assert_eq!(parser.parse_with(["-b"], [("", "")]), Ok((false, true)));
    assert_eq!(
        parser.parse_with(["-ab"], [("", "")]),
        Err(Error::InvalidSwizzleOption("b".into()))
    );
    assert_eq!(
        parser.parse_with(["-ba"], [("", "")]),
        Err(Error::InvalidSwizzleOption("b".into()))
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

    let parser = Parser::verb(|verb| {
        verb.option::<Casing, _>(|option| {
            option
                .name("c")
                .valid("c(amel-case)?")
                .valid("p(ascal-case)?")
                .valid("s(nake-case)?")
                .default(Casing::Same)
        })
        .map(|(case,)| case)
    })?;
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
            vec!["c".into()]
        ))
    );
    Ok(())
}

// #[test]
// fn boba() {
//     Builder::option()
//         .name("boba")
//         .parse::<String>()
//         .build();
//     Builder::verb()
//         .name("fett")
//         .option();
//     Builder::group()
//         .name("jango");
// }
