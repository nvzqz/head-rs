use core::{
    mem::{self, MaybeUninit},
    ptr, slice,
};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// A dynamically-sized view into a contiguous header and trailing sequence.
#[repr(C)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct HeaderSlice<H, T> {
    /// The value preceding a slice of `T` in memory.
    pub header: H,

    /// The trailing contiguous sequence of values.
    pub slice: [T],
}

#[repr(C)]
struct HeaderSliceDummy<H, T> {
    header: MaybeUninit<H>,
    slice: MaybeUninit<[T; 0]>,
}

#[repr(C)]
struct HeaderSliceHeader<H, T> {
    header: H,
    // Using `MaybeUninit` to avoid dropck for `T`.
    slice: MaybeUninit<[T; 0]>,
}

impl<H, T> HeaderSliceHeader<H, T> {
    #[inline]
    fn as_header_slice(&self) -> &HeaderSlice<H, T> {
        // SAFETY: `header` satisfies slice alignment requirement of `T`.
        unsafe { HeaderSlice::from_header_unchecked(&self.header) }
    }

    #[inline]
    fn as_header_slice_mut(&mut self) -> &mut HeaderSlice<H, T> {
        // SAFETY: `header` satisfies slice alignment requirement of `T`.
        unsafe { HeaderSlice::from_header_unchecked_mut(&mut self.header) }
    }
}

/// Convenience functions for handling raw memory.
#[allow(dead_code)]
impl<H, T> HeaderSlice<H, T> {
    /// Returns the alignment for header-slice allocations.
    #[inline]
    pub(crate) fn align() -> usize {
        mem::align_of::<HeaderSliceDummy<H, T>>()
    }

    /// Returns the offset from the base address of a header-slice to the slice.
    #[inline]
    pub(crate) fn items_offset() -> usize {
        let dummy = HeaderSliceDummy::<H, T> {
            header: MaybeUninit::uninit(),
            slice: MaybeUninit::uninit(),
        };

        let base_addr = &dummy as *const _ as usize;
        let slice_addr = &dummy.slice as *const _ as usize;

        slice_addr - base_addr
    }
}

// TODO: `From<Arc<H>>` for `Arc<HeaderSlice<H, H>>`
// TODO: `From<Rc<H>>`  for `Rc<HeaderSlice<H, H>>`

// TODO: `Clone` for `Box<HeaderSlice<H, T>>`

impl<'a, H> From<&'a H> for &'a HeaderSlice<H, H> {
    #[inline]
    fn from(header: &'a H) -> Self {
        // SAFETY: `H` satisfies slice alignment requirement.
        unsafe { HeaderSlice::from_header_unchecked(header) }
    }
}

impl<'a, H> From<&'a mut H> for &'a mut HeaderSlice<H, H> {
    #[inline]
    fn from(header: &'a mut H) -> Self {
        // SAFETY: `H` satisfies slice alignment requirement.
        unsafe { HeaderSlice::from_header_unchecked_mut(header) }
    }
}

#[cfg(feature = "alloc")]
impl<H> From<Box<H>> for Box<HeaderSlice<H, H>> {
    #[inline]
    fn from(header: Box<H>) -> Self {
        // SAFETY: `H` satisfies slice alignment requirement.
        unsafe { HeaderSlice::from_boxed_header_unchecked(header) }
    }
}

#[cfg(feature = "alloc")]
impl<H> From<Box<HeaderSlice<H, H>>> for Box<[H]> {
    #[inline]
    fn from(hs: Box<HeaderSlice<H, H>>) -> Self {
        hs.into_full_boxed_slice()
    }
}

/// Returns `true` if `header` is aligned to the slice element type `T`.
#[inline]
fn is_header_slice_aligned<H, T>(header: *const H) -> bool {
    header as usize % mem::align_of::<T>() == 0
}

impl<H, T> HeaderSlice<H, T> {
    /// Returns the result of calling `f` on a shared header-slice starting with
    /// `header`.
    #[inline]
    pub fn with_header<F, R>(header: H, f: F) -> R
    where
        F: for<'a> FnOnce(&'a Self) -> R,
    {
        let hs = HeaderSliceHeader::<H, T> {
            header,
            slice: MaybeUninit::uninit(),
        };

        f(hs.as_header_slice())
    }

    /// Returns the result of calling `f` on a mutable header-slice starting
    /// with `header`.
    #[inline]
    pub fn with_header_mut<F, R>(header: H, f: F) -> R
    where
        F: for<'a> FnOnce(&'a mut Self) -> R,
    {
        let mut hs = HeaderSliceHeader::<H, T> {
            header,
            slice: MaybeUninit::uninit(),
        };

        f(hs.as_header_slice_mut())
    }

    /// Attempts to create a shared header-slice from just `header`.
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    ///
    /// If `T` has equal or greater alignment than `H`, unwrapping the returned
    /// value is a no-op.
    #[inline]
    pub fn from_header(header: &H) -> Option<&Self> {
        if is_header_slice_aligned::<H, T>(header) {
            // SAFETY: `header` satisfies slice alignment requirement.
            Some(unsafe { Self::from_header_unchecked(header) })
        } else {
            None
        }
    }

    /// Attempts to create a mutable header-slice from just `header`.
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    ///
    /// If `T` has equal or greater alignment than `H`, unwrapping the returned
    /// value is a no-op.
    #[inline]
    pub fn from_header_mut(header: &mut H) -> Option<&mut Self> {
        if is_header_slice_aligned::<H, T>(header) {
            // SAFETY: `header` satisfies slice alignment requirement.
            Some(unsafe { Self::from_header_unchecked_mut(header) })
        } else {
            None
        }
    }

    /// Attempts to create a boxed header-slice from just `header`.
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    ///
    /// If `T` has equal or greater alignment than `H`, unwrapping the returned
    /// value is a no-op.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn from_boxed_header(header: Box<H>) -> Result<Box<Self>, Box<H>> {
        if is_header_slice_aligned::<H, T>(&*header) {
            // SAFETY: `header` satisfies slice alignment requirement.
            Ok(unsafe { Self::from_boxed_header_unchecked(header) })
        } else {
            Err(header)
        }
    }

    /// Create a shared header-slice from just `header`, without checking its
    /// alignment.
    ///
    /// # Safety
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    #[inline]
    pub unsafe fn from_header_unchecked(header: &H) -> &Self {
        Self::from_raw_parts(header, 0)
    }

    /// Create a mutable header-slice from just `header`, without checking its
    /// alignment.
    ///
    /// # Safety
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    #[inline]
    pub unsafe fn from_header_unchecked_mut(header: &mut H) -> &mut Self {
        Self::from_raw_parts_mut(header, 0)
    }

    /// Create a boxed header-slice from just `header`, without checking its
    /// alignment.
    ///
    /// # Safety
    ///
    /// The address of `header` must be at least aligned to `T` because the
    /// empty slice component must be properly aligned.
    #[cfg(feature = "alloc")]
    #[inline]
    pub unsafe fn from_boxed_header_unchecked(header: Box<H>) -> Box<Self> {
        Self::boxed_from_raw_parts(Box::into_raw(header), 0)
    }

    /// Forms a shared header-slice from a pointer and a length.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `header` and any slice following it must be [valid].
    ///
    ///   - `header` must be non-null and aligned to the greater alignment
    ///     between `H` and `T`.
    ///
    ///   - If `len` is non-zero, the slice following `header` must be aligned
    ///     to `T`.
    ///
    ///   - The entire memory range of spanning from the start of `header` to
    ///     the end of the trailing slice must be contained within a single
    ///     allocated object! Header-slices can never span across multiple
    ///     allocated objects.
    ///
    /// - The memory referenced by the returned header-slice must not be mutated
    ///   for the duration of lifetime `'a`, except inside an [`UnsafeCell`].
    ///
    /// - The total size of the resulting header-slice must be no larger than
    ///   `isize::MAX`.
    ///
    /// # Caveat
    ///
    /// The lifetime for the returned slice is inferred from its usage. To
    /// prevent accidental misuse, it's suggested to tie the lifetime to
    /// whichever source lifetime is safe in the context, such as by providing a
    /// helper function taking the lifetime of a host value for the slice, or by
    /// explicit annotation.
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    /// [`UnsafeCell`]: https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html
    #[inline]
    pub unsafe fn from_raw_parts<'a>(header: *const H, len: usize) -> &'a Self {
        // We never create `&[H]` because data past `header` may refer to
        // invalid instances of `H`. So instead we strictly use a raw slice
        // pointer.
        &*(ptr::slice_from_raw_parts(header, len) as *const Self)
    }

    /// Forms a mutable header-slice from a pointer and a length.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `header` and any slice following it must be [valid].
    ///
    ///   - `header` must be non-null and aligned to the greater alignment
    ///     between `H` and `T`.
    ///
    ///   - If `len` is non-zero, the slice following `header` must be aligned
    ///     to `T`.
    ///
    ///   - The entire memory range of spanning from the start of `header` to
    ///     the end of the trailing slice must be contained within a single
    ///     allocated object! Header-slices can never span across multiple
    ///     allocated objects.
    ///
    /// - The memory referenced by the returned header-slice must not be
    ///   accessed through any other pointer (not derived from the return value)
    ///   for the duration of lifetime `'a`. Both read and write accesses are
    ///   forbidden.
    ///
    /// - The total size of the resulting header-slice must be no larger than
    ///   `isize::MAX`.
    ///
    /// # Caveat
    ///
    /// The lifetime for the returned slice is inferred from its usage. To
    /// prevent accidental misuse, it's suggested to tie the lifetime to
    /// whichever source lifetime is safe in the context, such as by providing a
    /// helper function taking the lifetime of a host value for the slice, or by
    /// explicit annotation.
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    #[inline]
    pub unsafe fn from_raw_parts_mut<'a>(header: *mut H, len: usize) -> &'a mut Self {
        // We never create `&mut [H]` because data past `header` may refer to
        // invalid instances of `H`. So instead we strictly use a raw slice
        // pointer.
        &mut *(ptr::slice_from_raw_parts_mut(header, len) as *mut Self)
    }

    /// Forms a boxed header-slice from a pointer and a length.
    ///
    /// # Safety
    ///
    /// `header` must point to a header-slice with a slice of `len` items that
    /// has been allocated by the global allocator.
    ///
    /// Improper use can lead to:
    ///
    /// - A double-free if the function is called twice on the same raw pointer.
    ///
    /// - Mutable aliasing, which causes undefined behavior.
    ///
    /// [valid]: https://doc.rust-lang.org/std/ptr/index.html#safety
    #[cfg(feature = "alloc")]
    #[inline]
    pub unsafe fn boxed_from_raw_parts(header: *mut H, len: usize) -> Box<Self> {
        // We never create `&mut [H]` because data past `header` may refer to
        // invalid instances of `H`. So instead we strictly use a raw slice
        // pointer.
        Box::from_raw(ptr::slice_from_raw_parts_mut(header, len) as *mut Self)
    }
}

impl<H> HeaderSlice<H, H> {
    /// Attempts to create a shared header-slice from `slice`, using the first
    /// element as the header.
    ///
    /// Returns `None` if `slice` is empty.
    #[inline]
    pub fn from_full_slice(slice: &[H]) -> Option<&Self> {
        if slice.is_empty() {
            None
        } else {
            // SAFETY: `slice` has an element for a header.
            Some(unsafe { Self::from_full_slice_unchecked(slice) })
        }
    }

    /// Attempts to create a mutable header-slice from `slice`, using the first
    /// element as the header.
    ///
    /// Returns `None` if `slice` is empty.
    #[inline]
    pub fn from_full_slice_mut(slice: &mut [H]) -> Option<&mut Self> {
        if slice.is_empty() {
            None
        } else {
            // SAFETY: `slice` has an element for a header.
            Some(unsafe { Self::from_full_slice_unchecked_mut(slice) })
        }
    }

    /// Attempts to create a boxed header-slice from `slice`, using the first
    /// element as the header.
    ///
    /// Returns `None` if `slice` is empty.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn from_full_boxed_slice(slice: Box<[H]>) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            // SAFETY: `slice` has an element for a header.
            Some(unsafe { Self::from_full_boxed_slice_unchecked(slice) })
        }
    }

    /// Creates a shared header-slice from `slice`, using the first element as
    /// the header without checking if it exists.
    ///
    /// # Safety
    ///
    /// `slice` must be non-empty.
    #[inline]
    pub unsafe fn from_full_slice_unchecked(slice: &[H]) -> &Self {
        Self::from_raw_parts(slice.as_ptr(), slice.len().wrapping_sub(1))
    }

    /// Creates a mutable header-slice from `slice`, using the first element as
    /// the header without checking if it exists.
    ///
    /// # Safety
    ///
    /// `slice` must be non-empty.
    #[inline]
    pub unsafe fn from_full_slice_unchecked_mut(slice: &mut [H]) -> &mut Self {
        Self::from_raw_parts_mut(slice.as_mut_ptr(), slice.len().wrapping_sub(1))
    }

    /// Creates a boxed header-slice from `slice`, using the first element as
    /// the header without checking if it exists.
    ///
    /// # Safety
    ///
    /// `slice` must be non-empty.
    #[cfg(feature = "alloc")]
    #[inline]
    pub unsafe fn from_full_boxed_slice_unchecked(slice: Box<[H]>) -> Box<Self> {
        let new_len = slice.len().wrapping_sub(1);
        let header = Box::into_raw(slice) as *mut H;

        Self::boxed_from_raw_parts(header, new_len)
    }

    /// Returns the full range of `self` as a single shared slice.
    #[inline]
    pub fn as_full_slice(&self) -> &[H] {
        let data = self as *const Self as *const H;
        let len = self.slice.len() + 1;

        unsafe { slice::from_raw_parts(data, len) }
    }

    /// Returns the full range of `self` as a single mutable slice.
    #[inline]
    pub fn as_full_slice_mut(&mut self) -> &mut [H] {
        let data = self as *mut Self as *mut H;
        let len = self.slice.len() + 1;

        unsafe { slice::from_raw_parts_mut(data, len) }
    }

    /// Returns the full range of `self` as a single boxed slice.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn into_full_boxed_slice(self: Box<Self>) -> Box<[H]> {
        let len = self.slice.len() + 1;
        let data = Box::into_raw(self) as *mut H;

        unsafe { Box::from_raw(slice::from_raw_parts_mut(data, len)) }
    }
}
