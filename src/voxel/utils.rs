//! Utilities for working with voxels.

use std::ops::Range;

/// A 3D spatial dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    X = 0,
    Y = 1,
    Z = 2,
}

/// A side (e.g. a specific side of a chunk along some dimension).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Lower,
    Upper,
}

/// Helper for iterating over part of a 3D grid of elements, with `N` elements
/// along each dimension.
#[derive(Clone, Debug)]
pub struct Loop3<const N: usize> {
    i_range: Range<usize>,
    j_range: Range<usize>,
    k_range: Range<usize>,
    move_j_loop_out: bool,
    move_k_loop_out: bool,
}

/// A [`Loop3`] with a flat slice containing a data value for each grid
/// location, laid out so that the linear index varies fastest with k, then j,
/// then i.
#[derive(Clone, Debug)]
pub struct DataLoop3<'a, 'b, T, const N: usize> {
    lp: &'a Loop3<N>,
    data: &'b [T],
}

/// A [`Loop3`] with a flat mutable slice containing a data value for each grid
/// location, laid out so that the linear index varies fastest with k, then j,
/// then i.
#[derive(Debug)]
pub struct MutDataLoop3<'a, 'b, T, const N: usize> {
    lp: &'a Loop3<N>,
    data: &'b mut [T],
}

impl Dimension {
    /// Returns the index of the dimension (`0` for `X`, `1` for `Y` and `2` for
    /// `Z`).
    pub const fn idx(self) -> usize {
        self as usize
    }
}

impl Side {
    /// Returns the index of the side (`0` for `Lower`, `1` for `Upper`).
    pub const fn idx(self) -> usize {
        self as usize
    }

    /// Returns the opposite `Side`.
    pub const fn opposite(self) -> Self {
        match self {
            Self::Lower => Self::Upper,
            Self::Upper => Self::Lower,
        }
    }

    /// Returns a [`Range`] over a single index on the relevant side of a
    /// collection of `N` elements (`0` for the lower side and `N - 1` for the
    /// upper side).
    pub const fn as_range<const N: usize>(self) -> Range<usize> {
        match self {
            Self::Lower => 0..1,
            Self::Upper => N - 1..N,
        }
    }
}

impl<const N: usize> Loop3<N> {
    /// Creates a loop over all `N^3` grid locations.
    pub const fn over_all() -> Self {
        Self {
            i_range: 0..N,
            j_range: 0..N,
            k_range: 0..N,
            move_j_loop_out: false,
            move_k_loop_out: false,
        }
    }

    /// Creates a loop over all `(N-2)^3` interior grid locations.
    pub const fn over_interior() -> Self {
        Self {
            i_range: Self::interior_range(),
            j_range: Self::interior_range(),
            k_range: Self::interior_range(),
            move_j_loop_out: false,
            move_k_loop_out: false,
        }
    }

    /// Creates a loop over one full face of the grid (`N^2` locations),
    /// specified by a dimension and side.
    pub const fn over_face(dim: Dimension, side: Side) -> Self {
        Self::range_over_face(dim, side, Self::full_range())
    }

    /// Creates a loop over the interior of a face of the grid (`(N-2)^2`
    /// locations), specified by a dimension and side.
    pub const fn over_face_interior(dim: Dimension, side: Side) -> Self {
        Self::range_over_face(dim, side, Self::interior_range())
    }

    /// Creates a loop over one full edge of the grid (`N` locations), specified
    /// by the dimension and side of the face holding the edge, and the edge's
    /// side on the face along a secondary dimension, which is the dimension
    /// following the face dimension in the `X -> Y -> Z -> X` cycle.
    pub const fn over_edge(face_dim: Dimension, face_side: Side, secondary_side: Side) -> Self {
        Self::range_over_edge(face_dim, face_side, secondary_side, Self::full_range())
    }

    /// Creates a loop over the interior of an edge of the grid (`N-2`
    /// locations), specified by the dimension and side of the face holding
    /// the edge, and the edge's side on the face along a secondary
    /// dimension, which is the dimension following the face dimension in
    /// the `X -> Y -> Z -> X` cycle.
    pub const fn over_edge_interior(
        face_dim: Dimension,
        face_side: Side,
        secondary_side: Side,
    ) -> Self {
        Self::range_over_edge(face_dim, face_side, secondary_side, Self::interior_range())
    }

    /// Creates a single-iteration loop over a corner of the grid specified by
    /// a side along the `X`, `Y` and `Z` dimensions.
    pub const fn over_corner(x_side: Side, y_side: Side, z_side: Side) -> Self {
        Self {
            i_range: x_side.as_range::<N>(),
            j_range: y_side.as_range::<N>(),
            k_range: z_side.as_range::<N>(),
            move_j_loop_out: false,
            move_k_loop_out: false,
        }
    }

    /// Creates 6 loops together covering the full boundary of the grid (no
    /// locations are iterated over more than once).
    pub const fn over_full_boundary() -> [Self; 6] {
        [
            Self {
                i_range: Side::Lower.as_range::<N>(),
                j_range: Self::full_range(),
                k_range: Self::full_range(),
                move_j_loop_out: false,
                move_k_loop_out: false,
            },
            Self {
                i_range: Side::Upper.as_range::<N>(),
                j_range: Self::full_range(),
                k_range: Self::full_range(),
                move_j_loop_out: false,
                move_k_loop_out: false,
            },
            Self {
                i_range: Self::interior_range(),
                j_range: Side::Lower.as_range::<N>(),
                k_range: Self::full_range(),
                move_j_loop_out: true,
                move_k_loop_out: false,
            },
            Self {
                i_range: Self::interior_range(),
                j_range: Side::Upper.as_range::<N>(),
                k_range: Self::full_range(),
                move_j_loop_out: true,
                move_k_loop_out: false,
            },
            Self {
                i_range: Self::interior_range(),
                j_range: Self::interior_range(),
                k_range: Side::Lower.as_range::<N>(),
                move_j_loop_out: false,
                move_k_loop_out: true,
            },
            Self {
                i_range: Self::interior_range(),
                j_range: Self::interior_range(),
                k_range: Side::Upper.as_range::<N>(),
                move_j_loop_out: false,
                move_k_loop_out: true,
            },
        ]
    }

    const fn range_over_face(dim: Dimension, side: Side, range: Range<usize>) -> Self {
        match dim {
            Dimension::X => Self {
                i_range: side.as_range::<N>(),
                j_range: range.start..range.end,
                k_range: range.start..range.end,
                move_j_loop_out: false,
                move_k_loop_out: false,
            },
            Dimension::Y => Self {
                i_range: range.start..range.end,
                j_range: side.as_range::<N>(),
                k_range: range.start..range.end,
                move_j_loop_out: true,
                move_k_loop_out: false,
            },
            Dimension::Z => Self {
                i_range: range.start..range.end,
                j_range: range.start..range.end,
                k_range: side.as_range::<N>(),
                move_j_loop_out: false,
                move_k_loop_out: true,
            },
        }
    }

    const fn range_over_edge(
        face_dim: Dimension,
        face_side: Side,
        secondary_side: Side,
        range: Range<usize>,
    ) -> Self {
        match face_dim {
            Dimension::X => Self {
                i_range: face_side.as_range::<N>(),
                j_range: secondary_side.as_range::<N>(),
                k_range: range.start..range.end,
                move_j_loop_out: false,
                move_k_loop_out: false,
            },
            Dimension::Y => Self {
                i_range: range.start..range.end,
                j_range: face_side.as_range::<N>(),
                k_range: secondary_side.as_range::<N>(),
                move_j_loop_out: true,
                move_k_loop_out: true,
            },
            Dimension::Z => Self {
                i_range: secondary_side.as_range::<N>(),
                j_range: range.start..range.end,
                k_range: face_side.as_range::<N>(),
                move_j_loop_out: true,
                move_k_loop_out: false,
            },
        }
    }

    const fn full_range() -> Range<usize> {
        0..N
    }

    const fn interior_range() -> Range<usize> {
        1..N - 1
    }

    /// Returns the range of indices for the x-dimension.
    #[inline(always)]
    pub const fn i_range(&self) -> Range<usize> {
        self.i_range.start..self.i_range.end
    }

    /// Returns the range of indices for the y-dimension.
    #[inline(always)]
    pub const fn j_range(&self) -> Range<usize> {
        self.j_range.start..self.j_range.end
    }

    /// Returns the range of indices for the z-dimension.
    #[inline(always)]
    pub const fn k_range(&self) -> Range<usize> {
        self.k_range.start..self.k_range.end
    }

    /// Returns the total number of iterations in the loop.
    pub const fn n_iterations(&self) -> usize {
        (self.i_range.end - self.i_range.start)
            * (self.j_range.end - self.j_range.start)
            * (self.k_range.end - self.k_range.start)
    }

    /// Returns the maximum linear index for any loop iteration.
    pub const fn max_linear_idx(&self) -> usize {
        Self::linear_idx(
            self.i_range.end - 1,
            self.j_range.end - 1,
            self.k_range.end - 1,
        )
    }

    /// Returns the linear index for the given 3D indices.
    pub const fn linear_idx(i: usize, j: usize, k: usize) -> usize {
        i * (N * N) + j * N + k
    }

    /// Executes the given closure for each iteration in the loop, passing in
    /// the 3D indices of the iteration.
    #[inline(always)]
    pub fn execute(&self, f: &mut impl FnMut(usize, usize, usize)) {
        match (self.move_j_loop_out, self.move_k_loop_out) {
            (false, false) => {
                for i in self.i_range() {
                    for j in self.j_range() {
                        for k in self.k_range() {
                            f(i, j, k);
                        }
                    }
                }
            }
            (true, false) => {
                for j in self.j_range() {
                    for i in self.i_range() {
                        for k in self.k_range() {
                            f(i, j, k);
                        }
                    }
                }
            }
            (false, true) => {
                for k in self.k_range() {
                    for i in self.i_range() {
                        for j in self.j_range() {
                            f(i, j, k);
                        }
                    }
                }
            }
            (true, true) => {
                for j in self.j_range() {
                    for k in self.k_range() {
                        for i in self.i_range() {
                            f(i, j, k);
                        }
                    }
                }
            }
        }
    }

    /// Executes the given closure for each iteration in the loop, passing in
    /// the 3D indices and the linear index of the iteration.
    #[inline(always)]
    pub fn execute_with_linear_idx(&self, f: &mut impl FnMut(&[usize; 3], usize)) {
        match (self.move_j_loop_out, self.move_k_loop_out) {
            (false, false) => {
                for i in self.i_range() {
                    for j in self.j_range() {
                        for k in self.k_range() {
                            f(&[i, j, k], Self::linear_idx(i, j, k));
                        }
                    }
                }
            }
            (true, false) => {
                for j in self.j_range() {
                    for i in self.i_range() {
                        for k in self.k_range() {
                            f(&[i, j, k], Self::linear_idx(i, j, k));
                        }
                    }
                }
            }
            (false, true) => {
                for k in self.k_range() {
                    for i in self.i_range() {
                        for j in self.j_range() {
                            f(&[i, j, k], Self::linear_idx(i, j, k));
                        }
                    }
                }
            }
            (true, true) => {
                for j in self.j_range() {
                    for k in self.k_range() {
                        for i in self.i_range() {
                            f(&[i, j, k], Self::linear_idx(i, j, k));
                        }
                    }
                }
            }
        }
    }

    /// Iterates over this loop in tandem with the given other loop, executing
    /// the given closure with the 3D indices in each loop for each iteration.
    ///
    /// # Panics
    /// If the number of iterations in the two loops is not equal.
    #[inline(always)]
    pub fn zip_execute<const M: usize>(
        &self,
        other: &Loop3<M>,
        f: &mut impl FnMut(&[usize; 3], &[usize; 3]),
    ) {
        assert_eq!(self.n_iterations(), other.n_iterations());
        match (self.move_j_loop_out, self.move_k_loop_out) {
            (false, false) => {
                for (i0, i1) in self.i_range().zip(other.i_range()) {
                    for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                        for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                            f(&[i0, j0, k0], &[i1, j1, k1]);
                        }
                    }
                }
            }
            (true, false) => {
                for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                    for (i0, i1) in self.i_range().zip(other.i_range()) {
                        for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                            f(&[i0, j0, k0], &[i1, j1, k1]);
                        }
                    }
                }
            }
            (false, true) => {
                for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                    for (i0, i1) in self.i_range().zip(other.i_range()) {
                        for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                            f(&[i0, j0, k0], &[i1, j1, k1]);
                        }
                    }
                }
            }
            (true, true) => {
                for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                    for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                        for (i0, i1) in self.i_range().zip(other.i_range()) {
                            f(&[i0, j0, k0], &[i1, j1, k1]);
                        }
                    }
                }
            }
        }
    }

    /// Iterates over this loop in tandem with the given other loop, executing
    /// the given closure with the 3D indices and linear indices in each loop
    /// for each iteration.
    ///
    /// # Panics
    /// If the number of iterations in the two loops is not equal.
    #[inline(always)]
    pub fn zip_execute_with_linear_indices<const M: usize>(
        &self,
        other: &Loop3<M>,
        f: &mut impl FnMut((&[usize; 3], usize), (&[usize; 3], usize)),
    ) {
        // Note: Doing the matching to select optimal loop order could be a lot slower
        // than using the standard order when the method is run cold, but seems to be
        // faster after warming up (could be that the branch predictor needs some time
        // to learn the patterns)
        assert_eq!(self.n_iterations(), other.n_iterations());
        match (self.move_j_loop_out, self.move_k_loop_out) {
            (false, false) => {
                for (i0, i1) in self.i_range().zip(other.i_range()) {
                    for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                        for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                            f(
                                (&[i0, j0, k0], Self::linear_idx(i0, j0, k0)),
                                (&[i1, j1, k1], Loop3::<M>::linear_idx(i1, j1, k1)),
                            );
                        }
                    }
                }
            }
            (true, false) => {
                for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                    for (i0, i1) in self.i_range().zip(other.i_range()) {
                        for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                            f(
                                (&[i0, j0, k0], Self::linear_idx(i0, j0, k0)),
                                (&[i1, j1, k1], Loop3::<M>::linear_idx(i1, j1, k1)),
                            );
                        }
                    }
                }
            }
            (false, true) => {
                for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                    for (i0, i1) in self.i_range().zip(other.i_range()) {
                        for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                            f(
                                (&[i0, j0, k0], Self::linear_idx(i0, j0, k0)),
                                (&[i1, j1, k1], Loop3::<M>::linear_idx(i1, j1, k1)),
                            );
                        }
                    }
                }
            }
            (true, true) => {
                for (j0, j1) in (self.j_range()).zip(other.j_range()) {
                    for (k0, k1) in (self.k_range()).zip(other.k_range()) {
                        for (i0, i1) in self.i_range().zip(other.i_range()) {
                            f(
                                (&[i0, j0, k0], Self::linear_idx(i0, j0, k0)),
                                (&[i1, j1, k1], Loop3::<M>::linear_idx(i1, j1, k1)),
                            );
                        }
                    }
                }
            }
        }
    }
}

impl<'a, 'b, T, const N: usize> DataLoop3<'a, 'b, T, N> {
    /// Creates a new loop over (part of) the given data slice.
    #[inline(always)]
    pub fn new(lp: &'a Loop3<N>, data: &'b [T]) -> Self {
        Self { lp, data }
    }

    /// Executes the given closure for each iteration in the loop, passing in
    /// the 3D indices and the data value of the iteration.
    ///
    /// # Panics
    /// If the length of the data slice is smaller than the maximum linear
    /// index in the loop.
    #[inline(always)]
    pub fn execute(self, f: &mut impl FnMut(&[usize; 3], &T)) {
        assert!(self.data.len() >= self.lp.max_linear_idx());
        self.lp.execute_with_linear_idx(&mut |indices, data_idx| {
            // SAFETY: We checked that the length of the data slice is not
            // smaller than maximum linear index in the loop
            let data = unsafe { self.data.get_unchecked(data_idx) };
            f(indices, data);
        });
    }
}

impl<'a, 'b, T, const N: usize> MutDataLoop3<'a, 'b, T, N> {
    /// Creates a new loop over (part of) the given mutable data slice.
    #[inline(always)]
    pub fn new(lp: &'a Loop3<N>, data: &'b mut [T]) -> Self {
        Self { lp, data }
    }

    /// Executes the given closure for each iteration in the loop, passing in
    /// the 3D indices and the mutable data value of the iteration.
    ///
    /// # Panics
    /// If the length of the data slice is smaller than the maximum linear
    /// index in the loop.
    #[inline(always)]
    pub fn execute(self, f: &mut impl FnMut(&[usize; 3], &mut T)) {
        assert!(self.data.len() >= self.lp.max_linear_idx());
        self.lp.execute_with_linear_idx(&mut |indices, data_idx| {
            // SAFETY: We checked that the length of the data slice is not
            // smaller than maximum linear index in the loop
            let data = unsafe { self.data.get_unchecked_mut(data_idx) };
            f(indices, data);
        });
    }

    /// Iterates over the loop, writing the given value into the associated
    /// location in the data slice for each iteration.
    ///
    /// # Panics
    /// If the length of the data slice is smaller than the maximum linear
    /// index in the loop.
    #[inline(always)]
    pub fn fill_data_with_value(self, value: T)
    where
        T: Copy,
    {
        assert!(self.data.len() >= self.lp.max_linear_idx());
        self.lp.execute_with_linear_idx(&mut |_, data_idx| {
            // SAFETY: We checked that the length of the data slice is not
            // smaller than maximum linear index in the loop
            let data = unsafe { self.data.get_unchecked_mut(data_idx) };

            *data = value;
        });
    }

    /// Iterates over the loop, writing the next value in the given slice into
    /// the associated location in the data slice for each subsequent
    /// iteration.
    ///
    /// # Panics
    /// - If the length of the input slice is not equal to the number of
    ///   iterations in the loop.
    /// - If the length of the data slice is smaller than the maximum linear
    ///   index in the loop.
    #[inline(always)]
    pub fn map_slice_values_into_data<U>(self, slice: &[U], map: &impl Fn(&U) -> T) {
        assert_eq!(slice.len(), self.lp.n_iterations());
        assert!(self.data.len() >= self.lp.max_linear_idx());

        let mut slice_idx = 0;
        self.lp.execute_with_linear_idx(&mut |_, data_idx| {
            // SAFETY: We checked that the length of the slice is not
            // smaller than the number of iterations in the loop
            let value = unsafe { slice.get_unchecked(slice_idx) };

            // SAFETY: We checked that the length of the data slice is not
            // smaller than maximum linear index in the loop
            let data = unsafe { self.data.get_unchecked_mut(data_idx) };

            *data = map(value);

            slice_idx += 1;
        });
    }

    /// Iterates over this loop in tandem with the given other loop, and for
    /// each iteration fetches the associated data value from the other
    /// loop, applies the given mapping to it and writes the result into the
    /// associated location in this loop's data slice.
    ///
    /// # Panics
    /// - If the number of iterations in the two loops is not equal.
    /// - If the length of the data slice for this loop is smaller than the
    ///   maximum linear index in the loop.
    /// - If the length of the data slice for the other loop is smaller than the
    ///   maximum linear index in that loop.
    #[inline(always)]
    pub fn map_other_data_into_data<U, const M: usize>(
        self,
        other: DataLoop3<'_, '_, U, M>,
        map: &impl Fn(&U) -> T,
    ) {
        assert!(self.data.len() >= self.lp.max_linear_idx());
        assert!(other.data.len() >= other.lp.max_linear_idx());

        self.lp.zip_execute_with_linear_indices(
            other.lp,
            &mut |(_, self_data_idx), (_, other_data_idx)| {
                // SAFETY: We checked that the length of the data slice is not
                // smaller than maximum linear index in the loop
                let data = unsafe { self.data.get_unchecked_mut(self_data_idx) };

                // SAFETY: We checked that the length of the other data slice is
                // not smaller than maximum linear index in the other loop
                let other_value = unsafe { other.data.get_unchecked(other_data_idx) };

                *data = map(other_value);
            },
        );
    }
}
