// seg.rs ----------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// Seg — integer segment range representation modeled directly from Trellis / Kosh useg.rs.
// Internally represents the closed range [_First, _Last].

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seg {
    pub _First: u32,
    pub _Last: u32,
}

// USeg — unsigned 32-bit segment alias matching Trellis silo::USeg.
pub type USeg = Seg;

impl Seg {
    //---------------------------------------------------------------------------------------------
    // Constructors & Factories

    #[inline]
    pub const fn Empty() -> Self {
        Self {
            _First: 1,
            _Last: 0,
        }
    }

    #[inline]
    pub const fn New(first: u32, last: u32) -> Self {
        Self {
            _First: first,
            _Last: last,
        }
    }

    #[inline]
    pub const fn FromLen(len: u32) -> Self {
        if len == 0 {
            Self::Empty()
        } else {
            Self {
                _First: 0,
                _Last: len - 1,
            }
        }
    }

    #[inline]
    pub const fn WithLen(first: u32, len: u32) -> Self {
        if len == 0 {
            Self::Empty()
        } else {
            Self {
                _First: first,
                _Last: first + len - 1,
            }
        }
    }

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries

    #[inline]
    pub const fn First(&self) -> u32 {
        self._First
    }

    #[inline]
    pub const fn Begin(&self) -> u32 {
        self._First
    }

    #[inline]
    pub const fn Last(&self) -> u32 {
        self._Last
    }

    #[inline]
    pub const fn End(&self) -> u32 {
        if self.IsEmpty() {
            self._First
        } else {
            self._Last + 1
        }
    }

    #[inline]
    pub const fn Mid(&self) -> u32 {
        if self._Last >= self._First {
            self._First + (self._Last - self._First) / 2
        } else {
            self._First
        }
    }

    #[inline]
    pub const fn IsEmpty(&self) -> bool {
        self._First > self._Last
    }

    #[inline]
    pub const fn Len(&self) -> u32 {
        if self.IsEmpty() {
            0
        } else {
            self._Last - self._First + 1
        }
    }

    #[inline]
    pub const fn Size(&self) -> u32 {
        self.Len()
    }

    #[inline]
    pub const fn Contains(&self, val: u32) -> bool {
        !self.IsEmpty() && val >= self._First && val <= self._Last
    }

    #[inline]
    pub const fn IsWithin(&self, val: u32) -> bool {
        self.Contains(val)
    }

    #[inline]
    pub const fn Overlaps(&self, other: &Self) -> bool {
        if self.IsEmpty() || other.IsEmpty() {
            return false;
        }
        self._First <= other._Last && other._First <= self._Last
    }

    #[inline]
    pub fn Intersect(&self, other: &Self) -> Self {
        if self.IsEmpty() || other.IsEmpty() {
            return Self::Empty();
        }
        let first = self._First.max(other._First);
        let last = self._Last.min(other._Last);
        if first <= last {
            Self::New(first, last)
        } else {
            Self::Empty()
        }
    }

    //---------------------------------------------------------------------------------------------
    // Slicing & Operations

    #[inline]
    pub const fn LSnip(&self, count: u32) -> Self {
        let sz = self.Len();
        if sz <= count {
            Self::Empty()
        } else {
            Self::WithLen(self._First + count, sz - count)
        }
    }

    #[inline]
    pub const fn RSnip(&self, count: u32) -> Self {
        let sz = self.Len();
        if sz <= count {
            Self::Empty()
        } else {
            Self::WithLen(self._First, sz - count)
        }
    }

    #[inline(always)]
    pub fn Span<F: FnMut(u32) -> bool>(&self, mut f: F) -> bool {
        if self.IsEmpty() {
            return true;
        }
        let mut i = self._First;
        while i <= self._Last {
            if !f(i) {
                return false;
            }
            if i == u32::MAX {
                break;
            }
            i += 1;
        }
        true
    }

    #[inline(always)]
    pub fn Traverse<F: FnMut(u32)>(&self, mut f: F) {
        if self.IsEmpty() {
            return;
        }
        let mut i = self._First;
        while i <= self._Last {
            f(i);
            if i == u32::MAX {
                break;
            }
            i += 1;
        }
    }

    //---------------------------------------------------------------------------------------------
    // Sorting (Partition & Quicksort)

    pub fn Partition<L: FnMut(u32, u32) -> bool, S: FnMut(u32, u32)>(
        &self,
        mut less_at: L,
        mut swap_at: S,
    ) -> u32 {
        let mid = self.Mid();
        if less_at(self._First, mid) {
            swap_at(self._First, mid);
        }

        let mut pivot = self._First;
        self.LSnip(1).Traverse(|i| {
            if less_at(i, self._First) {
                pivot += 1;
                swap_at(pivot, i);
            }
        });

        if less_at(pivot, self._First) {
            swap_at(self._First, pivot);
        }

        pivot
    }

    pub fn QSort<L: FnMut(u32, u32) -> bool + Copy, S: FnMut(u32, u32) + Copy>(
        &self,
        less_at: L,
        swap_at: S,
    ) {
        let mut current_seg = *self;
        while current_seg.Size() > 1 {
            let pivot = current_seg.Partition(less_at, swap_at);
            let useg1 = if pivot > current_seg._First {
                Seg::New(current_seg._First, pivot - 1)
            } else {
                Seg::Empty()
            };
            let useg2 = if current_seg._Last > pivot {
                Seg::New(pivot + 1, current_seg._Last)
            } else {
                Seg::Empty()
            };

            if useg1.Size() < useg2.Size() {
                if useg1.Size() > 1 {
                    useg1.QSort(less_at, swap_at);
                }
                current_seg = useg2;
            } else {
                if useg2.Size() > 1 {
                    useg2.QSort(less_at, swap_at);
                }
                current_seg = useg1;
            }
        }
    }
}
