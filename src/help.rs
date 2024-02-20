use crate::meta::{Meta, Name};
use core::{
    fmt::{self, Write},
    mem::{replace, take},
    slice::from_ref,
};
use std::{borrow::Cow, fmt::Display, fs, iter::from_fn, ops::Deref};
use termion::{
    color::Rgb,
    style::{Bold, Faint, Italic, Reset, Underline},
};

const INDENTATION: &str = "  ";
const INDENT: usize = INDENTATION.len();
const OCEAN_BLUE: Rgb = Rgb(36, 113, 163);
const TURQUOISE: Rgb = Rgb(64, 224, 208);
const RUBY_RED: Rgb = Rgb(220, 20, 60);
const SILVER_GRAY: Rgb = Rgb(169, 169, 169);
const MAUVE: Rgb = Rgb(224, 176, 255);
// const SUNSET_ORANGE: Rgb = Rgb(255, 140, 79);
// const EMERALD_GREEN: Rgb = Rgb(0, 158, 96);
// const AMETHYST_PURPLE: Rgb = Rgb(138, 43, 226);
// const GOLDENROD_YELLOW: Rgb = Rgb(218, 165, 32);
// const LIME_GREEN: Rgb = Rgb(50, 205, 50);
// const TEAL: Rgb = Rgb(0, 128, 128);
// const CORAL_PINK: Rgb = Rgb(255, 127, 80);
// const SAFFRON_YELLOW: Rgb = Rgb(244, 196, 48);
// const INDIGO: Rgb = Rgb(75, 0, 130);
// const AQUA: Rgb = Rgb(0, 255, 255);
// const VIOLET: Rgb = Rgb(238, 130, 238);
// const FOREST_GREEN: Rgb = Rgb(34, 139, 34);
// const PEACH: Rgb = Rgb(255, 218, 185);
// const STEEL_BLUE: Rgb = Rgb(70, 130, 180);
// const CHOCOLATE_BROWN: Rgb = Rgb(139, 69, 19);
// const CORNFLOWER_BLUE: Rgb = Rgb(100, 149, 237);
// const OLIVE_GREEN: Rgb = Rgb(128, 128, 0);

const ROOT: Rgb = RUBY_RED;
const GROUP: Rgb = OCEAN_BLUE;
const VERB: Rgb = TURQUOISE;
const OPTION: Rgb = TURQUOISE;
const USAGE: Rgb = MAUVE;
const NOTE: Rgb = SILVER_GRAY;

struct Helper<'a> {
    buffer: &'a mut String,
    indent: usize,
    width: usize,
}

#[derive(Default)]
struct Columns {
    short: usize,
    long: usize,
    types: usize,
}

impl<'a> Helper<'a> {
    fn space(&mut self, width: usize) -> Result<(), fmt::Error> {
        for _ in 0..width {
            write!(self.buffer, " ")?;
        }
        Ok(())
    }

    fn own(&mut self) -> Helper {
        Helper {
            buffer: self.buffer,
            indent: self.indent,
            width: self.width,
        }
    }

    fn indent(&mut self) -> Helper {
        self.indent_with(INDENT)
    }

    fn indent_with(&mut self, by: usize) -> Helper {
        let mut helper = self.own();
        helper.indent += by;
        helper
    }

    fn indentation(&mut self) -> Result<(), fmt::Error> {
        self.space(self.indent)
    }

    fn scope<T>(
        &mut self,
        scope: impl FnOnce(Helper) -> Result<T, fmt::Error>,
    ) -> Result<String, fmt::Error> {
        let buffer = take(self.buffer);
        scope(self.own())?;
        Ok(replace(self.buffer, buffer))
    }

    fn names(
        &mut self,
        metas: &[Meta],
        short: bool,
        long: bool,
        prefix: impl fmt::Display,
        position: &mut usize,
    ) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, ", ", |meta| match meta {
            Meta::Name(Name::Plain, value) => Some(Cow::Borrowed(value)),
            Meta::Name(Name::Short, value) if short => Some(Cow::Borrowed(value)),
            Meta::Name(Name::Long, value) if long => Some(Cow::Borrowed(value)),
            Meta::Position if short => {
                let value = format!("[{position}]");
                *position += 1;
                Some(Cow::Owned(value))
            }
            _ => None,
        })
    }

    fn types(&mut self, metas: &[Meta], prefix: impl fmt::Display) -> Result<usize, fmt::Error> {
        let mut last = None;
        for meta in visible(metas) {
            if let Meta::Type(value) = meta {
                last = Some(value);
            }
        }
        if let Some(value) = last {
            self.write(format_args!("{prefix}{value}"))
        } else {
            Ok(0)
        }
    }

    fn versions(&mut self, metas: &[Meta], prefix: impl fmt::Display) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, ", ", |meta| match meta {
            Meta::Version(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })
    }

    fn authors(&mut self, metas: &[Meta], prefix: impl fmt::Display) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, ", ", |meta| match meta {
            Meta::Author(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })
    }

    fn join(
        &mut self,
        metas: &[Meta],
        prefix: impl fmt::Display,
        separator: impl fmt::Display,
        mut find: impl FnMut(&Meta) -> Option<Cow<str>>,
    ) -> Result<usize, fmt::Error> {
        self.write_with(|helper| {
            let mut join = false;
            for meta in visible(metas) {
                if let Some(value) = find(meta) {
                    if replace(&mut join, true) {
                        write!(helper.buffer, "{separator}")?;
                    } else {
                        write!(helper.buffer, "{prefix}")?;
                    }
                    write!(helper.buffer, "{value}")?;
                }
            }
            Ok(())
        })
    }

    fn wrap(
        &mut self,
        value: &str,
        prefix: impl fmt::Display,
        suffix: impl fmt::Display,
        cursor: &mut usize,
        join: &mut bool,
    ) -> Result<(), fmt::Error> {
        for line in value.split('\n') {
            if replace(join, true) {
                writeln!(self.buffer)?;
                self.indentation()?;
            } else {
                *cursor += self.write(&prefix)?;
            }

            let mut join = false;
            for word in line.split(' ') {
                if replace(&mut join, true) {
                    write!(self.buffer, " ")?;
                }
                self.word(word, cursor)?;
            }
        }
        if *join {
            *cursor += self.write(suffix)?;
        }
        Ok(())
    }

    fn word(&mut self, word: &str, cursor: &mut usize) -> Result<(), fmt::Error> {
        if *cursor + word.len() > self.width - self.indent {
            writeln!(self.buffer)?;
            self.indentation()?;
            *cursor = 0;
        }
        write!(self.buffer, "{word}")?;
        *cursor += word.len();
        Ok(())
    }

    fn help(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        let mut join = false;
        let mut cursor = 0;
        for meta in visible(metas) {
            if let Meta::Help(value) = meta {
                if !value.chars().all(char::is_whitespace) {
                    self.wrap(value, "", "", &mut cursor, &mut join)?
                }
            }
        }
        Ok(())
    }

    fn columns(
        metas: &[Meta],
        depth: usize,
        (short, long, types): &mut (bool, bool, bool),
    ) -> Columns {
        let mut columns = Columns::default();
        for meta in visible(metas) {
            match meta {
                Meta::Position if depth == 0 => {
                    columns.short += 3 + if replace(short, true) { 2 } else { 0 }
                }
                Meta::Name(Name::Short, value) if depth == 0 => {
                    columns.short += value.len() + if replace(short, true) { 2 } else { 0 }
                }
                Meta::Name(Name::Long, value) if depth == 0 => {
                    columns.long += value.len() + if replace(long, true) { 2 } else { 0 }
                }
                Meta::Type(value) if depth == 0 => {
                    columns.types += value.len() + if replace(types, true) { 2 } else { 0 }
                }
                Meta::Root(metas)
                | Meta::Option(metas)
                | Meta::Verb(metas)
                | Meta::Group(metas)
                    if depth > 0 =>
                {
                    let child = Self::columns(metas, depth - 1, &mut (false, false, false));
                    columns.short = columns.short.max(child.short);
                    columns.long = columns.long.max(child.long);
                    columns.types = columns.types.max(child.types);
                }
                _ => {}
            }
        }
        columns
    }

    fn tags(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut count = self.join(metas, "[", ", ", |meta| match meta {
            Meta::Require => Some(Cow::Borrowed("require")),
            Meta::Swizzle => Some(Cow::Borrowed("swizzle")),
            Meta::Many(_) => Some(Cow::Borrowed("many")),
            _ => None,
        })?;
        let prefix = if count == 0 { "[" } else { ", " };
        count += self.join(
            metas,
            format_args!("{prefix}valid: "),
            " | ",
            |meta| match meta {
                Meta::Valid(value) => Some(Cow::Borrowed(value)),
                _ => None,
            },
        )?;
        let prefix = if count == 0 { "[" } else { ", " };
        count += self.join(
            metas,
            format_args!("{prefix}default: "),
            " | ",
            |meta| match meta {
                Meta::Default(value) => Some(Cow::Borrowed(value)),
                Meta::Environment(value) => Some(Cow::Owned(format!("${value}"))),
                _ => None,
            },
        )?;
        if count > 0 {
            write!(self.buffer, "]")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn node(&mut self, metas: &[Meta], depth: usize) -> Result<(), fmt::Error> {
        let mut option = 0;
        let columns = Self::columns(metas, 1, &mut (false, false, false));
        let mut helper = self.own();
        for meta in visible(metas) {
            match meta {
                Meta::Help(value) => {
                    helper.indentation()?;
                    helper.wrap(value, "", "", &mut 0, &mut false)?;
                    writeln!(helper.buffer)?;
                }
                Meta::Note(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        format_args!("{}{Italic}", NOTE.fg_string()),
                        format_args!("{Reset}"),
                        &mut 0,
                        &mut false,
                    )?;
                    writeln!(helper.buffer)?;
                }
                Meta::Usage(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        format_args!("{}{Underline}", USAGE.fg_string()),
                        format_args!("{Reset}"),
                        &mut 0,
                        &mut false,
                    )?;
                    writeln!(helper.buffer)?;
                }
                Meta::Root(metas) => {
                    writeln!(helper.buffer)?;
                    helper.indentation()?;
                    if helper.names(
                        metas,
                        true,
                        true,
                        format_args!("{}{Bold}", ROOT.fg_string()),
                        &mut 0,
                    )? > 0
                    {
                        helper.versions(metas, " ")?;
                        helper.authors(metas, format_args!("{Italic}{Faint} by "))?;
                        write!(helper.buffer, "{Reset}")?;
                    }
                    writeln!(helper.buffer)?;
                    helper.node(metas, depth + 1)?;
                }
                Meta::Group(metas) => {
                    helper.indentation()?;
                    if helper.names(
                        metas,
                        true,
                        true,
                        format_args!("{}{Bold}", GROUP.fg_string()),
                        &mut 0,
                    )? > 0
                    {
                        writeln!(helper.buffer, "{Reset}")?;
                        helper.indent().node(metas, depth + 1)?;
                        writeln!(helper.buffer)?;
                    } else {
                        helper.node(metas, depth + 1)?;
                    }
                }
                Meta::Verb(metas) if depth == 0 => {
                    helper.indentation()?;
                    if helper.names(
                        metas,
                        true,
                        true,
                        format_args!("{}{Bold}", VERB.fg_string()),
                        &mut 0,
                    )? > 0
                    {
                        helper.versions(metas, " ")?;
                        writeln!(helper.buffer, "{Reset}")?;
                    } else {
                        writeln!(helper.buffer)?;
                    }
                    helper.indent().node(metas, depth + 1)?;
                    writeln!(helper.buffer)?;
                }
                Meta::Verb(metas) => {
                    helper.indentation()?;
                    helper
                        .write_columns(metas, &columns, true, &mut 0)?
                        .help(metas)?;
                    writeln!(helper.buffer)?;
                }
                Meta::Option(metas) => {
                    helper.indentation()?;
                    let mut helper = helper.write_columns(metas, &columns, false, &mut option)?;
                    let width = helper.write_with(|helper| helper.help(metas))?;
                    let buffer = helper.scope(|mut helper| helper.tags(metas))?;
                    if width + buffer.len() > helper.width - helper.indent {
                        writeln!(helper.buffer)?;
                        helper.indentation()?;
                    } else {
                        write!(helper.buffer, " ")?;
                    }
                    writeln!(helper.buffer, "{Faint}{buffer}{Reset}")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn write_columns(
        &mut self,
        metas: &[Meta],
        columns: &Columns,
        verb: bool,
        option: &mut usize,
    ) -> Result<Helper, fmt::Error> {
        let mut format = 0;
        let width = self.write_with(|helper| {
            if verb {
                format += helper.write(format_args!("{}{Bold}", VERB.fg_string()))?;
            } else {
                format += helper.write(format_args!("{}", OPTION.fg_string()))?;
            }
            helper.write_column(columns.short, format_args!("{INDENTATION}"), |helper| {
                helper.names(metas, true, false, "", option)?;
                Ok(())
            })?;
            helper.write_column(columns.long, format_args!("{INDENTATION}"), |helper| {
                helper.names(metas, false, true, "", option)?;
                Ok(())
            })?;
            format += helper.write(format_args!("{Reset}"))?;
            format += helper.write(format_args!("{Faint}"))?;
            helper.write_column(columns.types + 2, format_args!("{INDENTATION}"), |helper| {
                if helper.types(metas, "<")? > 0 {
                    write!(helper.buffer, ">")?;
                }
                Ok(())
            })?;
            format += helper.write(format_args!("{Reset}"))?;
            Ok(())
        })?;
        Ok(self.indent_with(width.saturating_sub(format)))
    }

    fn write(&mut self, value: impl Display) -> Result<usize, fmt::Error> {
        self.write_with(|helper| write!(helper.buffer, "{value}"))
    }

    fn write_column(
        &mut self,
        width: usize,
        suffix: impl fmt::Display,
        write: impl FnOnce(&mut Self) -> Result<(), fmt::Error>,
    ) -> Result<bool, fmt::Error> {
        if width > 0 {
            let actual = self.write_with(write)?;
            write!(self.buffer, "{suffix}")?;
            self.space(width.saturating_sub(actual))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn write_with(
        &mut self,
        write: impl FnOnce(&mut Self) -> Result<(), fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        let start = self.buffer.len();
        write(self)?;
        Ok(self.buffer.len().saturating_sub(start))
    }
}

pub(crate) fn help(meta: &Meta) -> Option<String> {
    let mut buffer = String::new();
    let mut writer = Helper {
        buffer: &mut buffer,
        indent: 0,
        width: term_size::dimensions().map_or(96, |pair| pair.0 - 16),
    };
    writer.node(from_ref(meta), 0).ok()?;
    Some(buffer)
}

pub(crate) fn version(meta: &Meta, depth: usize) -> Option<String> {
    join(meta, depth, |meta| match meta {
        Meta::Version(version) => Some(Cow::Borrowed(version)),
        _ => None,
    })
}

pub(crate) fn license(meta: &Meta, depth: usize) -> Option<String> {
    join(meta, depth, |meta| match meta {
        Meta::License(name, file) => match fs::read_to_string(file.deref()) {
            Ok(content) => Some(Cow::Owned(content)),
            Err(_) if file.chars().all(char::is_whitespace) => Some(Cow::Borrowed(name)),
            Err(_) => Some(Cow::Borrowed(file)),
        },
        _ => None,
    })
}

pub(crate) fn author(meta: &Meta, depth: usize) -> Option<String> {
    join(meta, depth, |meta| match meta {
        Meta::Author(author) => Some(Cow::Borrowed(author)),
        _ => None,
    })
}

fn visible(metas: &[Meta]) -> impl Iterator<Item = &Meta> {
    let mut metas = metas.iter();
    from_fn(move || loop {
        let meta = metas.next()?;
        match meta {
            Meta::Hide => loop {
                if let Meta::Show = metas.next()? {
                    break;
                }
            },
            meta => return Some(meta),
        }
    })
}

fn join(meta: &Meta, depth: usize, find: impl Fn(&Meta) -> Option<Cow<str>>) -> Option<String> {
    fn descend(
        meta: &Meta,
        depth: usize,
        buffer: &mut String,
        find: impl Fn(&Meta) -> Option<Cow<str>> + Copy,
    ) -> Result<(), fmt::Error> {
        match meta {
            Meta::Root(metas) | Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas)
                if depth > 0 =>
            {
                for meta in metas {
                    descend(meta, depth - 1, buffer, find)?;
                }
            }
            meta => match find(meta) {
                Some(value) if buffer.is_empty() => write!(buffer, "{value}")?,
                Some(value) => write!(buffer, ", {value}")?,
                None => {}
            },
        }
        Ok(())
    }

    let mut buffer = String::new();
    descend(meta, depth, &mut buffer, &find).ok()?;
    Some(buffer)
}
