// shape.rs ---------------------------------------------------------------------------------------
use	core::fmt;
use	crate::silo::Arr;

//-------------------------------------------------------------------------------------------------

pub const K_MAX_TENSOR_RANK: usize = 6;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub enum ShapeError
{
    RankOverflow( usize),
    ZeroDimension( u32),
    IncompatibleBroadcast( TensorShape, TensorShape),
    IndexOutOfBounds
    {
        axis:  u32,
        index: u32,
        dim:   u32,
    },
    LinearIndexOutOfBounds
    {
        index: u32,
        total: u32,
    },
    RankMismatch
    {
        expected: u32,
        actual:   u32,
    },
    CannotSliceScalar,
    Overflow,
    DuplicateSymbol( String),
}

//-------------------------------------------------------------------------------------------------

impl fmt::Display for ShapeError
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        match self
        {
            Self::RankOverflow( r) => write!( f, "Tensor rank {} exceeds maximum of {}", r, K_MAX_TENSOR_RANK),
            Self::ZeroDimension( axis) => write!( f, "Dimension at axis {} cannot be zero", axis),
            Self::IncompatibleBroadcast( a, b) =>
            {
                write!( f, "Shapes {:?} and {:?} are incompatible for broadcasting", a, b)
            }
            Self::IndexOutOfBounds { axis, index, dim } =>
            {
                write!( f, "Index {} out of bounds for axis {} with dimension {}", index, axis, dim)
            }
            Self::LinearIndexOutOfBounds { index, total } =>
            {
                write!( f, "Linear index {} out of bounds for tensor with {} elements", index, total)
            }
            Self::RankMismatch { expected, actual } =>
            {
                write!( f, "Rank mismatch: expected {}, got {}", expected, actual)
            }
            Self::CannotSliceScalar =>
            {
                write!( f, "Cannot slice or extract sub-shape from a scalar")
            }
            Self::Overflow =>
            {
                write!( f, "Tensor dimensions or element count overflowed u32 capacity")
            }
            Self::DuplicateSymbol( s) =>
            {
                write!( f, "Variable symbol '{}' already exists in repository", s)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Copy, PartialEq, Eq)]
pub struct TensorShape
{
    pub(crate) _Rank: u8,
    pub(crate) _D0:   u32,
    pub(crate) _D1:   u32,
    pub(crate) _D2:   u32,
    pub(crate) _D3:   u32,
    pub(crate) _D4:   u32,
    pub(crate) _D5:   u32,
}

//-------------------------------------------------------------------------------------------------

impl TensorShape
{
    pub const fn	Scalar() -> Self
    {
        Self {
            _Rank: 0,
            _D0:   0,
            _D1:   0,
            _D2:   0,
            _D3:   0,
            _D4:   0,
            _D5:   0,
        }
    }

    pub fn	Vector( len: u32) -> Result< Self, ShapeError>
    {
        if len == 0
        {
            return Err( ShapeError::ZeroDimension( 0));
        }
        Ok( Self {
            _Rank: 1,
            _D0:   len,
            _D1:   0,
            _D2:   0,
            _D3:   0,
            _D4:   0,
            _D5:   0,
        })
    }

    pub fn	Matrix( rows: u32, cols: u32) -> Result< Self, ShapeError>
    {
        if rows == 0
        {
            return Err( ShapeError::ZeroDimension( 0));
        }
        if cols == 0
        {
            return Err( ShapeError::ZeroDimension( 1));
        }
        rows.checked_mul( cols).ok_or( ShapeError::Overflow)?;
        Ok( Self {
            _Rank: 2,
            _D0:   rows,
            _D1:   cols,
            _D2:   0,
            _D3:   0,
            _D4:   0,
            _D5:   0,
        })
    }

    pub fn	FromArr( dims: Arr< '_, u32>) -> Result< Self, ShapeError>
    {
        let  	rank = dims.Size() as usize;
        if rank > K_MAX_TENSOR_RANK
        {
            return Err( ShapeError::RankOverflow( rank));
        }
        let  	mut total = 1u32;
        let  	mut i = 0usize;
        while i < rank
        {
            let  	d = dims[i as u32];
            if d == 0
            {
                return Err( ShapeError::ZeroDimension( i as u32));
            }
            total = total.checked_mul( d).ok_or( ShapeError::Overflow)?;
            i += 1;
        }

        // Validate strides computation does not overflow
        if rank > 0
        {
            let  	mut s = 1u32;
            let  	mut j = rank - 1;
            while j > 0
            {
                s = s.checked_mul( dims[j as u32]).ok_or( ShapeError::Overflow)?;
                j -= 1;
            }
        }

        let  	mut shape = Self::Scalar();
        shape._Rank = rank as u8;
        let  	mut k = 0usize;
        while k < rank
        {
            shape.SetDim( k as u32, dims[k as u32]);
            k += 1;
        }
        return Ok( shape);
    }

    pub fn	FromSlice( dims: &[u32]) -> Result< Self, ShapeError>
    {
        Self::FromArr( Arr::New( dims.as_ptr(), dims.len() as u32))
    }

    pub fn	Rank( &self) -> u32
    {
        return self._Rank as u32;
    }

    pub fn	Dim( &self, axis: u32) -> u32
    {
        match axis
        {
            0 => self._D0,
            1 => self._D1,
            2 => self._D2,
            3 => self._D3,
            4 => self._D4,
            5 => self._D5,
            _ => 0,
        }
    }

    pub fn	SetDim( &mut self, axis: u32, val: u32)
    {
        match axis
        {
            0 => self._D0 = val,
            1 => self._D1 = val,
            2 => self._D2 = val,
            3 => self._D3 = val,
            4 => self._D4 = val,
            5 => self._D5 = val,
            _ => {},
        }
    }

    pub fn	DimsArray( &self) -> [u32; K_MAX_TENSOR_RANK]
    {
        return [self._D0, self._D1, self._D2, self._D3, self._D4, self._D5];
    }

    pub fn	IsScalar( &self) -> bool
    {
        return self._Rank == 0;
    }

    pub fn	IsVector( &self) -> bool
    {
        return self._Rank == 1;
    }

    pub fn	IsMatrix( &self) -> bool
    {
        return self._Rank == 2;
    }

    pub fn	TotalElements( &self) -> u32
    {
        if self._Rank == 0
        {
            return 1;
        }
        let  	dims = self.DimsArray();
        let  	mut total = 1u32;
        let  	mut i = 0usize;
        while i < self._Rank as usize
        {
            total = total.saturating_mul( dims[i]);
            i += 1;
        }
        return total;
    }

    pub fn	Strides( &self) -> [u32; K_MAX_TENSOR_RANK]
    {
        let  	mut strides = [0u32; K_MAX_TENSOR_RANK];
        if self._Rank == 0
        {
            return strides;
        }
        let  	dims = self.DimsArray();
        let  	rank = self._Rank as usize;
        strides[rank - 1] = 1;
        let  	mut i = rank - 1;
        while i > 0
        {
            strides[i - 1] = strides[i] * dims[i];
            i -= 1;
        }
        return strides;
    }

    pub fn	LinearIndex( &self, indices: Arr< '_, u32>) -> Result< u32, ShapeError>
    {
        let  	rank = self._Rank as usize;
        if indices.Size() as usize != rank
        {
            return Err( ShapeError::RankMismatch {
                expected: rank as u32,
                actual:   indices.Size(),
            });
        }
        if rank == 0
        {
            return Ok( 0);
        }
        let  	dims = self.DimsArray();
        let  	strides = self.Strides();
        let  	mut linear = 0u32;
        let  	mut i = 0usize;
        while i < rank
        {
            let  	idx = indices[i as u32];
            if idx >= dims[i]
            {
                return Err( ShapeError::IndexOutOfBounds {
                    axis:  i as u32,
                    index: idx,
                    dim:   dims[i],
                });
            }
            let  	term = idx.checked_mul( strides[i]).ok_or( ShapeError::Overflow)?;
            linear = linear.checked_add( term).ok_or( ShapeError::Overflow)?;
            i += 1;
        }
        return Ok( linear);
    }

    pub fn	LinearIndexSlice( &self, indices: &[u32]) -> Result< u32, ShapeError>
    {
        self.LinearIndex( Arr::New( indices.as_ptr(), indices.len() as u32))
    }

    pub fn	MultiIndex( &self, linearIndex: u32) -> Result< [u32; K_MAX_TENSOR_RANK], ShapeError>
    {
        let  	total = self.TotalElements();
        if linearIndex >= total
        {
            return Err( ShapeError::LinearIndexOutOfBounds {
                index: linearIndex,
                total,
            });
        }
        let  	mut coords = [0u32; K_MAX_TENSOR_RANK];
        if self._Rank == 0
        {
            return Ok( coords);
        }
        let  	strides = self.Strides();
        let  	mut rem = linearIndex;
        let  	rank = self._Rank as usize;
        let  	mut i = 0usize;
        while i < rank
        {
            coords[i] = rem / strides[i];
            rem %= strides[i];
            i += 1;
        }
        return Ok( coords);
    }

    pub fn	SubShape( &self) -> Result< Self, ShapeError>
    {
        if self._Rank == 0
        {
            return Err( ShapeError::CannotSliceScalar);
        }
        if self._Rank == 1
        {
            return Ok( Self::Scalar());
        }
        let  	mut shape = Self::Scalar();
        shape._Rank = self._Rank - 1;
        shape._D0 = self._D1;
        shape._D1 = self._D2;
        shape._D2 = self._D3;
        shape._D3 = self._D4;
        shape._D4 = self._D5;
        shape._D5 = 0;
        return Ok( shape);
    }

    pub fn	SliceShape( &self, numLeadingIndices: usize) -> Result< Self, ShapeError>
    {
        let  	rank = self._Rank as usize;
        if numLeadingIndices > rank
        {
            return Err( ShapeError::RankMismatch {
                expected: rank as u32,
                actual:   numLeadingIndices as u32,
            });
        }
        if numLeadingIndices == 0
        {
            return Ok( *self);
        }
        if numLeadingIndices == rank
        {
            return Ok( Self::Scalar());
        }
        let  	dims = self.DimsArray();
        let  	newRank = rank - numLeadingIndices;
        let  	mut shape = Self::Scalar();
        shape._Rank = newRank as u8;
        let  	mut i = 0usize;
        while i < newRank
        {
            shape.SetDim( i as u32, dims[numLeadingIndices + i]);
            i += 1;
        }
        return Ok( shape);
    }

    pub fn	LeadingOffset( &self, indices: Arr< '_, u32>) -> Result< u32, ShapeError>
    {
        let  	rank = self._Rank as usize;
        let  	idxLen = indices.Size() as usize;
        if idxLen > rank
        {
            return Err( ShapeError::RankMismatch {
                expected: rank as u32,
                actual:   idxLen as u32,
            });
        }
        if idxLen == 0
        {
            return Ok( 0);
        }
        let  	dims = self.DimsArray();
        let  	strides = self.Strides();
        let  	mut offset = 0u32;
        let  	mut i = 0usize;
        while i < idxLen
        {
            let  	dim = dims[i];
            let  	idx = indices[i as u32];
            if idx >= dim
            {
                return Err( ShapeError::IndexOutOfBounds {
                    axis:  i as u32,
                    index: idx,
                    dim,
                });
            }
            let  	term = idx.checked_mul( strides[i]).ok_or( ShapeError::Overflow)?;
            offset = offset.checked_add( term).ok_or( ShapeError::Overflow)?;
            i += 1;
        }
        return Ok( offset);
    }

    pub fn	LeadingOffsetSlice( &self, indices: &[u32]) -> Result< u32, ShapeError>
    {
        self.LeadingOffset( Arr::New( indices.as_ptr(), indices.len() as u32))
    }

    pub fn	SubShapeElementCount( &self, numLeadingIndices: usize) -> Result< u32, ShapeError>
    {
        let  	subShape = self.SliceShape( numLeadingIndices)?;
        return Ok( subShape.TotalElements());
    }

    pub fn	CanBroadcastWith( &self, other: &Self) -> bool
    {
        return self.BroadcastWith( other).is_ok();
    }

    pub fn	BroadcastWith( &self, other: &Self) -> Result< Self, ShapeError>
    {
        let  	rankA = self._Rank as usize;
        let  	rankB = other._Rank as usize;
        let  	maxRank = if rankA > rankB { rankA } else { rankB };
        if maxRank > K_MAX_TENSOR_RANK
        {
            return Err( ShapeError::RankOverflow( maxRank));
        }

        let  	dimsA = self.DimsArray();
        let  	dimsB = other.DimsArray();
        let  	mut result = Self::Scalar();
        result._Rank = maxRank as u8;

        let  	mut i = 0usize;
        while i < maxRank
        {
            let  	dimA = if i < rankA { dimsA[rankA - 1 - i] } else { 1 };
            let  	dimB = if i < rankB { dimsB[rankB - 1 - i] } else { 1 };

            if dimA == dimB
            {
                result.SetDim( ( maxRank - 1 - i) as u32, dimA);
            }
            else if dimA == 1
            {
                result.SetDim( ( maxRank - 1 - i) as u32, dimB);
            }
            else if dimB == 1
            {
                result.SetDim( ( maxRank - 1 - i) as u32, dimA);
            }
            else
            {
                return Err( ShapeError::IncompatibleBroadcast( *self, *other));
            }
            i += 1;
        }

        // Validate broadcasted shape does not overflow
        let  	resDims = result.DimsArray();
        let  	mut checkTotal = 1u32;
        let  	mut c = 0usize;
        while c < maxRank
        {
            checkTotal = checkTotal.checked_mul( resDims[c]).ok_or( ShapeError::Overflow)?;
            c += 1;
        }

        return Ok( result);
    }

    pub fn	BroadcastCoord(
        targetCoords: Arr< '_, u32>,
        origShape: &TensorShape,
    ) -> [u32; K_MAX_TENSOR_RANK]
    {
        let  	mut mapped = [0u32; K_MAX_TENSOR_RANK];
        let  	targetRank = targetCoords.Size() as usize;
        let  	origRank = origShape.Rank() as usize;
        let  	origDims = origShape.DimsArray();

        let  	mut i = 0usize;
        while i < origRank
        {
            let  	tIdx = ( targetRank - 1 - i) as u32;
            let  	origDim = origDims[origRank - 1 - i];
            if origDim == 1
            {
                mapped[origRank - 1 - i] = 0;
            }
            else
            {
                mapped[origRank - 1 - i] = targetCoords[tIdx];
            }
            i += 1;
        }
        return mapped;
    }

    pub fn	BroadcastCoordSlice(
        targetCoords: &[u32],
        origShape: &TensorShape,
    ) -> [u32; K_MAX_TENSOR_RANK]
    {
        Self::BroadcastCoord( Arr::New( targetCoords.as_ptr(), targetCoords.len() as u32), origShape)
    }
}

//-------------------------------------------------------------------------------------------------

impl Default for TensorShape
{
    fn	default() -> Self
    {
        return Self::Scalar();
    }
}

//-------------------------------------------------------------------------------------------------

impl fmt::Debug for TensorShape
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        if self._Rank == 0
        {
            return write!( f, "Shape()");
        }
        write!( f, "Shape(")?;
        let  	dims = self.DimsArray();
        let  	mut i = 0usize;
        while i < self._Rank as usize
        {
            if i > 0
            {
                write!( f, ", ")?;
            }
            write!( f, "{}", dims[i])?;
            i += 1;
        }
        if self._Rank == 1
        {
            write!( f, ",")?;
        }
        return write!( f, ")");
    }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSource!( TensorShape, _Rank, _D0, _D1, _D2, _D3, _D4, _D5 );

//-------------------------------------------------------------------------------------------------
