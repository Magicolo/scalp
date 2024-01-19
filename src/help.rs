use std::{fmt::{self, Write}, slice::from_ref, borrow::Cow};

use crate::Meta;

struct Helper<'a, W> {
    write: &'a mut W,
    indent: isize,
}

struct Items<'a, 'b, W> {
    helper: &'b mut Helper<'a, W>,
    prefix: &'b str,
    separator: &'b str,
    count: usize,
}

impl<W: Write> Items<'_, '_, W> {
    pub fn item(
        &mut self,
        write: impl FnOnce(&mut Helper<W>) -> Result<(), fmt::Error>,
    ) -> Result<(), fmt::Error> {
        if self.count == 0 {
            self.helper.write.write_str(self.prefix)?;
        } else {
            self.helper.write.write_str(self.separator)?;
        }
        write(self.helper)?;
        self.count += 1;
        Ok(())
    }

    pub const fn count(&self) -> usize {
        self.count
    }
}

impl<'a, W: Write + 'a> Helper<'a, W> {
    fn items<'b>(&'b mut self, prefix: &'b str, separator: &'b str) -> Items<'a, 'b, W> {
        Items {
            helper: self,
            prefix,
            separator,
            count: 0,
        }
    }

    fn indent(&mut self) -> Helper<W> {
        Helper {
            write: self.write,
            indent: self.indent + 1,
        }
    }

    fn indentation(&mut self) -> Result<(), fmt::Error> {
        for _ in 0..self.indent {
            self.write("  ")?;
        }
        Ok(())
    }

    fn names(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut items = self.items("", ", ");
        for meta in metas {
            match meta {
                Meta::Name(value) => items.item(|help| help.write(value))?,
                Meta::Hide => break,
                _ => {}
            }
        }
        Ok(items.count() > 0)
    }

    fn type_name(&mut self, prefix: &str, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut found = None;
        for meta in metas.iter() {
            if let Meta::Type(value) = meta {
                found = Some(value)
            }
        }
        match found {
            Some(value) => {
                self.write(prefix)?;
                self.write(value)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn help(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut items = self.items("", "\n");
        for meta in metas {
            match meta {
                Meta::Help(value) => items.item(|help| help.write(value))?,
                Meta::Hide => break,
                _ => {}
            }
        }
        Ok(items.count() > 0)
    }

    fn environment(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut items = self.items("~ Environment: ", ", ");
        for meta in metas {
            match meta {
                Meta::Environment(value) => items.item(|help| help.write(value))?,
                Meta::Hide => break,
                _ => {}
            }
        }
        Ok(items.count() > 0)
    }

    fn default(&mut self, metas: &[Meta]) -> Result<bool, fmt::Error> {
        let mut items = self.items("~ Default: ", ", ");
        for meta in metas {
            match meta {
                Meta::Default(value) => items.item(|help| help.write(value))?,
                Meta::Hide => break,
                _ => {}
            }
        }
        Ok(items.count() > 0)
    }

    fn node(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        for meta in metas {
            match meta {
                Meta::Name(value) | Meta::Help(value) => {
                    self.indentation()?;
                    self.line(value)?;
                }
                Meta::Root(metas) => {
                    self.line("")?;
                    self.indent().node(metas)?;
                    self.line("")?;
                }
                Meta::Group(metas) => {
                    self.line("")?;
                    self.indent().node(metas)?;
                }
                Meta::Verb(metas) => self.indent().verb(metas)?,
                Meta::Option(metas) => self.indent().option(metas)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn write(&mut self, value: &str) -> Result<(), fmt::Error> {
        self.write.write_str(value)
    }

    fn line(&mut self, line: &str) -> Result<(), fmt::Error> {
        self.write(line)?;
        self.write.write_char('\n')?;
        Ok(())
    }

    fn option(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        self.indentation()?;
        self.names(metas)?;
        self.type_name(": ", metas)?;

        let mut helper = self.indent();
        helper.line("")?;
        helper.indentation()?;

        if helper.help(metas)? {
            helper.line("")?;
            helper.indentation()?;
        }

        if helper.environment(metas)? {
            helper.line("")?;
            helper.indentation()?;
        }

        if helper.default(metas)? {
            helper.line("")?;
            helper.indentation()?;
        }

        helper.line("")?;
        Ok(())
    }

    fn verb(&mut self, metas: &[Meta]) -> Result<(), fmt::Error> {
        self.indentation()?;
        self.names(metas)?;
        let mut helper = self.indent();
        helper.line("")?;
        helper.indentation()?;

        if helper.help(metas)? {
            helper.line("")?;
            helper.indentation()?;
        }

        helper.line("")?;
        Ok(())
    }
}

pub(crate) fn help(meta: &Meta) -> Option<String> {
    let mut buffer = String::new();
    let mut writer = Helper {
        write: &mut buffer,
        indent: -1,
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
