//! A byte vector that ensures a specified alignment.

use anyhow::{Result, bail};
use std::{
    alloc::{self, Layout},
    cmp, mem,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
    slice,
};

/// A valid pointer address alignment, guaranteed to be non-zero
/// and a power of two.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alignment(usize);

/// A container with similar functionality to [`Vec<u8>`], but
/// that guarantees that the underlying memory block has a
/// specified alignment.
///
/// # Warning
/// The address of `AlignedByteVec`s data is only guaranteed to
/// be aligned when the capacity is non-zero.
#[derive(Debug)]
pub struct AlignedByteVec {
    layout: Layout,
    ptr: NonNull<u8>,
    len: usize,
}

impl Alignment {
    pub const ONE: Self = Self(1);
    pub const TWO: Self = Self(2);
    pub const FOUR: Self = Self(4);
    pub const EIGHT: Self = Self(8);
    pub const SIXTEEN: Self = Self(16);

    /// Wraps the given alignment in an [`Alignment`].
    ///
    /// # Errors
    /// Returns an error if `alignment` is zero or not a power of two.
    pub fn try_new(alignment: usize) -> Result<Self> {
        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            bail!("`Alignment` created with invalid alignment: {}", alignment)
        } else {
            Ok(Self(alignment))
        }
    }

    /// Wraps the given alignment in an [`Alignment`].
    ///
    /// # Panics
    /// If `alignment` is zero or not a power of two.
    pub fn new(alignment: usize) -> Self {
        Self::try_new(alignment).unwrap_or_else(|err| panic!("{}", err))
    }

    /// Creates a new [`Alignment`] corresponding to the alignment of
    /// type `T`.
    pub const fn of<T>() -> Self {
        Self(mem::align_of::<T>())
    }

    /// Returns the alignment as a [`usize`].
    pub const fn get(&self) -> usize {
        self.0
    }

    /// Whether the given number is a multiple of this alignment.
    pub const fn is_aligned(&self, number: usize) -> bool {
        number & (self.0 - 1) == 0
    }

    fn of_layout(layout: Layout) -> Self {
        Self(layout.align())
    }
}

impl From<Alignment> for usize {
    fn from(alignment: Alignment) -> Self {
        alignment.0
    }
}

impl AlignedByteVec {
    /// Constructs a new, empty [`AlignedByteVec`] with the given
    /// alignment.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    pub fn new(alignment: Alignment) -> Self {
        Self {
            // SAFETY:
            // - `Alignment` is guaranteed to hold a valid alignment.
            // - The passed size of zero never overflows `isize`.
            layout: unsafe { Layout::from_size_align_unchecked(0, alignment.into()) },
            ptr: NonNull::dangling(),
            len: 0,
        }
    }

    /// Constructs a new, empty [`AlignedByteVec`] with the given alignment and
    /// at least the specified capacity.
    ///
    /// The vector will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If capacity is 0, the vector will not allocate.
    ///
    /// # Panics
    /// If `capacity` exceeds `isize::MAX`.
    pub fn with_capacity(alignment: Alignment, capacity: usize) -> Self {
        if capacity == 0 {
            Self::new(alignment)
        } else {
            let (layout, ptr) = Self::allocate_memory_with_alignment_and_size(alignment, capacity);
            Self {
                layout,
                ptr,
                len: 0,
            }
        }
    }

    /// Constructs a new [`AlignedByteVec`] with the given alignment and copies
    /// the given bytes into it.
    ///
    /// # Panics
    /// If the length of `bytes` exceeds `isize::MAX`.
    pub fn copied_from_slice(alignment: Alignment, bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            Self::new(alignment)
        } else {
            let len = bytes.len();
            let (layout, ptr) = Self::allocate_memory_with_alignment_and_size(alignment, len);
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), len);
            }
            Self { layout, ptr, len }
        }
    }

    /// Returns the alignment of the block of memory containing the data of
    /// the vector.
    pub fn alignment(&self) -> Alignment {
        Alignment::of_layout(self.layout)
    }

    /// Returns the number of elements the vector can hold without
    /// reallocating.
    pub fn capacity(&self) -> usize {
        self.layout.size()
    }

    /// Returns the number of elements in the vector, also referred to as
    /// its 'length'.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    pub fn as_slice(&self) -> &[u8] {
        self
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self
    }

    /// Returns a raw pointer to the vector's buffer, or a dangling raw pointer
    /// valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer
    /// (non-transitively) points to is never written to (except inside an
    /// `UnsafeCell`) using this pointer or any pointer derived from it. If
    /// you need to mutate the contents of the slice, use [`Self::as_mut_ptr`].
    pub fn as_ptr(&self) -> *const u8 {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.ptr.as_ptr()
    }

    /// Returns an unsafe mutable pointer to the vector's buffer, or a dangling
    /// raw pointer valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        // We shadow the slice method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.ptr.as_ptr()
    }

    /// Extends the vector with all the bytes in the given slice.
    ///
    /// # Panics
    /// If the new capacity of the vector exceeds `isize::MAX`.
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let old_len = self.len();
        let added_len = other.len();
        let new_len = old_len.checked_add(added_len).unwrap();

        if added_len > 0 {
            self.reserve(added_len);

            unsafe {
                // SAFETY:
                // The memory blocks are guaranteed to be nonoverlapping since `self`
                // is borrowed mutably (so `other` is not from the same memory)
                ptr::copy_nonoverlapping(other.as_ptr(), self.ptr.as_ptr().add(old_len), added_len);

                // Force new length for the vector to encompass new data
                self.set_len(new_len);
            }
        }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            unsafe {
                self.set_len(len);
            }
        }
    }

    /// Resizes the vector in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the vector is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the vector is simply truncated.
    pub fn resize(&mut self, new_len: usize, value: u8) {
        let len = self.len();

        if new_len > len {
            let n_additional = new_len - len;
            self.reserve(n_additional);
            unsafe {
                ptr::write_bytes(self.ptr.as_ptr().add(len), value, n_additional);
                self.set_len(new_len);
            };
        } else {
            self.truncate(new_len);
        }
    }

    fn reserve(&mut self, n_additional: usize) {
        let len = self.len();
        let old_layout = self.layout;
        let alignment = Alignment::of_layout(old_layout);
        let old_capacity = old_layout.size();

        // Calculate capacity required to hold the additional elements
        let required_capacity = len.checked_add(n_additional).expect("Capacity overflow");

        // Only do something if the required capacity exceeds the current one
        if required_capacity > old_capacity {
            // Require new capacity that is either the required capacity or twice
            // the current capacity, whichever is largest
            let new_minimum_capacity = cmp::max(
                required_capacity,
                old_capacity.checked_mul(2).expect("Capacity overflow"),
            );

            // Allocate new memory
            let new_layout = Self::create_layout_for_allocation(alignment, new_minimum_capacity);
            let new_ptr = unsafe { Self::allocate_with_layout(new_layout) };

            // If we already had some allocated memory, we copy it into the new
            // memory block and deallocate the old memory block
            if old_capacity != 0 {
                let old_ptr = self.ptr.as_ptr();
                unsafe {
                    // We only need to copy `len` values, as this is the only
                    // accessible data
                    ptr::copy_nonoverlapping(old_ptr, new_ptr.as_ptr(), len);
                    alloc::dealloc(old_ptr, old_layout);
                }
            }

            self.layout = new_layout;
            self.ptr = new_ptr;
        }
    }

    unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    fn allocate_memory_with_alignment_and_size(
        alignment: Alignment,
        size: usize,
    ) -> (Layout, NonNull<u8>) {
        let layout = Self::create_layout_for_allocation(alignment, size);
        let ptr = unsafe { Self::allocate_with_layout(layout) };
        (layout, ptr)
    }

    fn create_layout_for_allocation(alignment: Alignment, minimum_size: usize) -> Layout {
        // Calling `alloc` with zero size is undefined behavior
        assert_ne!(minimum_size, 0);

        let alignment: usize = alignment.into();

        // Round up the size to the nearest alignment (the expression
        // is only valid if alignment is a power of two, but if it is
        // not, `from_size_align` will return an error regardless of
        // the size)
        let size = (minimum_size + alignment - 1) & !(alignment - 1);

        if usize::BITS < 64 && size > isize::MAX as usize {
            panic!("Allocation size exceeds `isize::MAX`")
        }

        // SAFETY:
        // - `Alignment` is guaranteed to hold a valid alignment.
        // - We just checked that `size` doesn't overflow `isize`.
        unsafe { Layout::from_size_align_unchecked(size, alignment) }
    }

    unsafe fn allocate_with_layout(layout: Layout) -> NonNull<u8> {
        let ptr = unsafe { alloc::alloc(layout) };

        if ptr.is_null() {
            // Abort if the allocation failed
            alloc::handle_alloc_error(layout);
        } else {
            unsafe { NonNull::new_unchecked(ptr) }
        }
    }
}

// SAFETY: The allocated memory is never aliased
unsafe impl Send for AlignedByteVec {}
unsafe impl Sync for AlignedByteVec {}

impl Drop for AlignedByteVec {
    fn drop(&mut self) {
        // If we have any allocated memory, we must deallocate it manually
        if self.layout.size() != 0 {
            unsafe {
                alloc::dealloc(self.ptr.as_ptr(), self.layout);
            }
        }
    }
}

impl Deref for AlignedByteVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl DerefMut for AlignedByteVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Clone for AlignedByteVec {
    fn clone(&self) -> Self {
        if self.layout.size() == 0 {
            Self {
                layout: self.layout,
                ptr: NonNull::dangling(),
                len: 0,
            }
        } else {
            let mut cloned = Self::with_capacity(self.alignment(), self.capacity());
            unsafe {
                ptr::copy_nonoverlapping(self.ptr.as_ptr(), cloned.ptr.as_ptr(), self.len());
                cloned.set_len(self.len());
            };
            cloned
        }
    }
}

impl PartialEq for AlignedByteVec {
    fn eq(&self, other: &Self) -> bool {
        self[..] == other[..]
    }
}

impl Eq for AlignedByteVec {}

#[cfg(test)]
mod tests {
    use super::*;

    const BYTES: [u8; 32] = [
        45, 12, 246, 71, 89, 105, 39, 128, 201, 3, 22, 75, 199, 213, 37, 9, 85, 71, 224, 23, 176,
        105, 45, 12, 146, 7, 81, 2, 173, 199, 237, 64,
    ];

    const BYTES2: [u8; 16] = [
        45, 71, 89, 2, 173, 199, 237, 64, 22, 75, 199, 213, 37, 9, 85, 71,
    ];

    fn has_alignment_of(bytes: &[u8], alignment: Alignment) -> bool {
        (bytes.as_ptr() as usize).is_multiple_of(<Alignment as Into<usize>>::into(alignment))
    }

    #[test]
    #[should_panic]
    fn creating_alignment_with_zero_alignment_fails() {
        Alignment::new(0);
    }

    #[test]
    #[should_panic]
    fn creating_alignment_with_non_power_of_two_alignment_fails() {
        Alignment::new(3);
    }

    #[test]
    fn creating_new_empty_aligned_byte_vec_works() {
        let alignment = Alignment::new(4);
        let vec = AlignedByteVec::new(alignment);

        assert_eq!(vec.alignment(), alignment);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }

    #[test]
    fn creating_aligned_byte_vec_copied_from_empty_slice_works() {
        let alignment = Alignment::new(1);
        let vec = AlignedByteVec::copied_from_slice(alignment, &[]);

        assert_eq!(vec.alignment(), alignment);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }

    #[test]
    fn creating_aligned_byte_vec_copied_from_slice_works() {
        let alignment = Alignment::new(8);
        let vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);

        assert!(has_alignment_of(&vec, alignment));
        assert_eq!(vec.alignment(), alignment);
        assert_eq!(vec.capacity(), BYTES.len());
        assert_eq!(vec.len(), BYTES.len());
        assert_eq!(&*vec, &BYTES);
        assert_eq!(vec.as_slice(), &BYTES);
    }

    #[test]
    fn creating_aligned_byte_vec_with_capacity_works() {
        let vec = AlignedByteVec::with_capacity(Alignment::new(1), 0);

        assert_eq!(vec.capacity(), 0);
        assert!(vec.is_empty());
        assert!(vec.as_slice().is_empty());

        for cap_and_align in [1, 2, 4, 8, 16, 32, 64, 128] {
            let vec = AlignedByteVec::with_capacity(Alignment::new(cap_and_align), cap_and_align);

            assert_eq!(vec.capacity(), cap_and_align);
            assert!(vec.is_empty());
            assert!(vec.as_slice().is_empty());
        }
    }

    #[test]
    fn creating_aligned_byte_vec_with_nonzero_capacity_gives_specified_alignments() {
        for alignment in [1, 2, 4, 8, 16, 32, 64, 128] {
            let alignment = Alignment::new(alignment);
            let vec = AlignedByteVec::with_capacity(alignment, alignment.into());

            assert!(has_alignment_of(&vec, alignment));
            assert_eq!(vec.alignment(), alignment);
        }
    }

    #[test]
    fn creating_aligned_byte_vec_copied_from_slice_gives_specified_alignments() {
        for alignment in [1, 2, 4, 8, 16, 32, 64, 128] {
            let alignment = Alignment::new(alignment);
            let vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);

            assert!(has_alignment_of(&vec, alignment));
            assert_eq!(vec.alignment(), alignment);
        }
    }

    #[test]
    fn extending_empty_aligned_byte_vec_with_slice_works() {
        let alignment = Alignment::new(4);
        let mut vec = AlignedByteVec::new(alignment);
        vec.extend_from_slice(&BYTES);

        assert!(has_alignment_of(&vec, alignment));
        assert_eq!(vec.alignment(), alignment);
        assert!(vec.capacity() >= BYTES.len());
        assert_eq!(vec.len(), BYTES.len());
        assert_eq!(&*vec, &BYTES);
        assert_eq!(vec.as_slice(), &BYTES);
    }

    #[test]
    fn extending_nonempty_aligned_byte_vec_with_slice_works() {
        let alignment = Alignment::new(8);
        let mut vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);
        vec.extend_from_slice(&BYTES2);

        assert!(has_alignment_of(&vec, alignment));
        assert_eq!(vec.alignment(), alignment);
        assert!(vec.capacity() >= BYTES.len() + BYTES2.len());
        assert_eq!(vec.len(), BYTES.len() + BYTES2.len());
        assert_eq!(&vec[..BYTES.len()], &BYTES);
        assert_eq!(&vec[BYTES.len()..], &BYTES2);

        vec.extend_from_slice(&BYTES);

        assert!(has_alignment_of(&vec, alignment));
        assert_eq!(vec.alignment(), alignment);
        assert!(vec.capacity() >= 2 * BYTES.len() + BYTES2.len());
        assert_eq!(vec.len(), 2 * BYTES.len() + BYTES2.len());
        assert_eq!(&vec[..BYTES.len()], &BYTES);
        assert_eq!(&vec[BYTES.len()..(BYTES.len() + BYTES2.len())], &BYTES2);
        assert_eq!(&vec[(BYTES.len() + BYTES2.len())..], &BYTES);
    }

    #[test]
    fn extending_nonempty_aligned_byte_vec_with_empty_slice_works() {
        let alignment = Alignment::new(8);
        let mut vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);
        vec.extend_from_slice(&[]);

        assert!(has_alignment_of(&vec, alignment));
        assert_eq!(vec.alignment(), alignment);
        assert_eq!(vec.capacity(), BYTES.len());
        assert_eq!(vec.len(), BYTES.len());
        assert_eq!(&*vec, &BYTES);
        assert_eq!(vec.as_slice(), &BYTES);
    }

    #[test]
    fn cloning_empty_aligned_byte_vec_works() {
        let alignment = Alignment::new(2);
        let vec = AlignedByteVec::new(alignment);

        #[allow(clippy::redundant_clone)]
        let cloned = vec.clone();

        assert_eq!(cloned.alignment(), alignment);
        assert_eq!(cloned.capacity(), 0);
        assert_eq!(cloned.len(), 0);
        assert!(cloned.is_empty());
    }

    #[test]
    fn cloning_nonempty_aligned_byte_vec_works() {
        let alignment = Alignment::new(2);
        let vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);

        #[allow(clippy::redundant_clone)]
        let cloned = vec.clone();

        assert!(has_alignment_of(&cloned, alignment));
        assert_eq!(cloned.alignment(), alignment);
        assert_eq!(cloned.capacity(), BYTES.len());
        assert_eq!(cloned.len(), BYTES.len());
        assert_eq!(&*cloned, &BYTES);
        assert_eq!(cloned.as_slice(), &BYTES);
    }

    #[test]
    fn truncating_nonempty_aligned_byte_vec_works() {
        let alignment = Alignment::new(2);
        let mut vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);
        let new_len = 28;
        vec.truncate(new_len);

        assert_eq!(vec.len(), new_len);
        assert_eq!(vec.as_slice(), &BYTES[..new_len]);
    }

    #[test]
    fn resizing_nonempty_aligned_byte_vec_to_shorter_len_works() {
        let alignment = Alignment::new(2);
        let mut vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);
        let new_len = 28;
        vec.resize(new_len, 0);

        assert_eq!(vec.len(), new_len);
        assert_eq!(vec.as_slice(), &BYTES[..new_len]);
    }

    #[test]
    fn resizing_nonempty_aligned_byte_vec_to_longer_len_works() {
        let alignment = Alignment::new(2);
        let mut vec = AlignedByteVec::copied_from_slice(alignment, &BYTES);
        let new_len = 46;
        let value = 0;
        vec.resize(new_len, value);

        assert_eq!(vec.len(), new_len);
        assert_eq!(&vec[..BYTES.len()], &BYTES);
        assert!(vec[BYTES.len()..].iter().all(|&v| v == value));
    }
}
