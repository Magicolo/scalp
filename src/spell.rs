use std::{mem::swap, ops::Deref};

pub struct Spell(Vec<usize>, Vec<usize>);

impl Spell {
    pub const fn new() -> Self {
        Self(Vec::new(), Vec::new())
    }

    pub fn suggest<T: Deref<Target = str>>(
        &mut self,
        word: &str,
        dictionary: impl IntoIterator<Item = T>,
        maximum: usize,
    ) -> Vec<(T, usize)> {
        let dictionary = dictionary.into_iter();
        let mut results = Vec::with_capacity(dictionary.size_hint().0);
        for candidate in dictionary {
            let distance = self.distance(word.as_bytes(), candidate.as_bytes());
            if distance < maximum {
                results.push((candidate, distance));
            }
        }
        results.sort_by_key(|&(_, distance)| distance);
        results
    }

    fn distance(&mut self, left: &[u8], right: &[u8]) -> usize {
        let left_count = left.len();
        let right_count = right.len();
        if left_count > right_count {
            return self.distance(right, left);
        }

        let Self(previous, current) = self;
        previous.clear();
        previous.resize(left_count + 1, 0);
        current.resize(left_count + 1, 0);

        for i in 1..=right_count {
            current[0] = i;
            for j in 1..=left_count {
                let left = char::from(left[j - 1]).to_ascii_lowercase();
                let right = char::from(right[i - 1]).to_ascii_lowercase();
                let insert = current[j - 1] + 1;
                let delete = previous[j] + 1;
                let replace = previous[j - 1] + if left == right { 0 } else { 1 };
                current[j] = insert.min(delete).min(replace);
            }
            swap(previous, current);
        }

        previous[left_count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        assert_eq!(Spell::new().distance(b"boba", b"boba"), 0);
        assert_eq!(Spell::new().distance(b"boba", b"bobo"), 1);
        assert_eq!(Spell::new().distance(b"boba", b"bobba"), 1);
        assert_eq!(Spell::new().distance(b"boba", b"boa"), 1);
        assert_eq!(Spell::new().distance(b"boba", b"fett"), 4);
    }

    #[test]
    fn best() {
        let best = Spell::new().suggest(
            "poulaye",
            [
                "poullayye",
                "poupou",
                "pilaye",
                "poulah",
                "piulaye",
                "paliyoo",
                "vladimarre",
                "poulaye",
                "p",
                "poullayye",
                "poulay",
            ],
            5,
        );
        println!("{best:?}");
    }
}
