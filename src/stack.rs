pub trait Push<T> {
    type Output;
    fn push(self, item: T) -> Self::Output;
}

pub trait Pop {
    type Item;
    type Output;
    fn pop(self) -> (Self::Item, Self::Output);
}

pub trait Count {
    const COUNT: usize;
}

macro_rules! stack {
    () => {
        impl Count for () {
            const COUNT: usize = 0;
        }

        impl Pop for () {
            type Item = ();
            type Output = ();

            #[inline]
            fn pop(self) -> ((), ()) {
                ((), ())
            }
        }
    };
    ($head: ident $(, $tail: ident)*) => {
        impl<$head $(, $tail)*> Push<$head> for ($($tail,)*) {
            type Output = ($($tail,)* $head,);

            #[inline]
            fn push(self, item: $head) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($tail,)*) = self;
                ($($tail,)* item,)
            }
        }

        impl<$head $(, $tail)*> Pop for ($($tail,)* $head,) {
            type Item = $head;
            type Output = ($($tail,)*);

            #[inline]
            fn pop(self) -> (Self::Item, Self::Output) {
                #[allow(non_snake_case)]
                let ($($tail,)* $head,) = self;
                ($head, ($($tail,)*))
            }
        }

        impl<$head $(, $tail)*> Count for ($($tail,)* $head,) where ($($tail,)*): Count {
            const COUNT: usize = 1 + <($($tail,)*) as Count>::COUNT;
        }

        stack!($($tail),*);
    };
}

stack!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39,
    T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58,
    T59, T60, T61, T62, T63
);
