/// A dynamically-sized view into a contiguous header and trailing sequence.
#[repr(C)]
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct HeaderSlice<H, T> {
    /// The value preceding a slice of `T` in memory.
    pub header: H,

    /// The trailing contiguous sequence of values.
    pub slice: [T],
}
