#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Case {
    #[default]
    Same,
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake {
        upper: bool,
    },
    Kebab {
        upper: bool,
    },
    Separate {
        separator: char,
        upper: bool,
    },
}

impl Case {
    #[inline]
    pub fn convert<'a>(&self, source: &'a str) -> impl Iterator<Item = char> + 'a {
        use orn::or8::*;
        match self {
            Case::Same => Iterator::T0(source.chars()),
            Case::Lower => Iterator::T1(Self::lower(source)),
            Case::Upper => Iterator::T2(Self::upper(source)),
            Case::Pascal => Iterator::T3(Self::pascal(source)),
            Case::Camel => Iterator::T4(Self::camel(source)),
            Case::Snake { upper } => Iterator::T5(Self::snake(source, *upper)),
            Case::Kebab { upper } => Iterator::T6(Self::kebab(source, *upper)),
            Case::Separate { separator, upper } => {
                Iterator::T7(separated(source, *separator, !*upper))
            }
        }
        .map(Or::into)
    }

    #[inline]
    pub fn upper(source: &str) -> impl Iterator<Item = char> + '_ {
        source.chars().filter_map(|letter| {
            if is_separator(letter) {
                None
            } else {
                Some(letter.to_ascii_uppercase())
            }
        })
    }

    #[inline]
    pub fn lower(source: &str) -> impl Iterator<Item = char> + '_ {
        source.chars().filter_map(|letter| {
            if is_separator(letter) {
                None
            } else {
                Some(letter.to_ascii_lowercase())
            }
        })
    }

    #[inline]
    pub fn pascal(source: &str) -> impl Iterator<Item = char> + '_ {
        continuous(source, true)
    }

    #[inline]
    pub fn camel(source: &str) -> impl Iterator<Item = char> + '_ {
        continuous(source, false)
    }

    #[inline]
    pub fn snake(source: &str, upper: bool) -> impl Iterator<Item = char> + '_ {
        separated(source, '_', !upper)
    }

    #[inline]
    pub fn kebab(source: &str, upper: bool) -> impl Iterator<Item = char> + '_ {
        separated(source, '-', !upper)
    }
}

#[inline]
const fn is_separator(letter: char) -> bool {
    matches!(letter, '_' | '-' | ' ' | '\n')
}

#[inline]
fn continuous(source: &str, mut first: bool) -> impl Iterator<Item = char> + '_ {
    let mut upper = first;
    let mut last = true;
    source.chars().flat_map(move |letter| {
        let mut result = None;
        if letter.is_ascii_alphabetic() {
            if upper {
                result = Some(letter.to_ascii_uppercase());
                upper = false;
            } else if last {
                result = Some(letter.to_ascii_lowercase());
            } else {
                result = Some(letter);
            }
            last = letter.is_ascii_uppercase();
            first = true;
        } else if is_separator(letter) {
            upper = first;
        } else {
            upper = first;
            result = Some(letter);
        }
        result
    })
}

#[inline]
fn separated(source: &str, separator: char, lower: bool) -> impl Iterator<Item = char> + '_ {
    let mut separate = false;
    let mut first = false;
    let mut last = false;
    source.chars().flat_map(move |letter| {
        let mut results = [None, None];
        if letter.is_ascii_uppercase() {
            if separate || last {
                results[0] = Some(separator);
                separate = false;
                last = false;
            }
            first = true;
            results[1] = Some(if lower {
                letter.to_ascii_lowercase()
            } else {
                letter
            });
        } else if letter.is_ascii_lowercase() {
            if separate {
                results[0] = Some(separator);
                separate = false;
            }
            first = true;
            last = true;
            results[1] = Some(if lower {
                letter
            } else {
                letter.to_ascii_uppercase()
            });
        } else if is_separator(letter) {
            separate = first;
            last = false;
        } else {
            results[0] = Some(letter);
            separate = false;
            first = false;
            last = false;
        }
        results.into_iter().flatten()
    })
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;

    #[test]
    fn pascal() -> Result<(), fmt::Error> {
        let convert = |value| Case::pascal(value).collect::<String>();
        assert_eq!(convert("BobaFett"), "BobaFett");
        assert_eq!(convert("bobaFett"), "BobaFett");
        assert_eq!(convert("boba fett"), "BobaFett");
        assert_eq!(convert("BOBA-FETT"), "BobaFett");
        assert_eq!(convert("BOBA"), "Boba");
        assert_eq!(convert("BOBA_FETT"), "BobaFett");
        assert_eq!(convert("boba-fett"), "BobaFett");
        assert_eq!(convert("_Boba_"), "Boba");
        assert_eq!(convert("_boba_"), "Boba");
        assert_eq!(convert("-Boba-"), "Boba");
        assert_eq!(convert("-boba-"), "Boba");
        assert_eq!(convert("1boba2fett"), "1Boba2Fett");
        assert_eq!(convert("1boBa2FetT"), "1BoBa2FetT");
        assert_eq!(convert("BObaFeTT"), "BobaFeTt");
        Ok(())
    }

    #[test]
    fn camel() -> Result<(), fmt::Error> {
        let convert = |value| Case::camel(value).collect::<String>();
        assert_eq!(convert("BobaFett"), "bobaFett");
        assert_eq!(convert("bobaFett"), "bobaFett");
        assert_eq!(convert("boba fett"), "bobaFett");
        assert_eq!(convert("BOBA-FETT"), "bobaFett");
        assert_eq!(convert("BOBA"), "boba");
        assert_eq!(convert("BOBA_FETT"), "bobaFett");
        assert_eq!(convert("boba-fett"), "bobaFett");
        assert_eq!(convert("_Boba_"), "boba");
        assert_eq!(convert("_boba_"), "boba");
        assert_eq!(convert("-Boba-"), "boba");
        assert_eq!(convert("-boba-"), "boba");
        assert_eq!(convert("1boba2fett"), "1boba2Fett");
        assert_eq!(convert("1boBa2FetT"), "1boBa2FetT");
        assert_eq!(convert("BObaFeTT"), "bobaFeTt");
        Ok(())
    }

    #[test]
    fn snake() -> Result<(), fmt::Error> {
        let convert = |value| Case::snake(value, false).collect::<String>();
        assert_eq!(convert("BobaFett"), "boba_fett");
        assert_eq!(convert("bobaFett"), "boba_fett");
        assert_eq!(convert("boba fett"), "boba_fett");
        assert_eq!(convert("BOBA-FETT"), "boba_fett");
        assert_eq!(convert("BOBA"), "boba");
        assert_eq!(convert("BOBA_FETT"), "boba_fett");
        assert_eq!(convert("boba-fett"), "boba_fett");
        assert_eq!(convert("_Boba_"), "boba");
        assert_eq!(convert("_boba_"), "boba");
        assert_eq!(convert("-Boba-"), "boba");
        assert_eq!(convert("-boba-"), "boba");
        assert_eq!(convert("1boba2fett"), "1boba2fett");
        assert_eq!(convert("1boBa2FetT"), "1bo_ba2fet_t");
        assert_eq!(convert("BObaFeTT"), "boba_fe_tt");
        Ok(())
    }

    #[test]
    fn kebab() -> Result<(), fmt::Error> {
        let convert = |value| Case::kebab(value, false).collect::<String>();
        assert_eq!(convert("BobaFett"), "boba-fett");
        assert_eq!(convert("bobaFett"), "boba-fett");
        assert_eq!(convert("boba fett"), "boba-fett");
        assert_eq!(convert("BOBA-FETT"), "boba-fett");
        assert_eq!(convert("BOBA"), "boba");
        assert_eq!(convert("BOBA_FETT"), "boba-fett");
        assert_eq!(convert("boba-fett"), "boba-fett");
        assert_eq!(convert("_Boba_"), "boba");
        assert_eq!(convert("_boba_"), "boba");
        assert_eq!(convert("-Boba-"), "boba");
        assert_eq!(convert("-boba-"), "boba");
        assert_eq!(convert("1boba2fett"), "1boba2fett");
        assert_eq!(convert("1boBa2FetT"), "1bo-ba2fet-t");
        assert_eq!(convert("BObaFeTT"), "boba-fe-tt");
        Ok(())
    }

    #[test]
    fn upper() -> Result<(), fmt::Error> {
        let convert = |value| Case::upper(value).collect::<String>();
        assert_eq!(convert("BobaFett"), "BOBAFETT");
        assert_eq!(convert("bobaFett"), "BOBAFETT");
        assert_eq!(convert("boba fett"), "BOBAFETT");
        assert_eq!(convert("BOBA-FETT"), "BOBAFETT");
        assert_eq!(convert("BOBA"), "BOBA");
        assert_eq!(convert("BOBA_FETT"), "BOBAFETT");
        assert_eq!(convert("boba-fett"), "BOBAFETT");
        assert_eq!(convert("_Boba_"), "BOBA");
        assert_eq!(convert("_boba_"), "BOBA");
        assert_eq!(convert("-Boba-"), "BOBA");
        assert_eq!(convert("-boba-"), "BOBA");
        assert_eq!(convert("1boba2fett"), "1BOBA2FETT");
        assert_eq!(convert("1boBa2FetT"), "1BOBA2FETT");
        assert_eq!(convert("BObaFeTT"), "BOBAFETT");
        Ok(())
    }

    #[test]
    fn upper_snake() -> Result<(), fmt::Error> {
        let convert = |value| Case::snake(value, true).collect::<String>();
        assert_eq!(convert("BobaFett"), "BOBA_FETT");
        assert_eq!(convert("bobaFett"), "BOBA_FETT");
        assert_eq!(convert("boba fett"), "BOBA_FETT");
        assert_eq!(convert("BOBA-FETT"), "BOBA_FETT");
        assert_eq!(convert("BOBA"), "BOBA");
        assert_eq!(convert("BOBA_FETT"), "BOBA_FETT");
        assert_eq!(convert("boba-fett"), "BOBA_FETT");
        assert_eq!(convert("_Boba_"), "BOBA");
        assert_eq!(convert("_boba_"), "BOBA");
        assert_eq!(convert("-Boba-"), "BOBA");
        assert_eq!(convert("-boba-"), "BOBA");
        assert_eq!(convert("1boba2fett"), "1BOBA2FETT");
        assert_eq!(convert("1boBa2FetT"), "1BO_BA2FET_T");
        assert_eq!(convert("BObaFeTT"), "BOBA_FE_TT");
        Ok(())
    }

    #[test]
    fn upper_kebab() -> Result<(), fmt::Error> {
        let convert = |value| Case::kebab(value, true).collect::<String>();
        assert_eq!(convert("BobaFett"), "BOBA-FETT");
        assert_eq!(convert("bobaFett"), "BOBA-FETT");
        assert_eq!(convert("boba fett"), "BOBA-FETT");
        assert_eq!(convert("BOBA-FETT"), "BOBA-FETT");
        assert_eq!(convert("BOBA"), "BOBA");
        assert_eq!(convert("BOBA_FETT"), "BOBA-FETT");
        assert_eq!(convert("boba-fett"), "BOBA-FETT");
        assert_eq!(convert("_Boba_"), "BOBA");
        assert_eq!(convert("_boba_"), "BOBA");
        assert_eq!(convert("-Boba-"), "BOBA");
        assert_eq!(convert("-boba-"), "BOBA");
        assert_eq!(convert("1boba2fett"), "1BOBA2FETT");
        assert_eq!(convert("1boBa2FetT"), "1BO-BA2FET-T");
        assert_eq!(convert("BObaFeTT"), "BOBA-FE-TT");
        Ok(())
    }
}
