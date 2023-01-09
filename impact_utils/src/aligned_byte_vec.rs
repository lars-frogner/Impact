//! A wrapper for [`Vec<u8>`] that ensures a specified alignment.

use std::{
    alloc::{self, Layout},
    cmp, mem,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr,
};

/// A valid pointer address alignment, guaranteed to be non-zero
/// and a power of two.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Alignment(usize);

/// A wrapper for [`Vec<u8>`] that ensures a specified alignment.
///
/// # Warning
/// The address of `AlignedByteVec`s data is only guaranteed to
/// be aligned when the capacity is non-zero.
#[derive(Debug)]
pub struct AlignedByteVec {
    layout: Layout,
    // It is important that the byte `Vec` is never dropped, as we
    // have to allocate and deallocate its memory for it. If `bytes`
    // dropped after we had deallocated its memory, we would have a double
    // free. Moreover, the `Vec` assumes that its memory was allocated with
    // align_of::<u8>() = 1, and would thus potentially deallocate its memory
    // with the wrong alignment, causing undefined behavior. For the same
    // reason, it is unsafe for us to call any method on `bytes` that could
    // cause it to re- or deallocate its memory.
    bytes: ManuallyDrop<Vec<u8>>,
}

impl Alignment {
    /// Wraps the given alignment in an [`Alignment`], returning an
    /// error if the alignment is invalid.
    ///
    /// # Panics
    /// If `alignment` is zero or not a power of two.
    pub fn new(alignment: usize) -> Self {
        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            panic!("`Alignment` created with invalid alignment: {}", alignment)
        } else {
            Self(alignment)
        }
    }

    /// Creates a new [`Alignment`] corresponding to the alignment of
    /// type `T`.
    pub fn of<T>() -> Self {
        Self(mem::align_of::<T>())
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
            bytes: ManuallyDrop::new(Vec::new()),
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
            let bytes = unsafe { Vec::from_raw_parts(ptr, 0, layout.size()) };
            Self {
                layout,
                bytes: ManuallyDrop::new(bytes),
            }
        }
    }

    /// Constructs a new [`AlignedByteVec`] with the given alignment and copies
    /// the given bytes into it.
    ///
    /// # Panics
    /// If the length of `bytes` exceeds `isize::MAX`.
    pub fn copied_from_slice(alignment: Alignment, bytes: &[u8]) -> Self {
        let (layout, ptr) = Self::allocate_memory_with_alignment_and_size(alignment, bytes.len());
        let vec = unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            Vec::from_raw_parts(ptr, bytes.len(), layout.size())
        };
        Self {
            layout,
            bytes: ManuallyDrop::new(vec),
        }
    }

    /// Returns the alignment of the block of memory containing the data of
    /// the vector.
    pub fn alignment(&self) -> usize {
        self.layout.align()
    }

    /// Returns the number of elements the vector can hold without
    /// reallocating.
    pub fn capacity(&self) -> usize {
        self.layout.size()
    }

    /// Returns the number of elements in the vector, also referred to as
    /// its 'length'.
    pub fn len(&self) -> usize {
        self.bytes.len()
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

    /// Extends the vector with all the bytes in the given slice.
    ///
    /// # Panics
    /// If the new capacity of the vector exceeds `isize::MAX`.
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        let old_len = self.bytes.len();
        let added_len = other.len();
        let new_len = old_len.checked_add(added_len).unwrap();

        self.reserve(added_len);

        unsafe {
            // SAFETY:
            // The memory blocks are guaranteed to be nonoverlapping since `self.bytes`
            // is borrowed mutably (so `other` is not from the same memory)
            ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.bytes.as_mut_ptr().offset(old_len.try_into().unwrap()),
                added_len,
            );

            // Force new length for the vector to encompass new data
            self.bytes.set_len(new_len);
        }
    }

    fn reserve(&mut self, n_additional: usize) {
        let old_len = self.bytes.len();
        let old_layout = self.layout;
        let alignment = Alignment::of_layout(old_layout);
        let old_capacity = old_layout.size();

        // Calculate capacity required to hold the additional elements
        let required_capacity = old_len
            .checked_add(n_additional)
            .expect("Capacity overflow");

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
                let old_ptr = self.bytes.as_mut_ptr();
                unsafe {
                    // We only need to copy `old_len` values, as this is the only
                    // accessible data
                    ptr::copy_nonoverlapping(old_ptr, new_ptr, old_len);
                    alloc::dealloc(old_ptr, old_layout);
                }
            }

            let new_bytes = unsafe { Vec::from_raw_parts(new_ptr, old_len, new_layout.size()) };

            self.layout = new_layout;
            self.bytes = ManuallyDrop::new(new_bytes);
        }
    }

    fn allocate_memory_with_alignment_and_size(
        alignment: Alignment,
        size: usize,
    ) -> (Layout, *mut u8) {
        let layout = Self::create_layout_for_allocation(alignment, size);
        let ptr = unsafe { Self::allocate_with_layout(layout) };
        (layout, ptr)
    }

    fn create_layout_for_allocation(alignment: Alignment, minimum_size: usize) -> Layout {
        // Calling `alloc` with zero size is undefined behavior
        assert_ne!(minimum_size, 0);

        let alignment: usize = alignment.into();

        // Round up the size to the nearest alignment (the expression
        // is only valid if alignement is a power of two, but if it is
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

    unsafe fn allocate_with_layout(layout: Layout) -> *mut u8 {
        let ptr = unsafe { alloc::alloc(layout) };

        if ptr.is_null() {
            // Abort if the allocation failed
            alloc::handle_alloc_error(layout);
        } else {
            ptr
        }
    }
}

impl Drop for AlignedByteVec {
    fn drop(&mut self) {
        // If `self.bytes` has any allocated memory, we must deallocate it
        // manually with the correct alignment
        if self.layout.size() != 0 {
            unsafe {
                alloc::dealloc(self.bytes.as_mut_ptr(), self.layout);
            }
        }
        // `self.bytes` has heap memory at this point, and will just be
        // popped off the stack
    }
}

impl Deref for AlignedByteVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for AlignedByteVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const BYTES: [u8; 32] = [
        45, 12, 246, 71, 89, 105, 39, 128, 201, 3, 22, 75, 199, 213, 37, 9, 85, 71, 224, 23, 176,
        105, 45, 12, 146, 7, 81, 2, 173, 199, 237, 64,
    ];

    const BYTES2: [u8; 16] = [
        45, 71, 89, 2, 173, 199, 237, 64, 22, 75, 199, 213, 37, 9, 85, 71,
    ];

    fn has_alignment_of(bytes: &[u8], alignment: Alignment) -> bool {
        (bytes.as_ptr() as usize) % <Alignment as Into<usize>>::into(alignment) == 0
    }

    #[test]
    #[should_panic]
    fn creating_alignement_with_zero_alignment_fails() {
        Alignment::new(0);
    }

    #[test]
    #[should_panic]
    fn creating_alignement_with_non_power_of_two_alignment_fails() {
        Alignment::new(3);
    }

    #[test]
    fn creating_new_empty_aligned_byte_vec_works() {
        let alignment = Alignment::new(4);
        let vec = AlignedByteVec::new(alignment);

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
}
