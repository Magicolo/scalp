use super::Meta;
use std::ops::{Deref, DerefMut};

pub trait Scope: Into<Meta> + Extend<Meta> + DerefMut<Target = Vec<Meta>> {
    fn push(&mut self, meta: Meta) {
        self.extend([meta]);
    }
}

pub trait Version: Scope {}
pub trait Node: Scope {}

macro_rules! scope {
    ($name: ident) => {
        #[derive(Default)]
        pub struct $name(Vec<Meta>);

        impl $name {
            pub const fn new() -> Self {
                Self(Vec::new())
            }
        }

        impl Scope for $name {}

        impl From<$name> for Meta {
            fn from(scope: $name) -> Self {
                Meta::$name(scope.0)
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

scope!(Root);
scope!(Option);
scope!(Group);
scope!(Verb);

impl Version for Root {}
impl Version for Verb {}
impl Node for Root {}
impl Node for Group {}
impl Node for Verb {}
