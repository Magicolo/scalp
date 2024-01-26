pub trait Stack {
    const COUNT: usize;
    type Push<T>: Stack;
    type Pop: Stack;
    type Clear: Stack;
    type Item;

    fn push<T>(self, item: T) -> Self::Push<T>;
    fn pop(self) -> (Self::Item, Self::Pop);
    fn clear(self) -> Self::Clear;
}

pub struct Overflow<T>(T);

impl Stack for () {
    const COUNT: usize = 0;
    type Push<T> = (T,);
    type Pop = ();
    type Clear = ();
    type Item = ();

    #[inline]
    fn push<T>(self, item: T) -> Self::Push<T> {
        (item,)
    }

    #[inline]
    fn pop(self) -> (Self::Item, Self::Pop) {
        ((), ())
    }

    #[inline]
    fn clear(self) -> Self::Clear {}
}

impl<T: Stack> Stack for Overflow<T> {
    const COUNT: usize = T::COUNT;
    type Push<U> = Overflow<T>;
    type Pop = T::Pop;
    type Clear = T::Clear;
    type Item = T::Item;

    #[inline]
    fn push<U>(self, _: U) -> Self::Push<T> {
        self
    }

    #[inline]
    fn pop(self) -> (Self::Item, Self::Pop) {
        self.0.pop()
    }

    #[inline]
    fn clear(self) -> Self::Clear {
        self.0.clear()
    }
}

macro_rules! stack {
    (@inner) => { };
    ($tail: ident $(, $head: ident)*) => {
        impl<$tail, $($head,)*> Stack for ($($head,)* $tail,) {
            const COUNT: usize = 1 + <($($head,)*) as Stack>::COUNT;
            type Push<T> = Overflow<Self>;
            type Pop = ($($head,)*);
            type Clear = ();
            type Item = $tail;

            #[inline]
            fn push<T>(self, _: T) -> Self::Push<T> {
                Overflow(self)
            }

            #[inline]
            fn pop(self) -> (Self::Item, Self::Pop) {
                #[allow(non_snake_case)]
                let ($($head,)* $tail,) = self;
                ($tail, ($($head,)*))
            }

            #[inline]
            fn clear(self) -> Self::Clear { }
        }

        stack!(@inner $($head),*);
    };
    (@inner $tail: ident $(, $head: ident)*) => {
        impl<$tail, $($head,)*> Stack for ($($head,)* $tail,) {
            const COUNT: usize = 1 + <($($head,)*) as Stack>::COUNT;
            type Push<T> = ($($head,)* $tail, T,);
            type Pop = ($($head,)*);
            type Clear = ();
            type Item = $tail;

            #[inline]
            fn push<T>(self, item: T) -> Self::Push<T> {
                #[allow(non_snake_case)]
                let ($($head,)* $tail,) = self;
                ($($head,)* $tail, item,)
            }

            #[inline]
            fn pop(self) -> (Self::Item, Self::Pop) {
                #[allow(non_snake_case)]
                let ($($head,)* $tail,) = self;
                ($tail, ($($head,)*))
            }

            #[inline]
            fn clear(self) -> Self::Clear { }
        }

        stack!(@inner $($head),*);
    };
}

stack!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39,
    T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58,
    T59, T60, T61, T62, T63
);
