//-- charset.rs -------------------------------------------------------------------------------------------------------------------

use crate::silo::{Arr, Buff, USeg};
use std::sync::LazyLock;
use std::{
    cmp, fmt,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
};

//---------------------------------------------------------------------------------------------------------------------------------
/// A 256-bit filter for `u8` characters — one bit per byte value.
/// Enables set algebra ( union, intersection, negation) over character classes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Charset {
    _Bits: [u64; Self::SZ],
}

//---------------------------------------------------------------------------------------------------------------------------------

impl Charset {
    const SZ: usize = 4;
    const SZ_BITS: u32 = 64;

    //-----------------------------------------------------------------------------------------------------------------------------

    pub const fn New() -> Self {
        Self {
            _Bits: [0u64; Self::SZ],
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn FromFilter(filter: fn(u8) -> bool) -> Self {
        let mut cs = Self::New();
        USeg::New(0, 255).Traverse(|i| {
            cs.Set(i as u8, filter(i as u8));
        });
        cs
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl From<&[u8]> for Charset {
    fn from(spec: &[u8]) -> Self {
        Self::from(Arr::FromSlice(spec))
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<'a> From<Arr<'a, u8>> for Charset {
    fn from(spec: Arr<'a, u8>) -> Self {
        if (spec.Size() > 2) && (*spec.First().unwrap() == b':') && (*spec.Last().unwrap() == b':')
        {
            let csetStr: &str = std::str::from_utf8(unsafe {
                let a = spec.LSnip(1u32).RSnip(1u32);
                std::slice::from_raw_parts(a.Data(), a.Size() as usize)
            })
            .unwrap();
            if csetStr == "alnum" {
                return *Self::AlphaNum();
            }
            if csetStr == "alpha" {
                return Self::Lower().Union(Self::Upper());
            }
            if csetStr == "ascii" {
                return *Self::Ascii();
            }
            if csetStr == "blank" {
                return *Self::Blank();
            }
            if csetStr == "cntrl" {
                return *Self::Cntrl();
            }
            if csetStr == "digit" {
                return *Self::Digit();
            }
            if csetStr == "graph" {
                return *Self::Graph();
            }
            if csetStr == "lower" {
                return *Self::Lower();
            }
            if csetStr == "print" {
                return *Self::Print();
            }
            if csetStr == "punct" {
                return *Self::Punct();
            }
            if csetStr == "space" {
                return *Self::Space();
            }
            if csetStr == "upper" {
                return *Self::Upper();
            }
            if csetStr == "word" {
                return *Self::Word();
            }
            if csetStr == "xdigit" {
                return *Self::XDigit();
            }
        }
        let mut cs = Self::New();
        let mut i = 0usize;
        while i < spec.Size() as usize {
            let first = *spec.Get(i as u32).unwrap();
            cs.SetChar(first);
            // peek for  '-' range
            if i + 2 < spec.Size() as usize && *spec.Get(i as u32 + 1).unwrap() == (b'-' as u8) {
                let last = *spec.Get(i as u32 + 2).unwrap();
                cs.SetByteRange(first, last, true);
                i += 3;
            } else {
                i += 1;
            }
        }
        cs
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Charset {
    pub fn Get<C: Into<u8>>(&self, c: C) -> bool {
        let c = c.into();
        let idx = (c as usize) / Self::SZ_BITS as usize;
        let bit = (c as u32) % Self::SZ_BITS;
        (self._Bits[idx] & (1u64 << bit as u64)) != 0u64
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    pub fn SetChar<C: Into<u8>>(&mut self, c: C) {
        let c = c.into();
        let idx = (c as usize) / Self::SZ_BITS as usize;
        let bit = (c as u32) % Self::SZ_BITS;
        self._Bits[idx] |= 1u64 << bit as u64;
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn ClearChar<C: Into<u8>>(&mut self, c: C) {
        let c = c.into();
        let idx = (c as usize) / Self::SZ_BITS as usize;
        let bit = (c as u32) % Self::SZ_BITS;
        self._Bits[idx] &= !(1u64 << bit as u64);
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Set the bit for byte `c` to `v`.
    pub fn Set<C: Into<u8>>(&mut self, c: C, v: bool) {
        if v {
            self.SetChar(c)
        } else {
            self.ClearChar(c)
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Set all bits in the inclusive range `start..=stop` to `value`.
    pub fn SetByteRange<C: Into<u8>>(&mut self, start: C, stop: C, value: bool) {
        let start = start.into() as u8;
        let stop = stop.into() as u8;
        if start <= stop {
            USeg::New(start as u32, stop as u32).Traverse(|c| {
                self.Set(c as u8, value);
            });
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Flip all 256 bits ( complement).
    pub fn Negate(&mut self) {
        USeg::FromLen(Self::SZ as u32).Traverse(|i| {
            self._Bits[i as usize] = !self._Bits[i as usize];
        });
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Return a negated copy.
    pub fn Negative(&self) -> Self {
        let mut cpy = *self;
        cpy.Negate();
        cpy
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Check whether `self` and `other` share any set bit.
    pub fn IsIntersect(&self, other: &Charset) -> bool {
        let mut intersect = false;
        USeg::FromLen(Self::SZ as u32).Span(|i| {
            if (self._Bits[i as usize] & other._Bits[i as usize]) != 0u64 {
                intersect = true;
                false
            } else {
                true
            }
        });
        intersect
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// OR `other` into self.
    pub fn UnionWith(&mut self, other: &Charset) {
        USeg::FromLen(Self::SZ as u32).Traverse(|i| {
            self._Bits[i as usize] |= other._Bits[i as usize];
        });
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// AND `other` into self.
    pub fn IntersectWith(&mut self, other: &Charset) {
        USeg::FromLen(Self::SZ as u32).Traverse(|i| {
            self._Bits[i as usize] &= other._Bits[i as usize];
        });
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Diff `other` into self.
    pub fn DiffWith(&mut self, other: &Charset) {
        self.IntersectWith(&other.Negative())
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Return the union of self and `other`
    ///
    pub fn Union(&self, other: &Charset) -> Self {
        let mut cpy = *self;
        cpy.UnionWith(other);
        cpy
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Intersect(&self, other: &Charset) -> Self {
        let mut cpy = *self;
        cpy.IntersectWith(other);
        cpy
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Lexicographic comparison of the four u64 words.
    pub fn Compare(&self, other: &Charset) -> i32 {
        match self.cmp(other) {
            cmp::Ordering::Less => -1,
            cmp::Ordering::Equal => 0,
            cmp::Ordering::Greater => 1,
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Collect all byte-values whose bit is set.
    pub fn ListChars(&self) -> Buff<u8> {
        let weight = self.Weight();
        let mut list = Buff::FromDispenser(weight, |_| 0u8);
        let mut idx = 0usize;
        USeg::FromLen(Self::SZ as u32).Traverse(|i| {
            let mut val = self._Bits[i as usize];
            while val != 0 {
                let tz = val.trailing_zeros();
                list[idx as u32] = (i * Self::SZ_BITS + tz) as u8;
                idx += 1;
                val &= val - 1;
            }
        });
        list
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Count of set bits ( population count).
    pub fn Weight(&self) -> u32 {
        let mut count = 0u32;
        USeg::FromLen(Self::SZ as u32).Traverse(|i| {
            count += self._Bits[i as usize].count_ones();
        });
        count
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    // Predefined character classes
    pub fn All() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::New().Negative());
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Digit() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_digit()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn NonDigit() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::Digit().Negative());
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Word() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| {
            let mut cs = Charset::New();
            cs.SetChar(b'_');
            cs.SetByteRange(b'a', b'z', true);
            cs.SetByteRange(b'A', b'Z', true);
            cs.SetByteRange(b'0', b'9', true);
            cs
        });
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn NonWord() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::Word().Negative());
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn AlphaNum() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_alphanumeric()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Ascii() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Blank() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| {
            let mut cs = Charset::New();
            cs.SetChar(b' ');
            cs.SetChar(b'\t');
            cs
        });
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn EndLine() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| {
            let mut cs = Charset::New();
            cs.SetChar(b'\n');
            cs
        });
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Cntrl() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_control()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Graph() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_graphic()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Print() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_graphic() || c == b' '));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Punct() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_punctuation()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Space() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_whitespace()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn NonSpace() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::Space().Negative());
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Alpha() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_alphabetic()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Upper() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_uppercase()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Lower() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_lowercase()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn XDigit() -> &'static Charset {
        static VAL: LazyLock<Charset> =
            LazyLock::new(|| Charset::FromFilter(|c| c.is_ascii_hexdigit()));
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Dot() -> &'static Charset {
        Charset::DotAll()
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn DotAll() -> &'static Charset {
        static VAL: LazyLock<Charset> = LazyLock::new(|| Charset::New().Negative());
        &VAL
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    // Formatting helpers
    fn PrettyPrintChar(c: u8, chrClsFlg: bool, out: &mut String) {
        let val = c;
        match val {
            b'\t' => {
                out.push_str("\\t");
                return;
            }
            b'\n' => {
                out.push_str("\\n");
                return;
            }
            b'\r' => {
                out.push_str("\\r");
                return;
            }
            0x0C => {
                out.push_str("\\f");
                return;
            } // form-feed
            0x07 => {
                out.push_str("\\a");
                return;
            } // bell
            0x0B => {
                out.push_str("\\v");
                return;
            } // vertical tab
            _ => {}
        }
        let mut hex = false;
        let mut escape = false;
        if !chrClsFlg {
            if b"'\"=".contains(&val) {
                hex = true;
            }
            if b"^$ *+{ }[].\\/|?".contains(&val) {
                escape = true;
            }
        } else {
            if b"^[]\\/- ".contains(&val) {
                escape = true;
            }
        }
        if !val.is_ascii_alphanumeric() && val != b'.' && val != b'$' && val != b'@' && val != b'_'
        {
            hex = true;
        }
        if escape {
            out.push('\\');
            out.push(val as char);
        } else if hex {
            out.push_str(&format!("\\x{:02X}", val));
        } else {
            out.push(val as char);
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Format as a bracket expression, optionally negated.
    fn ToBoxetString(&self, negFlg: bool) -> String {
        let chars = if negFlg {
            self.Negative().ListChars()
        } else {
            self.ListChars()
        };
        let mut s = String::with_capacity(chars.Size() as usize * 2 + 4);
        s.push('[');
        if negFlg {
            s.push('^');
        }
        let mut i = 0usize;
        while i < chars.Size() as usize {
            let mut j = i + 1;
            while j < chars.Size() as usize && chars[j as u32] == chars[(j - 1) as u32] + 1 {
                j += 1;
            }
            let runLen = j - i;
            Self::PrettyPrintChar(chars[i as u32], true, &mut s);
            if runLen > 2 {
                s.push('-');
            }
            if runLen > 1 {
                Self::PrettyPrintChar(chars[(i + runLen - 1) as u32], true, &mut s);
            }
            i += runLen;
        }
        s.push(']');
        s
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    fn ToString(&self) -> String {
        let posStr = self.ToBoxetString(false);
        let negStr = self.ToBoxetString(true);
        if posStr.len() <= negStr.len() {
            posStr
        } else {
            negStr
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------
}

//---------------------------------------------------------------------------------------------------------------------------------

impl fmt::Display for Charset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.Compare(Charset::Word()) == 0 {
            return write!(f, "[[Word]]");
        }
        if self.Compare(Charset::NonWord()) == 0 {
            return write!(f, "[[NonWord]]");
        }
        write!(f, "{ }", self.ToString())
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl fmt::Debug for Charset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Charset( { })", self)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl cmp::PartialOrd for Charset {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl cmp::Ord for Charset {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self._Bits.cmp(&other._Bits)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl Not for Charset {
    type Output = Self;
    fn not(self) -> Self::Output {
        self.Negative()
    }
}
impl BitOr for Charset {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.Union(&rhs)
    }
}
impl BitAnd for Charset {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.Intersect(&rhs)
    }
}
impl BitOrAssign for Charset {
    fn bitor_assign(&mut self, rhs: Self) {
        self.UnionWith(&rhs);
    }
}
impl BitAndAssign for Charset {
    fn bitand_assign(&mut self, rhs: Self) {
        self.IntersectWith(&rhs);
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

//---------------------------------------------------------------------------------------------------------------------------------
