//! Lower and upper bounds.

/// Inclusive lower and upper bounds.
#[derive(Clone, Debug)]
pub struct InclusiveBounds<T> {
    lower: T,
    upper: T,
}

/// A lower inclusive and upper exlusive set of bounds.
#[derive(Clone, Debug)]
pub struct UpperExclusiveBounds<T> {
    lower: T,
    upper: T,
}

pub trait Bounds<T>
where
    T: Copy + PartialOrd,
{
    /// Returns the lower bound.
    fn lower(&self) -> T;

    /// Returns the upper bound.
    fn upper(&self) -> T;

    /// Whether the given value is contained within the bounds.
    fn contain(&self, value: T) -> bool;

    /// Returns the lower and upper bound in a tuple.
    fn bounds(&self) -> (T, T) {
        (self.lower(), self.upper())
    }
}

impl<T> InclusiveBounds<T>
where
    T: PartialOrd + std::fmt::Debug,
{
    /// Creates a new set of inclusive bounds.
    ///
    /// # Panics
    /// If the given upper bound is smaller than the lower bound.
    pub fn new(lower: T, upper: T) -> Self {
        assert!(
            upper >= lower,
            "Upper bound ({:?}) is smaller than lower bound ({:?})",
            &upper,
            &lower
        );
        Self { lower, upper }
    }
}

impl<T> Bounds<T> for InclusiveBounds<T>
where
    T: Copy + PartialOrd,
{
    fn lower(&self) -> T {
        self.lower
    }

    fn upper(&self) -> T {
        self.upper
    }

    fn contain(&self, value: T) -> bool {
        value >= self.lower() && value <= self.upper()
    }
}

impl<T> UpperExclusiveBounds<T>
where
    T: PartialOrd + std::fmt::Debug,
{
    /// Creates a new set of upper exclusive bounds.
    ///
    /// # Panics
    /// If the given upper bound is not larger than the lower bound.
    pub fn new(lower: T, upper: T) -> Self {
        assert!(
            upper > lower,
            "Upper bound ({:?}) does not exceed lower bound ({:?})",
            &upper,
            &lower
        );
        Self { lower, upper }
    }
}

impl<T> Bounds<T> for UpperExclusiveBounds<T>
where
    T: Copy + PartialOrd,
{
    fn lower(&self) -> T {
        self.lower
    }

    fn upper(&self) -> T {
        self.upper
    }

    fn contain(&self, value: T) -> bool {
        value >= self.lower() && value < self.upper()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn constructing_valid_inclusive_bounds_succeeds() {
        InclusiveBounds::new(42.0, 42.0);
    }

    #[test]
    #[should_panic]
    fn constructing_invalid_inclusive_bounds() {
        InclusiveBounds::new(42.0, 41.9999);
    }

    #[test]
    fn inclusive_bounds_contain_inside_values() {
        let bounds = InclusiveBounds::new(42.0, 43.0);
        assert!(bounds.contain(42.5));
        assert!(bounds.contain(42.0));
        assert!(bounds.contain(43.0));
    }

    #[test]
    fn inclusive_bounds_dont_contain_outside_values() {
        let bounds = InclusiveBounds::new(42.0, 43.0);
        assert!(!bounds.contain(41.9999));
        assert!(!bounds.contain(43.0001));
    }

    #[test]
    fn constructing_valid_upper_exclusive_bounds_succeeds() {
        UpperExclusiveBounds::new(42.0, 42.0001);
    }

    #[test]
    #[should_panic]
    fn constructing_invalid_upper_exclusive_bounds() {
        UpperExclusiveBounds::new(42.0, 42.0);
    }

    #[test]
    fn upper_exclusive_bounds_contain_inside_values() {
        let bounds = UpperExclusiveBounds::new(42.0, 43.0);
        assert!(bounds.contain(42.5));
        assert!(bounds.contain(42.0));
    }

    #[test]
    fn upper_exclusive_bounds_dont_contain_outside_values() {
        let bounds = UpperExclusiveBounds::new(42.0, 43.0);
        assert!(!bounds.contain(43.5));
        assert!(!bounds.contain(41.5));
        assert!(!bounds.contain(43.0));
    }
}
