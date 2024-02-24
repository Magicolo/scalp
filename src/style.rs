use self::color::*;
use std::{
    borrow::Cow,
    fmt::{self, Display},
};
use termion::{
    color::{Bg, Color, Fg, Rgb},
    style::{Bold, Faint, Italic, Reset, Underline},
    terminal_size,
};

pub struct Default;
pub struct Plain;

#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum Item {
    Head,
    Bar(Line),
    Arrow(Line),
    Version,
    Description,
    Author,
    Help,
    Group,
    Verb,
    Type,
    Option,
    Usage,
    Note,
    Link,
    Summary,
    Tag,
}

#[derive(Clone, Copy)]
pub enum Line {
    Head,
    Description,
    Link,
    Usage,
}

pub trait Format {
    fn width(&self) -> usize;
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result;
}

macro_rules! format {
    ($type: ty, $count: expr) => {
        impl Format for $type {
            #[inline]
            fn width(&self) -> usize {
                $count
            }

            #[inline]
            fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(self, formatter)
            }
        }
    };
}

macro_rules! tuple {
    ($or: ident $(, $name: ident, $index: tt)*) => {
        impl<$($name: Format),*> Format for ($($name,)*) {
            #[inline]
            fn width(&self) -> usize {
                $(self.$index.width() + )* 0
            }

            #[inline]
            fn format(&self, _formatter: &mut fmt::Formatter) -> fmt::Result {
                $(self.$index.format(_formatter)?;)*
                Ok(())
            }
        }

        impl<$($name: Format),*> Format for orn::$or<$($name,)*> {
            #[inline]
            fn width(&self) -> usize {
                match self {
                    $(orn::$or::$name(value) => value.width(),)*
                    #[allow(unreachable_patterns)]
                    _ => 0,
                }
            }

            #[inline]
            fn format(&self, _formatter: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(orn::$or::$name(value) => value.format(_formatter),)*
                    #[allow(unreachable_patterns)]
                    _ => Ok(()),
                }
            }
        }
    };
}

tuple!(Or0);
tuple!(Or1, T0, 0);
tuple!(Or2, T0, 0, T1, 1);
tuple!(Or3, T0, 0, T1, 1, T2, 2);
tuple!(Or4, T0, 0, T1, 1, T2, 2, T3, 3);
tuple!(Or5, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4);
tuple!(Or6, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5);
tuple!(Or7, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6);
tuple!(Or8, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7);
tuple!(Or9, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8);
tuple!(Or10, T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9);

impl<T: Format + ?Sized> Format for &T {
    #[inline]
    fn width(&self) -> usize {
        T::width(self)
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        T::format(self, formatter)
    }
}

impl<T: Format> Format for [T] {
    #[inline]
    fn width(&self) -> usize {
        self.iter().map(Format::width).sum()
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for item in self {
            item.format(formatter)?;
        }
        Ok(())
    }
}

impl<T: Format, const N: usize> Format for [T; N] {
    #[inline]
    fn width(&self) -> usize {
        self.iter().map(Format::width).sum()
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for item in self {
            item.format(formatter)?;
        }
        Ok(())
    }
}

impl Format for Cow<'_, str> {
    #[inline]
    fn width(&self) -> usize {
        self.chars().count()
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self)
    }
}

impl Format for String {
    #[inline]
    fn width(&self) -> usize {
        self.chars().count()
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self)
    }
}

impl Format for str {
    #[inline]
    fn width(&self) -> usize {
        self.chars().count()
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self)
    }
}

impl<C: Color> Format for Fg<C> {
    #[inline]
    fn width(&self) -> usize {
        0
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

impl<C: Color> Format for Bg<C> {
    #[inline]
    fn width(&self) -> usize {
        0
    }

    #[inline]
    fn format(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.fmt(formatter)
    }
}

format!(char, 1);
format!(Reset, 0);
format!(Bold, 0);
format!(Faint, 0);
format!(Italic, 0);
format!(Underline, 0);

pub trait Style {
    fn indent(&self) -> usize;
    fn width(&self) -> usize;
    fn begin(&self, item: Item) -> &dyn Format;
    fn end(&self, item: Item) -> &dyn Format;
}

macro_rules! dynamic {
    ($($value: expr),*) => {
        &[$(&$value as &dyn Format),*]
    };
}
impl Style for Default {
    #[inline]
    fn indent(&self) -> usize {
        2
    }

    #[inline]
    fn width(&self) -> usize {
        terminal_size().map_or(64, |pair| pair.0 as usize - 25)
    }

    #[inline]
    fn begin(&self, item: Item) -> &dyn Format {
        const BAR: char = '│';
        const ARROW: char = '>';
        const HEAD: Fg<Rgb> = Fg(RUBY_RED);
        const DESCRIPTION: Fg<Rgb> = Fg(SALMON_PINK);
        const LINK: Fg<Rgb> = Fg(SALMON_PINK);
        const USAGE: Fg<Rgb> = Fg(VIOLET);

        match item {
            Item::Head => dynamic!(Bold, HEAD),
            Item::Bar(Line::Head) => dynamic!(HEAD, BAR),
            Item::Bar(Line::Description) => dynamic!(DESCRIPTION, BAR),
            Item::Bar(Line::Link) => dynamic!(LINK, BAR),
            Item::Bar(Line::Usage) => dynamic!(USAGE, BAR),
            Item::Arrow(Line::Head) => dynamic!(HEAD, ARROW),
            Item::Arrow(Line::Description) => dynamic!(DESCRIPTION, ARROW),
            Item::Arrow(Line::Link) => dynamic!(LINK, ARROW),
            Item::Arrow(Line::Usage) => dynamic!(USAGE, ARROW),
            Item::Version => dynamic!(Bold, Fg(RUBY_RED)),
            Item::Author => dynamic!(Faint, Italic, HEAD),
            Item::Description => dynamic!(DESCRIPTION),
            Item::Help => dynamic!(Fg(PEACH)),
            Item::Group => dynamic!(Bold, Fg(OCEAN_BLUE)),
            Item::Verb => dynamic!(Bold, Fg(TURQUOISE)),
            Item::Option => dynamic!(Fg(TURQUOISE)),
            Item::Type => dynamic!(Faint, Fg(TURQUOISE), '<'),
            Item::Usage => dynamic!(Underline, USAGE),
            Item::Link => dynamic!(Italic, LINK),
            Item::Note => dynamic!(Italic, Fg(SILVER_GRAY)),
            Item::Summary => dynamic!(Fg(SANDY_BROWN)),
            Item::Tag => dynamic!(Faint, Fg(CORAL_PINK), '['),
        }
    }

    #[inline]
    fn end(&self, item: Item) -> &dyn Format {
        match item {
            Item::Tag => dynamic!(']', Reset),
            Item::Type => dynamic!('>', Reset),
            _ => dynamic!(Reset),
        }
    }
}

impl Style for Plain {
    #[inline]
    fn indent(&self) -> usize {
        2
    }

    #[inline]
    fn width(&self) -> usize {
        96
    }

    #[inline]
    fn begin(&self, item: Item) -> &dyn Format {
        const BAR: char = '~';
        const ARROW: &str = "  ";

        match item {
            Item::Bar(Line::Head) => dynamic!(BAR),
            Item::Bar(Line::Description) => dynamic!(BAR),
            Item::Bar(Line::Link) => dynamic!(BAR),
            Item::Bar(Line::Usage) => dynamic!(BAR),
            Item::Arrow(Line::Head) => dynamic!(ARROW),
            Item::Arrow(Line::Description) => dynamic!(ARROW),
            Item::Arrow(Line::Link) => dynamic!(ARROW),
            Item::Arrow(Line::Usage) => dynamic!(ARROW),
            Item::Type => dynamic!('<'),
            Item::Tag => dynamic!('['),
            _ => dynamic!(""),
        }
    }

    #[inline]
    fn end(&self, item: Item) -> &dyn Format {
        match item {
            Item::Tag => dynamic!(']'),
            Item::Type => dynamic!('>'),
            _ => dynamic!(""),
        }
    }
}

pub mod color {
    use super::*;

    pub const OCEAN_BLUE: Rgb = Rgb(36, 113, 163);
    pub const TURQUOISE: Rgb = Rgb(64, 224, 208);
    pub const RUBY_RED: Rgb = Rgb(220, 20, 60);
    pub const SILVER_GRAY: Rgb = Rgb(169, 169, 169);
    pub const CORAL_PINK: Rgb = Rgb(255, 127, 80);
    pub const VIOLET: Rgb = Rgb(238, 130, 238);
    pub const PEACH: Rgb = Rgb(255, 218, 185);
    pub const SALMON_PINK: Rgb = Rgb(255, 145, 164);
    pub const SANDY_BROWN: Rgb = Rgb(244, 164, 96);
    pub const SUNFLOWER_YELLOW: Rgb = Rgb(255, 255, 85);
    pub const MANGO_ORANGE: Rgb = Rgb(255, 179, 71);
    pub const SEAFOAM_GREEN: Rgb = Rgb(50, 205, 153);
    pub const COBALT_BLUE: Rgb = Rgb(0, 71, 171);
    pub const SLATE_GRAY: Rgb = Rgb(112, 128, 144);
    pub const LAVENDER: Rgb = Rgb(230, 230, 250);
    pub const BURGUNDY: Rgb = Rgb(128, 0, 32);
    pub const SUNSET_ORANGE: Rgb = Rgb(255, 140, 79);
    pub const GOLDEN_YELLOW: Rgb = Rgb(255, 223, 0);
    pub const SKY_BLUE: Rgb = Rgb(135, 206, 250);
    pub const EARTH_BROWN: Rgb = Rgb(139, 69, 19);
    pub const MINT_GREEN: Rgb = Rgb(152, 255, 152);
    pub const MAUVE: Rgb = Rgb(224, 176, 255);
    pub const EMERALD_GREEN: Rgb = Rgb(0, 158, 96);
    pub const AMETHYST_PURPLE: Rgb = Rgb(138, 43, 226);
    pub const GOLDENROD_YELLOW: Rgb = Rgb(218, 165, 32);
    pub const LIME_GREEN: Rgb = Rgb(50, 205, 50);
    pub const TEAL: Rgb = Rgb(0, 128, 128);
    pub const SAFFRON_YELLOW: Rgb = Rgb(244, 196, 48);
    pub const INDIGO: Rgb = Rgb(75, 0, 130);
    pub const AQUA: Rgb = Rgb(0, 255, 255);
    pub const FOREST_GREEN: Rgb = Rgb(34, 139, 34);
    pub const STEEL_BLUE: Rgb = Rgb(70, 130, 180);
    pub const CHOCOLATE_BROWN: Rgb = Rgb(139, 69, 19);
    pub const CORNFLOWER_BLUE: Rgb = Rgb(100, 149, 237);
    pub const OLIVE_GREEN: Rgb = Rgb(128, 128, 0);
}
