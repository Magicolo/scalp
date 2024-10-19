use crate::meta::Meta;
use core::ops::{Deref, DerefMut};

pub trait Scope: Into<Meta> + Extend<Meta> + DerefMut<Target = Vec<Meta>> {
    fn push(&mut self, meta: Meta) {
        self.extend([meta]);
    }
}

pub trait Node: Scope {}

macro_rules! scope {
    ($visibility: vis, $name: ident, $meta: ident) => {
        #[derive(Default)]
        $visibility struct $name(Vec<Meta>);

        impl $name {
            $visibility const fn new() -> Self {
                Self(Vec::new())
            }
        }

        impl Scope for $name {}

        impl From<$name> for Meta {
            fn from(scope: $name) -> Self {
                Meta::$meta(scope.0)
            }
        }

        impl Extend<Meta> for $name {
            fn extend<T: IntoIterator<Item = Meta>>(&mut self, iter: T) {
                self.0.extend(iter);
            }
        }

        impl Deref for $name {
            type Target = Vec<Meta>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

scope!(pub(crate), Root, Group);
scope!(pub, Option, Option);
scope!(pub, Group, Group);
scope!(pub, Verb, Verb);

impl Node for Root {}
impl Node for Group {}
impl Node for Verb {}
