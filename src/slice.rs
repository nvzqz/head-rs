use core::{
    mem::{self, MaybeUninit},
    slice,
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
        &*(slice::from_raw_parts(header, 0) as *const [H] as *const Self)
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
        &mut *(slice::from_raw_parts_mut(header, 0) as *mut [H] as *mut Self)
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
        let data = Box::leak(header);
        Box::from_raw(slice::from_raw_parts_mut(data, 0) as *mut [H] as *mut Self)
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
        let new_len = slice.len().wrapping_sub(1);
        &*(slice::from_raw_parts(slice.as_ptr(), new_len) as *const [H] as *const Self)
    }

    /// Creates a mutable header-slice from `slice`, using the first element as
    /// the header without checking if it exists.
    ///
    /// # Safety
    ///
    /// `slice` must be non-empty.
    #[inline]
    pub unsafe fn from_full_slice_unchecked_mut(slice: &mut [H]) -> &mut Self {
        let new_len = slice.len().wrapping_sub(1);
        &mut *(slice::from_raw_parts_mut(slice.as_mut_ptr(), new_len) as *mut [H] as *mut Self)
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
        let data = Box::leak(slice) as *mut [H] as *mut H;

        Box::from_raw(slice::from_raw_parts_mut(data, new_len) as *mut [H] as *mut Self)
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
