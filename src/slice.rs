use core::{mem, slice};

/// A dynamically-sized view into a contiguous header and trailing sequence.
#[repr(C)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct HeaderSlice<H, T> {
    /// The value preceding a slice of `T` in memory.
    pub header: H,

    /// The trailing contiguous sequence of values.
    pub slice: [T],
}

// TODO: `From<Box<H>>` for `Box<HeaderSlice<H, H>>`
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
}
