use std::fmt::{self, Write};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Case {
    #[default]
    Same,
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    Kebab,
    UpperSnake,
    UpperKebab,
}

impl Case {
    #[inline]
    pub fn convert(&self, source: &str) -> String {
        let mut target = String::with_capacity(source.len());
        match self.convert_in(source, &mut target) {
            Ok(_) => target,
            Err(_) => unreachable!(),
        }
    }

    #[inline]
    pub fn convert_in<W: Write>(&self, source: &str, mut target: W) -> Result<(), fmt::Error> {
        match self {
            Case::Same => target.write_str(source),
            Case::Lower => Self::lower_in(source, target),
            Case::Upper => Self::upper_in(source, target),
            Case::Pascal => Self::pascal_in(source, target),
            Case::Camel => Self::camel_in(source, target),
            Case::Snake => Self::snake_in(source, target),
            Case::Kebab => Self::kebab_in(source, target),
            Case::UpperSnake => Self::upper_snake_in(source, target),
            Case::UpperKebab => Self::upper_kebab_in(source, target),
        }
    }

    #[inline]
    pub fn upper(source: &str) -> String {
        let mut target = String::with_capacity(source.len());
        match Self::upper_in(source, &mut target) {
            Ok(_) => target,
            Err(_) => unreachable!(),
        }
    }

    #[inline]
    pub fn upper_in<W: Write>(source: &str, mut target: W) -> Result<(), fmt::Error> {
        for letter in source.chars() {
            if !is_separator(letter) {
                target.write_char(letter.to_ascii_uppercase())?
            }
        }
        Ok(())
    }

    #[inline]
    pub fn lower(source: &str) -> String {
        let mut target = String::with_capacity(source.len());
        match Self::lower_in(source, &mut target) {
            Ok(_) => target,
            Err(_) => unreachable!(),
        }
    }

    #[inline]
    pub fn lower_in<W: Write>(source: &str, mut target: W) -> Result<(), fmt::Error> {
        for letter in source.chars() {
            if !is_separator(letter) {
                target.write_char(letter.to_ascii_lowercase())?
            }
        }
        Ok(())
    }

    #[inline]
    pub fn pascal(source: &str) -> String {
        continuous(source, true)
    }

    #[inline]
    pub fn pascal_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        continuous_in(source, target, true)
    }

    #[inline]
    pub fn camel(source: &str) -> String {
        continuous(source, false)
    }

    #[inline]
    pub fn camel_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        continuous_in(source, target, false)
    }

    #[inline]
    pub fn snake(source: &str) -> String {
        separate(source, '_', true)
    }

    #[inline]
    pub fn snake_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        separate_in(source, target, '_', true)
    }

    #[inline]
    pub fn kebab(source: &str) -> String {
        separate(source, '-', true)
    }

    #[inline]
    pub fn kebab_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        separate_in(source, target, '-', true)
    }

    #[inline]
    pub fn upper_snake(source: &str) -> String {
        separate(source, '_', false)
    }

    #[inline]
    pub fn upper_snake_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        separate_in(source, target, '_', false)
    }

    #[inline]
    pub fn upper_kebab(source: &str) -> String {
        separate(source, '-', false)
    }

    #[inline]
    pub fn upper_kebab_in<W: Write>(source: &str, target: W) -> Result<(), fmt::Error> {
        separate_in(source, target, '-', false)
    }
}

#[inline]
const fn is_separator(letter: char) -> bool {
    matches!(letter, '_' | '-' | ' ' | '\n')
}

#[inline]
fn continuous(source: &str, first: bool) -> String {
    let mut target = String::with_capacity(source.len());
    match continuous_in(source, &mut target, first) {
        Ok(_) => target,
        Err(_) => unreachable!(),
    }
}

fn continuous_in<W: Write>(
    source: &str,
    mut target: W,
    mut first: bool,
) -> Result<(), fmt::Error> {
    let mut upper = first;
    let mut last = true;
    for letter in source.chars() {
        if letter.is_ascii_alphabetic() {
            if upper {
                target.write_char(letter.to_ascii_uppercase())?;
                upper = false;
            } else if last {
                target.write_char(letter.to_ascii_lowercase())?;
            } else {
                target.write_char(letter)?;
            }
            last = letter.is_ascii_uppercase();
            first = true;
        } else if is_separator(letter) {
            upper = first;
        } else {
            upper = first;
            target.write_char(letter)?;
        }
    }
    Ok(())
}

#[inline]
fn separate(source: &str, separator: char, lower: bool) -> String {
    let mut target = String::with_capacity(source.len());
    match separate_in(source, &mut target, separator, lower) {
        Ok(_) => target,
        Err(_) => unreachable!(),
    }
}

fn separate_in<W: Write>(
    source: &str,
    mut target: W,
    separator: char,
    lower: bool,
) -> Result<(), fmt::Error> {
    let mut separate = false;
    let mut first = false;
    let mut last = false;
    for letter in source.chars() {
        if letter.is_ascii_uppercase() {
            if separate || last {
                target.write_char(separator)?;
                separate = false;
                last = false;
            }
            first = true;
            target.write_char(if lower {
                letter.to_ascii_lowercase()
            } else {
                letter
            })?;
        } else if letter.is_ascii_lowercase() {
            if separate {
                target.write_char(separator)?;
                separate = false;
            }
            first = true;
            last = true;
            target.write_char(if lower {
                letter
            } else {
                letter.to_ascii_uppercase()
            })?;
        } else if is_separator(letter) {
            separate = first;
            last = false;
        } else {
            target.write_char(letter)?;
            separate = false;
            first = false;
            last = false;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal() -> Result<(), fmt::Error> {
        let convert = Case::pascal;
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
        let convert = Case::camel;
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
        let convert = Case::snake;
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
        let convert = Case::kebab;
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
        let convert = Case::upper;
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
        let convert = Case::upper_snake;
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
        let convert = Case::upper_kebab;
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
