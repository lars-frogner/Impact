#[derive(Clone, Debug)]
pub struct StaticList<T, const N: usize>(pub [Option<T>; N]);

impl<T, const N: usize> StaticList<T, N> {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.first().is_none_or(|first| first.is_none())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StaticList<T, N> {
    type Item = &'a T;
    type IntoIter =
        std::iter::FilterMap<std::slice::Iter<'a, Option<T>>, fn(&'a Option<T>) -> Option<&'a T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.as_slice().iter().filter_map(Option::as_ref)
    }
}

impl<T, const N: usize> Default for StaticList<T, N> {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
    }
}
