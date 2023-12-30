pub fn to_pascal(source: &str) -> String {
    to_continuous(source, true)
}

pub fn to_pascal_in(source: &str, target: &mut String) {
    to_continuous_in(source, target, true)
}

pub fn to_camel(source: &str) -> String {
    to_continuous(source, false)
}

pub fn to_camel_in(source: &str, target: &mut String) {
    to_continuous_in(source, target, false)
}

pub fn to_snake(source: &str) -> String {
    to_separate(source, '_', true)
}

pub fn to_snake_in(source: &str, target: &mut String) {
    to_separate_in(source, target, '_', true)
}

pub fn to_kebab(source: &str) -> String {
    to_separate(source, '-', true)
}

pub fn to_kebab_in(source: &str, target: &mut String) {
    to_separate_in(source, target, '-', true)
}

pub fn to_screaming_snake(source: &str) -> String {
    to_separate(source, '_', false)
}

pub fn to_upper_snake_in(source: &str, target: &mut String) {
    to_separate_in(source, target, '_', false)
}

pub fn to_screaming_kebab(source: &str) -> String {
    to_separate(source, '-', false)
}

pub fn to_upper_kebab_in(source: &str, target: &mut String) {
    to_separate_in(source, target, '-', false)
}

fn to_continuous(source: &str, first: bool) -> String {
    let mut target = String::with_capacity(source.len());
    to_continuous_in(source, &mut target, first);
    target
}

fn to_continuous_in(source: &str, target: &mut String, mut first: bool) {
    let mut upper = first;
    let mut last = true;
    for letter in source.chars() {
        if letter.is_ascii_alphabetic() {
            if upper {
                target.push(letter.to_ascii_uppercase());
                upper = false;
            } else if last {
                target.push(letter.to_ascii_lowercase());
            } else {
                target.push(letter);
            }
            last = letter.is_ascii_uppercase();
            first = true;
        } else if matches!(letter, '_' | '-' | ' ' | '\n') {
            upper = first;
        } else {
            upper = first;
            target.push(letter);
        }
    }
}

fn to_separate(source: &str, separator: char, lower: bool) -> String {
    let mut target = String::with_capacity(source.len());
    to_separate_in(source, &mut target, separator, lower);
    target
}

fn to_separate_in(source: &str, target: &mut String, separator: char, lower: bool) {
    let mut separate = false;
    let mut first = false;
    let mut last = false;
    for letter in source.chars() {
        if letter.is_ascii_uppercase() {
            if separate || last {
                target.push(separator);
                separate = false;
                last = false;
            }
            first = true;
            target.push(if lower {
                letter.to_ascii_lowercase()
            } else {
                letter
            });
        } else if letter.is_ascii_lowercase() {
            if separate {
                target.push(separator);
                separate = false;
            }
            first = true;
            last = true;
            target.push(if lower {
                letter
            } else {
                letter.to_ascii_uppercase()
            });
        } else if matches!(letter, '_' | '-' | ' ' | '\n') {
            separate = first;
            last = false;
        } else {
            target.push(letter);
            separate = false;
            first = false;
            last = false;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn to_pascal() {
        let convert = super::to_pascal;
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
    }

    #[test]
    fn to_camel() {
        let convert = super::to_camel;
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
    }

    #[test]
    fn to_snake() {
        let convert = super::to_snake;
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
    }

    #[test]
    fn to_kebab() {
        let convert = super::to_kebab;
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
    }

    #[test]
    fn to_screaming_snake() {
        let convert: fn(&str) -> String = super::to_screaming_snake;
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
    }

    #[test]
    fn to_screaming_kebab() {
        let convert: fn(&str) -> String = super::to_screaming_kebab;
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
    }
}
