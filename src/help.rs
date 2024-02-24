use orn::Or2;

use crate::{
    meta::{Meta, Name},
    style::{Format, Item, Line, Style},
};
use core::{
    fmt::{self, Write},
    mem::{replace, take},
    slice::from_ref,
};
use std::{borrow::Cow, fs, iter::from_fn, ops::Deref};

struct Helper<'a, S: Style + ?Sized> {
    buffer: &'a mut String,
    style: &'a S,
    indent: usize,
}

#[derive(Default)]
struct Columns {
    short: usize,
    long: usize,
    types: usize,
}

struct Wrap<F>(F);

impl<F: Format> fmt::Display for Wrap<F> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.format(f)
    }
}

impl<'a, S: Style + ?Sized + 'a> Helper<'a, S> {
    fn space(&mut self, width: usize) -> Result<usize, fmt::Error> {
        for _ in 0..width {
            write!(self.buffer, " ")?;
        }
        Ok(width)
    }

    fn own(&mut self) -> Helper<S> {
        Helper {
            buffer: self.buffer,
            style: self.style,
            indent: self.indent,
        }
    }

    fn indent(&mut self) -> Helper<S> {
        self.indent_with(self.style.indent())
    }

    fn indent_with(&mut self, by: usize) -> Helper<S> {
        let mut helper = self.own();
        helper.indent += by;
        helper
    }

    fn indentation(&mut self) -> Result<usize, fmt::Error> {
        self.space(self.indent)?;
        Ok(self.indent)
    }

    fn scope<T>(
        &mut self,
        scope: impl FnOnce(Helper<S>) -> Result<T, fmt::Error>,
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
        prefix: impl Format,
        suffix: impl Format,
        position: &mut usize,
    ) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, suffix, ", ", |meta| match meta {
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

    fn types(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
    ) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, suffix, ", ", |meta| match meta {
            Meta::Type(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })
    }

    fn versions(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
    ) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, suffix, ", ", |meta| match meta {
            Meta::Version(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })
    }

    fn authors(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
    ) -> Result<usize, fmt::Error> {
        self.join(metas, prefix, suffix, ", ", |meta| match meta {
            Meta::Author(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })
    }

    fn join(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
        separator: impl Format,
        mut find: impl FnMut(&Meta) -> Option<Cow<str>>,
    ) -> Result<usize, fmt::Error> {
        let mut width = 0;
        let mut prefix = Some(prefix);
        for meta in visible(metas) {
            if let Some(value) = find(meta) {
                match prefix.take() {
                    Some(prefix) => width += self.write(prefix)?,
                    None => width += self.write(&separator)?,
                }
                width += self.write(value)?;
            }
        }
        if prefix.is_none() {
            width += self.write(suffix)?;
        }
        Ok(width)
    }

    fn wrap(
        &mut self,
        value: &str,
        prefix: impl Format,
        suffix: impl Format,
        pre: impl Format,
        post: impl Format,
        cursor: &mut usize,
        has: &mut bool,
    ) -> Result<usize, fmt::Error> {
        if value.is_empty() {
            return Ok(0);
        }
        let mut width = 0;
        let mut prefix = Some(prefix);
        for line in value.split('\n') {
            match prefix.take() {
                Some(prefix) if !replace(has, true) => {
                    width += self.write(prefix)?;
                }
                _ => {
                    width += self.write_line(())?;
                    *cursor = 0;
                    width += self.write(&pre)?;
                    *cursor += self.indentation()?;
                    width += self.write(&post)?;
                }
            }

            let mut has = false;
            for word in line.split(' ') {
                if replace(&mut has, true) {
                    width += self.write(" ")?;
                }

                if *cursor + word.len() > self.style.width() {
                    width += self.write_line(())?;
                    *cursor = 0;
                    width += self.write(&pre)?;
                    *cursor += self.indentation()?;
                    width += self.write(&post)?;
                }
                *cursor += self.write(word)?;
            }
        }
        if width > 0 {
            width += self.write(suffix)?;
        }
        Ok(width)
    }

    fn description(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
        line: impl Format,
    ) -> Result<usize, fmt::Error> {
        let mut count = 0;
        for meta in visible(metas) {
            if let Meta::Summary(value) = meta {
                if !value.chars().all(char::is_whitespace) {
                    count += self.write_line(())?;
                    count += self.indentation()?;
                    count += self.wrap(value, &prefix, "", "", &line, &mut 0, &mut false)?;
                }
            }
        }
        if count > 0 {
            count += self.write(suffix)?;
        }
        Ok(count)
    }

    fn summary(
        &mut self,
        metas: &[Meta],
        prefix: impl Format,
        suffix: impl Format,
        mut cursor: usize,
    ) -> Result<usize, fmt::Error> {
        let mut has = false;
        let mut width = 0;
        for meta in visible(metas) {
            if let Meta::Summary(value) = meta {
                width += self.wrap(value, &prefix, &suffix, "", "", &mut cursor, &mut has)?;
            }
        }
        if width == 0 {
            for meta in visible(metas) {
                if let Meta::Help(value) = meta {
                    width += self.wrap(value, &prefix, &suffix, "", "", &mut cursor, &mut has)?;
                }
            }
        }
        Ok(width)
    }

    fn columns(
        &self,
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
                    if columns.types == 0 {
                        columns.types += self.style.begin(Item::Type).width();
                        columns.types += self.style.end(Item::Type).width();
                    }
                    columns.types += value.len() + if replace(types, true) { 2 } else { 0 }
                }
                Meta::Root(metas)
                | Meta::Option(metas)
                | Meta::Verb(metas)
                | Meta::Group(metas)
                    if depth > 0 =>
                {
                    let child = self.columns(metas, depth - 1, &mut (false, false, false));
                    columns.short = columns.short.max(child.short);
                    columns.long = columns.long.max(child.long);
                    columns.types = columns.types.max(child.types);
                }
                _ => {}
            }
        }
        columns
    }

    fn tags(&mut self, metas: &[Meta]) -> Result<usize, fmt::Error> {
        let mut width = self.join(metas, "", "", ", ", |meta| match meta {
            Meta::Require => Some(Cow::Borrowed("require")),
            Meta::Swizzle => Some(Cow::Borrowed("swizzle")),
            Meta::Many(_) => Some(Cow::Borrowed("many")),
            _ => None,
        })?;
        let prefix = if width > 0 { ", " } else { "" };
        width += self.join(metas, (prefix, "valid: "), "", " | ", |meta| match meta {
            Meta::Valid(value) => Some(Cow::Borrowed(value)),
            _ => None,
        })?;
        let prefix = if width > 0 { ", " } else { "" };
        width += self.join(metas, (prefix, "default: "), "", " | ", |meta| match meta {
            Meta::Default(value) => Some(Cow::Borrowed(value)),
            Meta::Environment(value) => Some(Cow::Owned(format!("${value}"))),
            _ => None,
        })?;
        Ok(width)
    }

    fn node(&mut self, metas: &[Meta], depth: usize) -> fmt::Result {
        let mut option = 0;
        let columns = self.columns(metas, 1, &mut (false, false, false));
        let mut helper = self.own();
        for meta in visible(metas) {
            match meta {
                Meta::Help(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        helper.style.begin(Item::Help),
                        helper.style.end(Item::Help),
                        "",
                        "",
                        &mut 0,
                        &mut false,
                    )?;
                    helper.write_line("")?;
                }
                Meta::Line => {
                    helper.write_line("")?;
                }
                Meta::Note(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        helper.style.begin(Item::Note),
                        helper.style.end(Item::Note),
                        "",
                        "",
                        &mut 0,
                        &mut false,
                    )?;
                    helper.write_line("")?;
                }
                Meta::Root(metas) => {
                    helper.write_header(metas)?;
                    helper.node(metas, depth + 1)?;
                }
                Meta::Group(metas) => {
                    helper.indentation()?;
                    let width = helper.names(
                        metas,
                        true,
                        true,
                        helper.style.begin(Item::Group),
                        helper.style.end(Item::Group),
                        &mut 0,
                    )?;
                    helper.write_line("")?;
                    if width > 0 {
                        helper.indent().node(metas, depth + 1)?;
                        helper.write_line("")?;
                    } else {
                        helper.node(metas, depth + 1)?;
                    }
                }
                Meta::Verb(metas) if depth == 0 => {
                    helper.write_header(metas)?;
                    helper.node(metas, depth + 1)?;
                }
                Meta::Verb(metas) => {
                    helper.indentation()?;
                    let mut helper = helper.write_columns(metas, &columns, true, &mut 0)?;
                    helper.summary(
                        metas,
                        helper.style.begin(Item::Summary),
                        helper.style.end(Item::Summary),
                        helper.indent,
                    )?;
                    helper.write_line("")?;
                }
                Meta::Option(metas) => {
                    helper.indentation()?;
                    let mut helper = helper.write_columns(metas, &columns, false, &mut option)?;
                    let mut width = helper.indent;
                    width += helper.summary(
                        metas,
                        helper.style.begin(Item::Summary),
                        helper.style.end(Item::Summary),
                        width,
                    )?;
                    let buffer = helper.scope(|mut helper| helper.tags(metas))?;
                    if width + buffer.len() > helper.style.width() {
                        helper.write_line("")?;
                        width = helper.indentation()?;
                    } else if width > 0 {
                        width += helper.write(" ")?;
                    }
                    helper.wrap(
                        &buffer,
                        helper.style.begin(Item::Tag),
                        helper.style.end(Item::Tag),
                        "",
                        "",
                        &mut width,
                        &mut false,
                    )?;
                    helper.write_line("")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn write_header(&mut self, metas: &[Meta]) -> Result<usize, fmt::Error> {
        let mut width = 0;
        width += self.write_line(())?;
        width += self.indentation()?;
        let count = self.names(
            metas,
            true,
            true,
            (
                '\n',
                self.style.begin(Item::Bar(Line::Head)),
                self.style.end(Item::Bar(Line::Head)),
                ' ',
                self.style.begin(Item::Head),
            ),
            self.style.end(Item::Head),
            &mut 0,
        )?;
        if count > 0 {
            width += count;
            width += self.versions(
                metas,
                (self.style.begin(Item::Version), ' '),
                self.style.end(Item::Version),
            )?;
            width += self.authors(
                metas,
                (self.style.begin(Item::Author), " by "),
                self.style.end(Item::Author),
            )?;
            width += self.write_line(())?;
        }
        let mut has = false;
        let buffer = self.scope(|mut helper| {
            let line = (
                helper.style.begin(Item::Bar(Line::Description)),
                helper.style.end(Item::Bar(Line::Description)),
                helper.style.begin(Item::Arrow(Line::Description)),
                helper.style.end(Item::Arrow(Line::Description)),
                ' ',
                helper.style.begin(Item::Description),
            );
            let count = helper.description(
                metas,
                line,
                helper.style.end(Item::Description),
                (helper.style.end(Item::Description), line),
            )?;
            width += count;
            has |= count > 0;

            let count = helper.join(
                metas,
                (
                    '\n',
                    helper.style.begin(Item::Bar(Line::Link)),
                    helper.style.end(Item::Bar(Line::Link)),
                    helper.style.begin(Item::Arrow(Line::Link)),
                    helper.style.end(Item::Arrow(Line::Link)),
                    ' ',
                    helper.style.begin(Item::Link),
                ),
                helper.style.end(Item::Link),
                " ",
                |meta| match meta {
                    Meta::Home(value) => Some(Cow::Borrowed(value)),
                    _ => None,
                },
            )?;
            width += count;
            has |= count > 0;

            let count = helper.join(
                metas,
                (
                    '\n',
                    helper.style.begin(Item::Bar(Line::Link)),
                    helper.style.end(Item::Bar(Line::Link)),
                    helper.style.begin(Item::Arrow(Line::Link)),
                    helper.style.end(Item::Arrow(Line::Link)),
                    ' ',
                    helper.style.begin(Item::Link),
                ),
                helper.style.end(Item::Link),
                " ",
                |meta| match meta {
                    Meta::Repository(value) => Some(Cow::Borrowed(value)),
                    _ => None,
                },
            )?;
            width += count;
            has |= count > 0;

            width += helper.join(
                metas,
                (
                    if has {
                        Or2::T0((
                            '\n',
                            helper.style.begin(Item::Bar(Line::Usage)),
                            helper.style.end(Item::Bar(Line::Usage)),
                        ))
                    } else {
                        Or2::T1(())
                    },
                    '\n',
                    helper.style.begin(Item::Bar(Line::Usage)),
                    helper.style.end(Item::Bar(Line::Usage)),
                    helper.style.begin(Item::Arrow(Line::Usage)),
                    helper.style.end(Item::Arrow(Line::Usage)),
                    ' ',
                    helper.style.begin(Item::Usage),
                ),
                helper.style.end(Item::Usage),
                " ",
                |meta| match meta {
                    Meta::Usage(value) => Some(Cow::Borrowed(value)),
                    _ => None,
                },
            )?;
            Ok(())
        })?;

        if has {
            self.write_line((
                self.style.begin(Item::Bar(Line::Head)),
                self.style.end(Item::Bar(Line::Head)),
                buffer,
            ))?;
        }

        Ok(width)
    }

    fn write_columns(
        &mut self,
        metas: &[Meta],
        columns: &Columns,
        verb: bool,
        option: &mut usize,
    ) -> Result<Helper<S>, fmt::Error> {
        let item = if verb { Item::Verb } else { Item::Option };
        let mut width = 0;
        let pad = self.style.indent();
        width += self.write_column(columns.short, pad, |helper| {
            helper.names(
                metas,
                true,
                false,
                helper.style.begin(item),
                helper.style.end(item),
                option,
            )
        })?;
        width += self.write_column(columns.long, pad, |helper| {
            helper.names(
                metas,
                false,
                true,
                helper.style.begin(item),
                helper.style.end(item),
                option,
            )
        })?;
        width += self.write_column(columns.types, pad, |helper| {
            helper.types(
                metas,
                helper.style.begin(Item::Type),
                helper.style.end(Item::Type),
            )
        })?;
        Ok(self.indent_with(width))
    }

    #[inline]
    fn write(&mut self, value: impl Format) -> Result<usize, fmt::Error> {
        let width = value.width();
        write!(self.buffer, "{}", Wrap(value))?;
        Ok(width)
    }

    #[inline]
    fn write_line(&mut self, value: impl Format) -> Result<usize, fmt::Error> {
        let width = value.width();
        writeln!(self.buffer, "{}", Wrap(value))?;
        Ok(width)
    }

    fn write_column(
        &mut self,
        width: usize,
        pad: usize,
        write: impl FnOnce(&mut Self) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        if width == 0 {
            Ok(0)
        } else {
            let count = write(self)?;
            Ok(count + self.space(width.saturating_sub(count) + pad)?)
        }
    }
}

pub(crate) fn help<S: Style + ?Sized>(meta: &Meta, style: &S) -> Option<String> {
    let mut buffer = String::new();
    let mut writer = Helper {
        buffer: &mut buffer,
        style,
        indent: 0,
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
    ) -> fmt::Result {
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
