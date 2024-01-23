use std::{
    borrow::Cow,
    cmp::max,
    fmt::{self, Write},
    slice::from_ref,
};

use crate::Meta;

struct Helper<'a> {
    short: &'a str,
    long: &'a str,
    buffer: &'a mut String,
    indent: isize,
}

impl<'a> Helper<'a> {
    const INDENT: usize = 2;
    const NAME: usize = 24;
    const TYPE: usize = 16;
    const HELP: usize = 96;

    fn space(&mut self, width: usize) -> Result<(), fmt::Error> {
        for _ in 0..width {
            write!(self.buffer, " ")?;
        }
        Ok(())
    }

    fn own(&mut self) -> Helper {
        Helper {
            short: self.short,
            long: self.long,
            buffer: self.buffer,
            indent: self.indent,
        }
    }

    fn indent(&mut self) -> Helper {
        self.indent_with(Self::INDENT as _)
    }

    fn indent_with(&mut self, by: isize) -> Helper {
        let mut helper = self.own();
        helper.indent += by;
        helper
    }

    fn indentation(&mut self) -> Result<(), fmt::Error> {
        if let Ok(indent) = usize::try_from(self.indent) {
            self.space(indent)
        } else {
            Ok(())
        }
    }

    fn names(&mut self, metas: &[Meta], position: usize) -> Result<bool, fmt::Error> {
        let mut has = false;
        let mut join = false;
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Name(value) => {
                    if join {
                        write!(self.buffer, ", ")?;
                    } else {
                        join = true;
                    }
                    write!(self.buffer, "{value}")?;
                }
                Meta::Position => {
                    if join {
                        write!(self.buffer, ", ")?;
                    } else {
                        join = true;
                    }
                    write!(self.buffer, "[{position}]")?;
                    has = true;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(has)
    }

    fn wrap(&mut self, value: &str, width: usize, mut join: bool) -> Result<bool, fmt::Error> {
        let mut current = 0;
        for line in value.split('\n') {
            if join {
                writeln!(self.buffer)?;
                self.indentation()?;
            } else {
                join = true;
            }

            for word in line.split(' ') {
                if current + word.len() > width {
                    writeln!(self.buffer)?;
                    self.indentation()?;
                    current = 0;
                }
                write!(self.buffer, "{word} ")?;
                current += word.len();
            }
        }
        Ok(join)
    }

    fn type_name(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        let mut name = None;
        let mut many = None;
        for meta in metas.iter() {
            match meta {
                Meta::Type(value) => {
                    name = Some(value);
                    many = None;
                }
                Meta::Many(per) => many = Some(per.as_ref()),
                _ => {}
            }
        }
        match (name, many) {
            (Some(name), Some(Some(per))) => write!(self.buffer, "<{name}[{per}]>"),
            (Some(name), Some(None)) => write!(self.buffer, "<{name}[]>"),
            (Some(name), _) => write!(self.buffer, "<{name}>"),
            (None, _) => Ok(()),
        }
    }

    fn help(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut has = false;
        let mut metas = metas.iter();
        let mut join = false;
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Help(value) => {
                    join = self.wrap(value, Self::HELP, join)?;
                    has = true;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(has)
    }

    fn environment(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        let mut metas = metas.iter();
        let mut join = false;
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Environment(value) => {
                    if join {
                        write!(self.buffer, ", ")?;
                    } else {
                        writeln!(self.buffer)?;
                        self.indentation()?;
                        write!(self.buffer, "~ Environment: ")?;
                        join = true;
                    }
                    write!(self.buffer, "{value}")?;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(())
    }

    fn default(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        let mut join = false;
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Default(value) => {
                    if join {
                        write!(self.buffer, ", ")?;
                    } else {
                        writeln!(self.buffer)?;
                        self.indentation()?;
                        write!(self.buffer, "~ Default: ")?;
                        join = true;
                    }
                    write!(self.buffer, "{value}")?;
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(())
    }

    fn node(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        let mut option = 0;
        let mut metas = metas.iter();
        while let Some(meta) = metas.next() {
            match meta {
                Meta::Name(value) => {
                    self.indentation()?;
                    writeln!(self.buffer, "{value}")?;
                }
                Meta::Help(value) => {
                    self.indentation()?;
                    self.wrap(value, 128, false)?;
                    writeln!(self.buffer)?;
                }
                Meta::Root(metas) => {
                    writeln!(self.buffer)?;
                    self.indent().node(metas)?;
                    writeln!(self.buffer)?;
                }
                Meta::Group(metas) => {
                    writeln!(self.buffer)?;
                    self.indent().node(metas)?;
                }
                Meta::Verb(metas) => self.indent().verb(metas)?,
                Meta::Option(metas) => {
                    if self.indent().option(metas, option)? {
                        option += 1;
                    }
                }
                Meta::Hide => hide(metas.by_ref()),
                _ => {}
            }
        }
        Ok(())
    }

    fn option(&mut self, metas: &[Meta], position: usize) -> Result<bool, fmt::Error> {
        self.indentation()?;
        let start = self.buffer.len();
        let has = {
            let start = self.buffer.len();
            let has = self.names(metas, position)?;
            let width = self.buffer.len() - start;
            let indent = max(width + Self::INDENT, Self::NAME);
            self.space(indent - width)?;
            has
        };
        let indent = self.buffer.len() - start;
        self.type_name(metas)?;
        write!(self.buffer, " ")?;

        let mut helper = self.indent_with(indent as _);
        helper.help(metas)?;
        helper.environment(metas)?;
        helper.default(metas)?;
        writeln!(helper.buffer)?;
        writeln!(helper.buffer)?;
        Ok(has)
    }

    fn verb(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        self.indentation()?;
        let start = self.buffer.len();
        self.names(metas, 0)?;
        let width = self.buffer.len() - start;
        let indent = max(width + 4, Self::NAME);
        self.space(indent - width)?;

        let mut helper = self.indent_with(indent as _);
        helper.help(metas)?;
        writeln!(self.buffer)?;
        Ok(())
    }
}

pub(crate) fn help(short: &str, long: &str, meta: &Meta) -> Option<String> {
    let mut buffer = String::new();
    let mut writer = Helper {
        short,
        long,
        buffer: &mut buffer,
        indent: -(Helper::INDENT as isize),
    };
    writer.node(from_ref(meta)).ok()?;
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
