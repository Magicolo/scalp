use core::fmt::{self, Write};

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
    Other(fn(&str, &mut dyn Write) -> Result<(), fmt::Error>),
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
            Case::Snake { upper } => Self::snake_in(source, target, *upper),
            Case::Kebab { upper } => Self::kebab_in(source, target, *upper),
            Case::Separate { separator, upper } => separate_in(source, target, *separator, !*upper),
            Case::Other(convert) => convert(source, &mut target),
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
    pub fn snake(source: &str, upper: bool) -> String {
        separate(source, '_', !upper)
    }

    #[inline]
    pub fn snake_in<W: Write>(source: &str, target: W, upper: bool) -> Result<(), fmt::Error> {
        separate_in(source, target, '_', !upper)
    }

    #[inline]
    pub fn kebab(source: &str, upper: bool) -> String {
        separate(source, '-', !upper)
    }

    #[inline]
    pub fn kebab_in<W: Write>(source: &str, target: W, upper: bool) -> Result<(), fmt::Error> {
        separate_in(source, target, '-', !upper)
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

fn continuous_in<W: Write>(source: &str, mut target: W, mut first: bool) -> Result<(), fmt::Error> {
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
        assert_eq!(Case::snake("BobaFett", false), "boba_fett");
        assert_eq!(Case::snake("bobaFett", false), "boba_fett");
        assert_eq!(Case::snake("boba fett", false), "boba_fett");
        assert_eq!(Case::snake("BOBA-FETT", false), "boba_fett");
        assert_eq!(Case::snake("BOBA", false), "boba");
        assert_eq!(Case::snake("BOBA_FETT", false), "boba_fett");
        assert_eq!(Case::snake("boba-fett", false), "boba_fett");
        assert_eq!(Case::snake("_Boba_", false), "boba");
        assert_eq!(Case::snake("_boba_", false), "boba");
        assert_eq!(Case::snake("-Boba-", false), "boba");
        assert_eq!(Case::snake("-boba-", false), "boba");
        assert_eq!(Case::snake("1boba2fett", false), "1boba2fett");
        assert_eq!(Case::snake("1boBa2FetT", false), "1bo_ba2fet_t");
        assert_eq!(Case::snake("BObaFeTT", false), "boba_fe_tt");
        Ok(())
    }

    #[test]
    fn kebab() -> Result<(), fmt::Error> {
        assert_eq!(Case::kebab("BobaFett", false), "boba-fett");
        assert_eq!(Case::kebab("bobaFett", false), "boba-fett");
        assert_eq!(Case::kebab("boba fett", false), "boba-fett");
        assert_eq!(Case::kebab("BOBA-FETT", false), "boba-fett");
        assert_eq!(Case::kebab("BOBA", false), "boba");
        assert_eq!(Case::kebab("BOBA_FETT", false), "boba-fett");
        assert_eq!(Case::kebab("boba-fett", false), "boba-fett");
        assert_eq!(Case::kebab("_Boba_", false), "boba");
        assert_eq!(Case::kebab("_boba_", false), "boba");
        assert_eq!(Case::kebab("-Boba-", false), "boba");
        assert_eq!(Case::kebab("-boba-", false), "boba");
        assert_eq!(Case::kebab("1boba2fett", false), "1boba2fett");
        assert_eq!(Case::kebab("1boBa2FetT", false), "1bo-ba2fet-t");
        assert_eq!(Case::kebab("BObaFeTT", false), "boba-fe-tt");
        Ok(())
    }

    #[test]
    fn upper() -> Result<(), fmt::Error> {
        assert_eq!(Case::upper("BobaFett"), "BOBAFETT");
        assert_eq!(Case::upper("bobaFett"), "BOBAFETT");
        assert_eq!(Case::upper("boba fett"), "BOBAFETT");
        assert_eq!(Case::upper("BOBA-FETT"), "BOBAFETT");
        assert_eq!(Case::upper("BOBA"), "BOBA");
        assert_eq!(Case::upper("BOBA_FETT"), "BOBAFETT");
        assert_eq!(Case::upper("boba-fett"), "BOBAFETT");
        assert_eq!(Case::upper("_Boba_"), "BOBA");
        assert_eq!(Case::upper("_boba_"), "BOBA");
        assert_eq!(Case::upper("-Boba-"), "BOBA");
        assert_eq!(Case::upper("-boba-"), "BOBA");
        assert_eq!(Case::upper("1boba2fett"), "1BOBA2FETT");
        assert_eq!(Case::upper("1boBa2FetT"), "1BOBA2FETT");
        assert_eq!(Case::upper("BObaFeTT"), "BOBAFETT");
        Ok(())
    }

    #[test]
    fn upper_snake() -> Result<(), fmt::Error> {
        assert_eq!(Case::snake("BobaFett", true), "BOBA_FETT");
        assert_eq!(Case::snake("bobaFett", true), "BOBA_FETT");
        assert_eq!(Case::snake("boba fett", true), "BOBA_FETT");
        assert_eq!(Case::snake("BOBA-FETT", true), "BOBA_FETT");
        assert_eq!(Case::snake("BOBA", true), "BOBA");
        assert_eq!(Case::snake("BOBA_FETT", true), "BOBA_FETT");
        assert_eq!(Case::snake("boba-fett", true), "BOBA_FETT");
        assert_eq!(Case::snake("_Boba_", true), "BOBA");
        assert_eq!(Case::snake("_boba_", true), "BOBA");
        assert_eq!(Case::snake("-Boba-", true), "BOBA");
        assert_eq!(Case::snake("-boba-", true), "BOBA");
        assert_eq!(Case::snake("1boba2fett", true), "1BOBA2FETT");
        assert_eq!(Case::snake("1boBa2FetT", true), "1BO_BA2FET_T");
        assert_eq!(Case::snake("BObaFeTT", true), "BOBA_FE_TT");
        Ok(())
    }

    #[test]
    fn upper_kebab() -> Result<(), fmt::Error> {
        assert_eq!(Case::kebab("BobaFett", true), "BOBA-FETT");
        assert_eq!(Case::kebab("bobaFett", true), "BOBA-FETT");
        assert_eq!(Case::kebab("boba fett", true), "BOBA-FETT");
        assert_eq!(Case::kebab("BOBA-FETT", true), "BOBA-FETT");
        assert_eq!(Case::kebab("BOBA", true), "BOBA");
        assert_eq!(Case::kebab("BOBA_FETT", true), "BOBA-FETT");
        assert_eq!(Case::kebab("boba-fett", true), "BOBA-FETT");
        assert_eq!(Case::kebab("_Boba_", true), "BOBA");
        assert_eq!(Case::kebab("_boba_", true), "BOBA");
        assert_eq!(Case::kebab("-Boba-", true), "BOBA");
        assert_eq!(Case::kebab("-boba-", true), "BOBA");
        assert_eq!(Case::kebab("1boba2fett", true), "1BOBA2FETT");
        assert_eq!(Case::kebab("1boBa2FetT", true), "1BO-BA2FET-T");
        assert_eq!(Case::kebab("BObaFeTT", true), "BOBA-FE-TT");
        Ok(())
    }
}
