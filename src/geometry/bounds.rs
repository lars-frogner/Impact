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
