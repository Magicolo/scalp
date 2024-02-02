use crate::meta::Meta;
use core::{
    fmt::{self, Write},
    mem::{replace, take},
    slice::from_ref,
};
use std::borrow::Cow;
use termion::style::{Bold, Faint, Italic, NoFaint, NoItalic, NoUnderline, Reset, Underline};

const INDENT: usize = 2;
const OPEN: &str = "<";
const CLOSE: &str = ">";
const ASSIGN: &str = " = ";

struct Helper<'a> {
    buffer: &'a mut String,
    indent: usize,
    width: usize,
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
        prefix: &str,
        position: &mut usize,
    ) -> Result<bool, fmt::Error> {
        let mut has = false;
        let mut join = false;
        let mut metas = metas.iter();
        let mut separate = |buffer: &mut String| {
            if join {
                write!(buffer, ", ")?;
            } else {
                join = true;
                write!(buffer, "{prefix}")?;
            }
            Ok::<_, fmt::Error>(())
        };
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Name(value) => {
                    separate(self.buffer)?;
                    write!(self.buffer, "{value}")?;
                    has = true;
                }
                Meta::Position => {
                    separate(self.buffer)?;
                    write!(self.buffer, "[{position}]")?;
                    *position += 1;
                    has = true;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(has)
    }

    fn versions(&mut self, metas: &[Meta], prefix: &str) -> Result<bool, fmt::Error> {
        let mut join = false;
        let mut has = false;
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Version(value) => {
                    if join {
                        write!(self.buffer, ", ")?;
                    } else {
                        join = true;
                        write!(self.buffer, "{prefix}")?;
                    }
                    write!(self.buffer, "{value}")?;
                    has = true;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(has)
    }

    fn wrap(
        &mut self,
        value: &str,
        prefix: &str,
        suffix: &str,
        cursor: &mut usize,
        join: &mut bool,
    ) -> Result<(), fmt::Error> {
        for line in value.split('\n') {
            if replace(join, true) {
                writeln!(self.buffer)?;
                self.indentation()?;
            } else {
                write!(self.buffer, "{prefix}")?;
                *cursor += prefix.len();
            }

            for word in line.split(' ') {
                self.word(word, cursor)?;
                write!(self.buffer, " ")?;
            }
        }
        if *join {
            write!(self.buffer, "{suffix}")?;
            *cursor += suffix.len();
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
        let mut metas = metas.iter();
        let mut join = false;
        let mut cursor = 0;
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Help(value) => self.wrap(value, "", "", &mut cursor, &mut join)?,
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(())
    }

    fn name_width(metas: &[Meta], depth: usize, join: &mut bool) -> usize {
        let mut width = 0;
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Name(name) if depth == 0 => {
                    width += name.len() + if replace(join, true) { 2 } else { 0 }
                }
                Meta::Position if depth == 0 => {
                    width += 3 + if replace(join, true) { 2 } else { 0 }
                }
                Meta::Root(metas)
                | Meta::Option(metas)
                | Meta::Verb(metas)
                | Meta::Group(metas)
                    if depth > 0 =>
                {
                    width = width.max(Self::name_width(metas, depth - 1, &mut false))
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        width
    }

    fn tags(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut name = "";
        let mut many = false;
        let mut required = false;
        {
            let mut metas = metas.iter();
            while let Some(meta) = metas.next() {
                match meta {
                    Meta::Type(value, _) => {
                        name = value;
                        many = false;
                    }
                    Meta::Many(_) => many = true,
                    Meta::Required => required = true,
                    Meta::Hide => hide(metas.by_ref()),
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Ok(false);
        }

        write!(self.buffer, "{OPEN}")?;
        if required {
            write!(self.buffer, "required ")?;
        }
        write!(self.buffer, "{name}")?;
        if many {
            write!(self.buffer, " list")?;
        }
        let mut join = false;
        let mut separate = |buffer: &mut String| {
            if join {
                write!(buffer, ", ")
            } else {
                join = true;
                write!(buffer, "{ASSIGN}")
            }
        };
        for meta in metas {
            match meta {
                Meta::Default(value) => {
                    separate(self.buffer)?;
                    write!(self.buffer, "{value}")?;
                }
                Meta::Environment(value) => {
                    separate(self.buffer)?;
                    write!(self.buffer, "${value}")?;
                }
                _ => {}
            }
        }
        write!(self.buffer, "{CLOSE}")?;

        Ok(true)
    }

    fn node(&mut self, metas: &[Meta], depth: usize) -> Result<(), fmt::Error> {
        let mut option = 0;
        let names = Self::name_width(metas, 1, &mut false);
        let mut helper = self.own();
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
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
                        Italic.as_ref(),
                        NoItalic.as_ref(),
                        &mut 0,
                        &mut false,
                    )?;
                    writeln!(helper.buffer)?;
                }
                Meta::Usage(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        Underline.as_ref(),
                        NoUnderline.as_ref(),
                        &mut 0,
                        &mut false,
                    )?;
                    writeln!(helper.buffer)?;
                }
                Meta::Root(metas) => {
                    writeln!(helper.buffer)?;
                    helper.indentation()?;
                    if helper.names(metas, Bold.as_ref(), &mut 0)? {
                        helper.versions(metas, " ")?;
                        writeln!(helper.buffer, "{Reset}")?;
                    } else {
                        writeln!(helper.buffer)?;
                    }
                    helper.node(metas, depth + 1)?;
                }
                Meta::Group(metas) => {
                    helper.indentation()?;
                    if helper.names(metas, Bold.as_ref(), &mut 0)? {
                        writeln!(helper.buffer, "{Reset}")?;
                        helper.indent().node(metas, depth + 1)?;
                        writeln!(helper.buffer)?;
                    } else {
                        helper.node(metas, depth + 1)?;
                    }
                }
                Meta::Verb(metas) if depth == 0 => {
                    writeln!(helper.buffer)?;
                    helper.indentation()?;
                    if helper.names(metas, Bold.as_ref(), &mut 0)? {
                        helper.versions(metas, " ")?;
                        writeln!(helper.buffer, "{Reset}")?;
                    } else {
                        writeln!(helper.buffer)?;
                    }
                    helper.indent().node(metas, depth + 1)?;
                }
                Meta::Verb(metas) => {
                    helper.indentation()?;
                    let indent = names + INDENT;
                    let start = helper.buffer.len();
                    helper.names(metas, "", &mut 0)?;
                    let width = helper.buffer.len().saturating_sub(start);
                    helper.space(indent.saturating_sub(width))?;
                    helper.indent_with(indent).help(metas)?;
                    writeln!(helper.buffer)?;
                }
                Meta::Option(metas) => {
                    helper.indentation()?;
                    let indent = names + INDENT;
                    let start = helper.buffer.len();
                    helper.names(metas, "", &mut option)?;
                    let width = helper.buffer.len().saturating_sub(start);
                    helper.space(indent.saturating_sub(width))?;

                    let mut helper = helper.indent_with(indent);
                    let start = helper.buffer.len();
                    helper.help(metas)?;
                    let width = helper.buffer.len().saturating_sub(start);
                    let buffer = helper.scope(|mut helper| helper.tags(metas))?;
                    if width + buffer.len() > helper.width - helper.indent {
                        writeln!(helper.buffer)?;
                        helper.indentation()?;
                    }
                    writeln!(helper.buffer, "{Italic}{Faint}{buffer}{NoFaint}{NoItalic}")?;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(())
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

pub(crate) fn version(meta: &Meta, depth: usize) -> Option<&Cow<'static, str>> {
    match meta {
        Meta::Version(version) => Some(version),
        Meta::Root(metas) | Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas)
            if depth > 0 =>
        {
            metas.iter().find_map(|meta| version(meta, depth - 1))
        }
        _ => None,
    }
}

fn hide<'a>(metas: impl Iterator<Item = &'a Meta>) {
    for meta in metas {
        if let Meta::Show = meta {
            return;
        }
    }
}
