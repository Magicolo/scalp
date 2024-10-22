pub use checkito::*;
pub use scalp::*;

pub fn case() -> impl Generate<Item = Case> {
    (
        same(Case::Same),
        same(Case::Lower),
        same(Case::Upper),
        same(Case::Pascal),
        same(Case::Camel),
        bool::generator().map(|upper| Case::Kebab { upper }),
        bool::generator().map(|upper| Case::Snake { upper }),
        <(bool, char)>::generator().map(|(upper, separator)| Case::Separate { separator, upper }),
    )
        .any()
        .unify::<Case>()
}
