use crate::{
    meta::{Meta, Prefix, Text, prefix},
    parse::Key,
    style::{Format, Item, Line, Style},
};
use core::{fmt, mem::replace, slice::from_ref};
use std::{
    borrow::Cow,
    cell::Cell,
    fs,
    ops::{ControlFlow, Deref},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct Help {
    pub(crate) root: Arc<Meta>,
    pub(crate) meta: Arc<Meta>,
    pub(crate) path: Vec<Key>,
    pub(crate) style: Arc<dyn Style>,
}
#[derive(Debug, Clone)]
pub struct Version(pub(crate) Arc<Meta>);
#[derive(Debug, Clone)]
pub struct License(pub(crate) Arc<Meta>);
#[derive(Debug, Clone)]
pub struct Author(pub(crate) Arc<Meta>);

struct Helper<'a, 'b> {
    format: &'a mut fmt::Formatter<'b>,
    path: &'a [Key],
    style: &'a dyn Style,
    indent: usize,
}

#[derive(Default)]
struct Columns {
    short: usize,
    long: usize,
    types: usize,
}

impl fmt::Display for Help {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut writer = Helper {
            format: f,
            path: &self.path,
            style: &self.style,
            indent: 0,
        };
        writer.node(&self.root, from_ref(&self.meta), 0)
    }
}

impl Version {
    pub fn versions(&self) -> impl Iterator<Item = &Text> {
        self.0.children().iter().filter_map(|meta| match meta {
            Meta::Version(version) => Some(version),
            _ => None,
        })
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        &self.0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        join(f, self.versions())
    }
}

impl Author {
    pub fn authors(&self) -> impl Iterator<Item = &Text> {
        self.0.children().iter().filter_map(|meta| match meta {
            Meta::Author(name) => Some(name),
            _ => None,
        })
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        &self.0
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        join(f, self.authors())
    }
}

impl License {
    pub fn licenses(&self) -> impl Iterator<Item = (&Text, &Text)> {
        self.0.children().iter().filter_map(|meta| match meta {
            Meta::License(name, file) => Some((name, file)),
            _ => None,
        })
    }

    #[inline]
    pub fn meta(&self) -> &Meta {
        &self.0
    }
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        join(
            f,
            self.licenses()
                .map(|(name, file)| match fs::read_to_string(file.deref()) {
                    Ok(content) => content.into(),
                    Err(_) if file.chars().all(char::is_whitespace) => name.clone(),
                    Err(_) => file.clone(),
                }),
        )
    }
}

impl<'a, 'b> Helper<'a, 'b> {
    fn space(&mut self, width: usize) -> Result<usize, fmt::Error> {
        for _ in 0..width {
            write!(self.format, " ")?;
        }
        Ok(width)
    }

    fn own(&mut self) -> Helper<'_, 'b> {
        Helper {
            format: self.format,
            path: self.path,
            style: self.style,
            indent: self.indent,
        }
    }

    fn with<'c, 'd>(&'c self, formatter: &'c mut fmt::Formatter<'d>) -> Helper<'c, 'd> {
        Helper {
            format: formatter,
            path: self.path,
            style: self.style,
            indent: self.indent,
        }
    }

    fn indent(&mut self) -> Helper<'_, 'b> {
        self.indent_with(self.style.indent())
    }

    fn indent_with(&mut self, by: usize) -> Helper<'_, 'b> {
        let mut helper = self.own();
        helper.indent += by;
        helper
    }

    fn indentation(&mut self) -> Result<usize, fmt::Error> {
        self.space(self.indent)?;
        Ok(self.indent)
    }

    fn scope<F: FnOnce(Helper) -> fmt::Result>(&mut self, scope: F) -> Result<String, fmt::Error> {
        struct Scope<'a, 'b, 'c, F>(&'c Helper<'a, 'b>, Cell<Option<F>>);
        impl<F: FnOnce(Helper) -> fmt::Result> fmt::Display for Scope<'_, '_, '_, F> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if let Some(scope) = self.1.take() {
                    scope(self.0.with(f))?;
                }
                Ok(())
            }
        }
        Ok(format!("{}", Scope(self, Cell::new(Some(scope)))))
    }

    fn names(
        &mut self,
        metas: &[Meta],
        short: bool,
        long: bool,
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        let mut position = 0;
        self.join(
            metas,
            prefix,
            suffix,
            |mut helper| helper.write(", "),
            |meta| match meta {
                Meta::Name(value) => match self::prefix(value) {
                    Prefix::None => Some(Cow::Borrowed(value)),
                    Prefix::Short if short => Some(Cow::Borrowed(value)),
                    Prefix::Long if long => Some(Cow::Borrowed(value)),
                    _ => None,
                },
                // TODO: This is wrong? Position needs to consider sibling nodes which may not be
                // available through 'metas'.
                // - Example: When rendering the help for an option with '.position()'.
                Meta::Position if short => {
                    let current = position;
                    position += 1;
                    Some(Cow::Owned(format!("[{current}]")))
                }
                _ => None,
            },
        )
    }

    fn types(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        let mut name = None;
        for meta in Meta::visible(metas) {
            if let Meta::Type(value) = meta {
                name = Some(value);
            }
        }
        match name {
            Some(name) => Ok(prefix(self.own())? + self.write(name)? + suffix(self.own())?),
            None => Ok(0),
        }
    }

    fn versions(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        self.join(
            metas,
            prefix,
            suffix,
            |mut helper| helper.write(", "),
            |meta| match meta {
                Meta::Version(value) => Some(Cow::Borrowed(value)),
                _ => None,
            },
        )
    }

    fn authors(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        self.join(
            metas,
            prefix,
            suffix,
            |mut helper| helper.write(", "),
            |meta| match meta {
                Meta::Author(value) => Some(Cow::Borrowed(value)),
                _ => None,
            },
        )
    }

    fn join(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        separator: impl Fn(Helper) -> Result<usize, fmt::Error>,
        mut find: impl FnMut(&Meta) -> Option<Cow<str>>,
    ) -> Result<usize, fmt::Error> {
        let mut width = 0;
        let mut prefix = Some(prefix);
        for meta in Meta::visible(metas) {
            if let Some(value) = find(meta) {
                match prefix.take() {
                    Some(prefix) => width += prefix(self.own())?,
                    None => width += separator(self.own())?,
                }
                width += self.write(value)?;
            }
        }
        if prefix.is_none() {
            width += suffix(self.own())?;
        }
        Ok(width)
    }

    fn wrap(
        &mut self,
        value: &str,
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        wrap: impl Fn(Helper) -> Result<usize, fmt::Error>,
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
                    width += prefix(self.own())?;
                }
                _ => {
                    width += self.write_line("")?;
                    *cursor = self.indentation()?;
                    width += wrap(self.own())?;
                }
            }

            let mut has = false;
            for word in line.split(' ') {
                if replace(&mut has, true) {
                    width += self.write(" ")?;
                }

                if *cursor + word.len() > self.style.width() {
                    width += self.write_line("")?;
                    *cursor = self.indentation()?;
                    width += wrap(self.own())?;
                }
                *cursor += self.write(word)?;
            }
        }
        if width > 0 {
            width += suffix(self.own())?;
        }
        Ok(width)
    }

    fn description(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        line: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        let mut count = 0;
        for meta in Meta::visible(metas) {
            if let Meta::Summary(value) = meta {
                if !value.chars().all(char::is_whitespace) {
                    count += self.write_line("")?;
                    count += self.indentation()?;
                    count += self.wrap(value, &prefix, |_| Ok(0), &line, &mut 0, &mut false)?;
                }
            }
        }
        if count > 0 {
            count += suffix(self.own())?;
        }
        Ok(count)
    }

    fn summary(
        &mut self,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        mut cursor: usize,
    ) -> Result<usize, fmt::Error> {
        let mut has = false;
        let mut width = 0;
        for meta in Meta::visible(metas) {
            if let Meta::Summary(value) = meta {
                width += self.wrap(value, &prefix, &suffix, |_| Ok(0), &mut cursor, &mut has)?;
            }
        }
        if width == 0 {
            for meta in Meta::visible(metas) {
                if let Meta::Help(value) = meta {
                    width +=
                        self.wrap(value, &prefix, &suffix, |_| Ok(0), &mut cursor, &mut has)?;
                }
            }
        }
        Ok(width)
    }

    fn usage(
        &mut self,
        root: &Meta,
        metas: &[Meta],
        prefix: impl Fn(Helper) -> Result<usize, fmt::Error>,
        suffix: impl Fn(Helper) -> Result<usize, fmt::Error>,
    ) -> Result<usize, fmt::Error> {
        let mut helper = self.own();
        let mut width = helper.join(
            metas,
            &prefix,
            &suffix,
            |mut helper| helper.write(' '),
            |meta| match meta {
                Meta::Usage(value) => Some(Cow::Borrowed(value)),
                _ => None,
            },
        )?;
        if width == 0 {
            width += prefix(helper.own())?;
            width += helper.write("Usage:")?;
            for key in root.key().as_ref().into_iter().chain(helper.path) {
                width += helper.write(' ')?;
                width += helper.write(key)?;
            }

            match Meta::descend(
                metas,
                (),
                false,
                usize::MAX,
                |state, meta| match meta {
                    Meta::Option(_) => ControlFlow::Break(helper.write(" [OPTIONS]")),
                    _ => ControlFlow::Continue(state),
                },
                |state, _| ControlFlow::Continue(state),
            ) {
                ControlFlow::Break(result) => width += result?,
                ControlFlow::Continue(_) => {}
            }

            fn control(result: Result<usize, fmt::Error>) -> ControlFlow<fmt::Error, usize> {
                result.map_or_else(ControlFlow::Break, ControlFlow::Continue)
            }

            match Meta::descend(
                metas,
                None,
                false,
                usize::MAX,
                |state, _| ControlFlow::Continue(state),
                |state, meta| match meta {
                    Meta::Require(value) => ControlFlow::Continue(Some(value)),
                    Meta::Group(_) => {
                        if let Some(value) = state {
                            width += control(helper.write(' '))?;
                            width += control(helper.write('<'))?;
                            width += control(helper.write(value))?;
                            width += control(helper.write('>'))?;
                        }
                        ControlFlow::Continue(None)
                    }
                    Meta::Option(_) | Meta::Verb(_) => ControlFlow::Continue(None),
                    _ => ControlFlow::Continue(state),
                },
            ) {
                ControlFlow::Break(error) => Err(error),
                ControlFlow::Continue(_) => Ok(width + suffix(helper.own())?),
            }
        } else {
            Ok(width)
        }
    }

    fn columns(&mut self, metas: &[Meta], position: &mut usize, depth: usize) -> Columns {
        let (mut short, mut long) = (false, false);
        let mut columns = Columns::default();
        for meta in Meta::visible(metas) {
            let mut helper = self.own();
            match meta {
                Meta::Position if depth == 0 => {
                    *position += 1;
                    if *position < 10 {
                        columns.short += 3 + if replace(&mut short, true) { 2 } else { 0 }
                    } else {
                        columns.short += 4 + if replace(&mut short, true) { 2 } else { 0 }
                    }
                }
                Meta::Name(value) if depth == 0 => match prefix(value) {
                    Prefix::None => {}
                    Prefix::Short => {
                        columns.long += value.len() + if replace(&mut long, true) { 2 } else { 0 }
                    }
                    Prefix::Long => {
                        columns.short += value.len() + if replace(&mut short, true) { 2 } else { 0 }
                    }
                },
                Meta::Type(value) if depth == 0 => {
                    columns.types = value.len();
                    columns.types += helper.style.begin(Item::Type).width();
                    columns.types += helper.style.end(Item::Type).width();
                }
                Meta::Option(metas) | Meta::Verb(metas) | Meta::Group(metas) if depth > 0 => {
                    let child = helper.columns(metas, position, depth - 1);
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
        let mut width = self.join(
            metas,
            |_| Ok(0),
            |_| Ok(0),
            |mut helper| helper.write(", "),
            |meta| match meta {
                Meta::Require(_) => Some(Cow::Borrowed("require")),
                Meta::Swizzle => Some(Cow::Borrowed("swizzle")),
                Meta::Many(_) => Some(Cow::Borrowed("many")),
                _ => None,
            },
        )?;
        let prefix = if width > 0 { ", " } else { "" };
        width += self.join(
            metas,
            |mut helper| Ok(helper.write(prefix)? + helper.write("valid: ")?),
            |_| Ok(0),
            |mut helper| helper.write(" | "),
            |meta| match meta {
                Meta::Valid(value) => Some(Cow::Borrowed(value.as_str())),
                _ => None,
            },
        )?;
        let prefix = if width > 0 { ", " } else { "" };
        width += self.join(
            metas,
            |mut helper| Ok(helper.write(prefix)? + helper.write("default: ")?),
            |_| Ok(0),
            |mut helper| helper.write(" | "),
            |meta| match meta {
                Meta::Default(value) => Some(Cow::Borrowed(value)),
                Meta::Environment(value) => Some(Cow::Owned(format!("${value}"))),
                _ => None,
            },
        )?;
        Ok(width)
    }

    fn node(&mut self, root: &Meta, metas: &[Meta], depth: usize) -> fmt::Result {
        let columns = self.columns(metas, &mut 0, 1);
        for meta in Meta::visible(metas) {
            let mut helper = self.own();
            match meta {
                Meta::Help(value) => {
                    helper.indentation()?;
                    helper.wrap(
                        value,
                        |mut helper| helper.write_begin(Item::Help),
                        |mut helper| helper.write_end(Item::Help),
                        |_| Ok(0),
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
                        |mut helper| helper.write_begin(Item::Note),
                        |mut helper| helper.write_end(Item::Note),
                        |_| Ok(0),
                        &mut 0,
                        &mut false,
                    )?;
                    helper.write_line("")?;
                }
                Meta::Group(metas) | Meta::Verb(metas) if depth == 0 => {
                    helper.write_header(root, metas)?;
                    helper.node(root, metas, depth + 1)?;
                }
                Meta::Group(metas) => {
                    helper.indentation()?;
                    let width = helper.names(
                        metas,
                        true,
                        true,
                        |mut helper| helper.write_begin(Item::Group),
                        |mut helper| helper.write_end(Item::Group),
                    )?;
                    if width > 0 {
                        helper.write_line("")?;
                        helper.indent().node(root, metas, depth + 1)?;
                        helper.write_line("")?;
                    } else {
                        helper.node(root, metas, depth + 1)?;
                    }
                }
                Meta::Verb(metas) => {
                    helper.indentation()?;
                    let mut helper = helper.write_columns(metas, &columns, true)?;
                    helper.summary(
                        metas,
                        |mut helper| helper.write_begin(Item::Summary),
                        |mut helper| helper.write_end(Item::Summary),
                        helper.indent,
                    )?;
                    helper.write_line("")?;
                }
                Meta::Option(metas) => {
                    helper.indentation()?;
                    let mut helper = helper.write_columns(metas, &columns, false)?;
                    let mut width = helper.indent;
                    width += helper.summary(
                        metas,
                        |mut helper| helper.write_begin(Item::Summary),
                        |mut helper| helper.write_end(Item::Summary),
                        width,
                    )?;
                    let buffer = helper.scope(|mut helper| {
                        helper.tags(metas)?;
                        Ok(())
                    })?;
                    if width + buffer.len() > helper.style.width() {
                        helper.write_line("")?;
                        width = helper.indentation()?;
                    } else if width > helper.indent {
                        width += helper.write(" ")?;
                    }
                    helper.wrap(
                        &buffer,
                        |mut helper| helper.write_begin(Item::Tag),
                        |mut helper| helper.write_end(Item::Tag),
                        |_| Ok(0),
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

    fn write_header(&mut self, root: &Meta, metas: &[Meta]) -> Result<usize, fmt::Error> {
        let mut helper = self.own();
        let mut width = 0;
        width += helper.write_line("")?;
        width += helper.indentation()?;
        let count = helper.names(
            metas,
            true,
            true,
            |mut helper| {
                Ok(helper.write('\n')?
                    + helper.write_begin(Item::Bar(Line::Head))?
                    + helper.write_end(Item::Bar(Line::Head))?
                    + helper.write(' ')?
                    + helper.write_begin(Item::Head)?)
            },
            |mut helper| helper.write_end(Item::Head),
        )?;
        if count > 0 {
            width += count;
            width += helper.versions(
                metas,
                |mut helper| Ok(helper.write_begin(Item::Version)? + helper.write(' ')?),
                |mut helper| helper.write_end(Item::Version),
            )?;
            width += helper.authors(
                metas,
                |mut helper| Ok(helper.write_begin(Item::Author)? + helper.write(" by ")?),
                |mut helper| helper.write_end(Item::Author),
            )?;
            width += helper.write_line("")?;
        }
        let mut has = false;
        let buffer = helper.scope(|mut helper| {
            let line = |mut helper: Helper| {
                Ok(helper.write_begin(Item::Bar(Line::Description))?
                    + helper.write_end(Item::Bar(Line::Description))?
                    + helper.write_begin(Item::Arrow(Line::Description))?
                    + helper.write_end(Item::Arrow(Line::Description))?
                    + helper.write(' ')?
                    + helper.write_begin(Item::Description)?)
            };

            let count = helper.description(
                metas,
                line,
                |mut helper| helper.write_end(Item::Description),
                |mut helper| Ok(helper.write_end(Item::Description)? + line(helper)?),
            )?;
            width += count;
            has |= count > 0;

            let count = helper.join(
                metas,
                |mut helper| {
                    Ok(helper.write('\n')?
                        + helper.write_begin(Item::Bar(Line::Link))?
                        + helper.write_end(Item::Bar(Line::Link))?
                        + helper.write_begin(Item::Arrow(Line::Link))?
                        + helper.write_end(Item::Arrow(Line::Link))?
                        + helper.write(' ')?
                        + helper.write_begin(Item::Link)?)
                },
                |mut helper| helper.write_end(Item::Link),
                |mut helper| helper.write(' '),
                |meta| match meta {
                    Meta::Home(value) => Some(Cow::Borrowed(value)),
                    _ => None,
                },
            )?;
            width += count;
            has |= count > 0;

            let count = helper.join(
                metas,
                |mut helper| {
                    Ok(helper.write('\n')?
                        + helper.write_begin(Item::Bar(Line::Link))?
                        + helper.write_end(Item::Bar(Line::Link))?
                        + helper.write_begin(Item::Arrow(Line::Link))?
                        + helper.write_end(Item::Arrow(Line::Link))?
                        + helper.write(' ')?
                        + helper.write_begin(Item::Link)?)
                },
                |mut helper| helper.write_end(Item::Link),
                |mut helper| helper.write(' '),
                |meta| match meta {
                    Meta::Repository(value) => Some(Cow::Borrowed(value)),
                    _ => None,
                },
            )?;
            width += count;
            has |= count > 0;

            let count = helper.usage(
                root,
                metas,
                |mut helper| {
                    Ok(if has {
                        helper.write('\n')?
                            + helper.write_begin(Item::Bar(Line::Usage))?
                            + helper.write_end(Item::Bar(Line::Usage))?
                    } else {
                        0
                    } + helper.write('\n')?
                        + helper.write_begin(Item::Bar(Line::Usage))?
                        + helper.write_end(Item::Bar(Line::Usage))?
                        + helper.write_begin(Item::Arrow(Line::Usage))?
                        + helper.write_end(Item::Arrow(Line::Usage))?
                        + helper.write(' ')?
                        + helper.write_begin(Item::Usage)?)
                },
                |mut helper| helper.write_end(Item::Usage),
            )?;
            width += count;
            has |= count > 0;
            Ok(())
        })?;

        if has {
            width += helper.write_begin(Item::Bar(Line::Head))?;
            width += helper.write_end(Item::Bar(Line::Head))?;
            helper.write_line(buffer)?;
        }
        helper.write_line("")?;

        Ok(width)
    }

    fn write_columns(
        &mut self,
        metas: &[Meta],
        columns: &Columns,
        verb: bool,
    ) -> Result<Helper<'_, 'b>, fmt::Error> {
        let item = if verb { Item::Verb } else { Item::Option };
        let mut width = 0;
        let pad = self.style.indent();
        width += self.write_column(columns.short, pad, |helper| {
            helper.names(
                metas,
                true,
                false,
                |mut helper| helper.write_begin(item),
                |mut helper| helper.write_end(item),
            )
        })?;
        width += self.write_column(columns.long, pad, |helper| {
            helper.names(
                metas,
                false,
                true,
                |mut helper| helper.write_begin(item),
                |mut helper| helper.write_end(item),
            )
        })?;
        width += self.write_column(columns.types, pad, |helper| {
            helper.types(
                metas,
                |mut helper| helper.write_begin(Item::Type),
                |mut helper| helper.write_end(Item::Type),
            )
        })?;
        Ok(self.indent_with(width))
    }

    #[inline]
    fn write(&mut self, value: impl Format) -> Result<usize, fmt::Error> {
        let width = value.width();
        value.format(self.format)?;
        Ok(width)
    }

    #[inline]
    fn write_begin(&mut self, item: Item) -> Result<usize, fmt::Error> {
        self.write(self.style.begin(item))
    }

    #[inline]
    fn write_end(&mut self, item: Item) -> Result<usize, fmt::Error> {
        self.write(self.style.end(item))
    }

    #[inline]
    fn write_line(&mut self, value: impl Format) -> Result<usize, fmt::Error> {
        let width = self.write(value)?;
        writeln!(self.format)?;
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

fn join<D: fmt::Display>(
    write: &mut impl fmt::Write,
    items: impl IntoIterator<Item = D>,
) -> fmt::Result {
    let mut comma = false;
    for item in items {
        if replace(&mut comma, true) {
            write!(write, ", ")?;
        }
        write!(write, "{item}")?;
    }
    Ok(())
}
