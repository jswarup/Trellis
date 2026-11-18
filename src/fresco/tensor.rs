// tensor.rs --------------------------------------------------------------------------------------
use	core::fmt;
use	crate::{
    fresco::{
        exprrepos::ExprRepos,
        shape::{ShapeError, TensorShape},
        varexpr::VarExpr,
    },
    silo::{Arr, Buff, Stash},
};

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub enum TensorError
{
    Shape( ShapeError),
    DimensionMismatch
    {
        expected: u32,
        actual:   u32,
    },
    RankMismatch
    {
        expected: u32,
        actual:   u32,
    },
    EmptyTensor,
    InvalidElementCount
    {
        expected: u32,
        actual:   u32,
    },
    InvalidVarId( u32),
    InvalidExprId( u32),
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
}

//-------------------------------------------------------------------------------------------------

impl From< ShapeError> for TensorError
{
    fn	from( err: ShapeError) -> Self
    {
        Self::Shape( err)
    }
}

//-------------------------------------------------------------------------------------------------

impl fmt::Display for TensorError
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        match self
        {
            Self::Shape( err) => write!( f, "Shape error: {}", err),
            Self::DimensionMismatch { expected, actual } =>
            {
                write!( f, "Dimension mismatch: expected {}, got {}", expected, actual)
            }
            Self::RankMismatch { expected, actual } =>
            {
                write!( f, "Rank mismatch: expected {}, got {}", expected, actual)
            }
            Self::EmptyTensor => write!( f, "Tensor cannot be empty"),
            Self::InvalidElementCount { expected, actual } =>
            {
                write!( f, "Invalid element count: expected {}, got {}", expected, actual)
            }
            Self::InvalidVarId( id) => write!( f, "Invalid variable ID: {}", id),
            Self::InvalidExprId( id) => write!( f, "Invalid expression ID: {}", id),
            Self::IndexOutOfBounds { axis, index, dim } =>
            {
                write!( f, "Index {} out of bounds for axis {} with dim {}", index, axis, dim)
            }
            Self::LinearIndexOutOfBounds { index, total } =>
            {
                write!( f, "Linear index {} out of bounds for tensor of size {}", index, total)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct TensorExpr
{
    pub(crate) _Shape: TensorShape,
    pub(crate) _Elems: Buff< u32>,
}

//-------------------------------------------------------------------------------------------------

impl TensorExpr
{
    pub fn	New( shape: TensorShape, elems: Buff< u32>) -> Result< Self, TensorError>
    {
        let  	total = shape.TotalElements();
        if elems.Size() != total
        {
            return Err( TensorError::InvalidElementCount {
                expected: total,
                actual:   elems.Size(),
            });
        }
        return Ok( Self { _Shape: shape, _Elems: elems });
    }

    pub fn	FromScalar( exprId: u32) -> Self
    {
        let  	elems = Buff::FromArr( Arr::from( &[exprId]));
        Self {
            _Shape: TensorShape::Scalar(),
            _Elems: elems,
        }
    }

    pub fn	Zeros( repos: &mut ExprRepos, shape: TensorShape) -> Self
    {
        let  	total = shape.TotalElements();
        let  	zeroExpr = repos.RealCreate( 0.0);
        let  	mut stash = Stash::WithCapacity( total);
        let  	mut i = 0u32;
        while i < total
        {
            stash.Push( zeroExpr);
            i += 1;
        }
        Self {
            _Shape: shape,
            _Elems: stash.IntoBuff(),
        }
    }

    pub fn	FromReals(
        repos: &mut ExprRepos,
        shape: TensorShape,
        vals: Arr< '_, f64>,
    ) -> Result< Self, TensorError>
    {
        let  	total = shape.TotalElements();
        if vals.Size() != total
        {
            return Err( TensorError::InvalidElementCount {
                expected: total,
                actual:   vals.Size(),
            });
        }
        let  	mut stash = Stash::WithCapacity( total);
        let  	mut i = 0u32;
        while i < total
        {
            stash.Push( repos.RealCreate( vals[i]));
            i += 1;
        }
        return Ok( Self {
            _Shape: shape,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	FromRealsSlice(
        repos: &mut ExprRepos,
        shape: TensorShape,
        vals: &[f64],
    ) -> Result< Self, TensorError>
    {
        Self::FromReals( repos, shape, Arr::New( vals.as_ptr(), vals.len() as u32))
    }

    pub fn	FromElements( shape: TensorShape, elems: Buff< u32>) -> Result< Self, TensorError>
    {
        Self::New( shape, elems)
    }

    pub fn	FromVar( repos: &mut ExprRepos, varId: u32) -> Result< Self, TensorError>
    {
        if !repos.IsValidVarId( varId)
        {
            return Err( TensorError::InvalidVarId( varId));
        }
        let  	shape = *repos.VarAttrAt( varId).Shape();
        if shape.IsScalar()
        {
            let  	exprId = repos.Store( Box::new( VarExpr::New( varId)));
            return Ok( Self::FromScalar( exprId));
        }

        let  	total = shape.TotalElements();
        let  	mut stash = Stash::WithCapacity( total);
        let  	mut k = 0u32;
        while k < total
        {
            let  	leafId = repos.SubVarLinear( varId, k)?;
            let  	exprId = repos.Store( Box::new( VarExpr::New( leafId)));
            stash.Push( exprId);
            k += 1;
        }
        return Ok( Self {
            _Shape: shape,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	Shape( &self) -> &TensorShape
    {
        &self._Shape
    }

    pub fn	Elems( &self) -> &Buff< u32>
    {
        &self._Elems
    }

    pub fn	TotalElements( &self) -> u32
    {
        self._Shape.TotalElements()
    }

    pub fn	Rank( &self) -> u32
    {
        self._Shape.Rank()
    }

    pub fn	Dim( &self, axis: u32) -> u32
    {
        self._Shape.Dim( axis)
    }

    pub fn	ElemLinear( &self, linear: u32) -> Result< u32, TensorError>
    {
        let  	total = self.TotalElements();
        if linear >= total
        {
            return Err( TensorError::LinearIndexOutOfBounds {
                index: linear,
                total,
            });
        }
        return Ok( self._Elems[linear]);
    }

    pub fn	Elem( &self, indices: Arr< '_, u32>) -> Result< u32, TensorError>
    {
        let  	linear = self._Shape.LinearIndex( indices)?;
        return Ok( self._Elems[linear]);
    }

    pub fn	ElemSlice( &self, indices: &[u32]) -> Result< u32, TensorError>
    {
        self.Elem( Arr::New( indices.as_ptr(), indices.len() as u32))
    }

    pub fn	SubTensor( &self, i: u32) -> Result< Self, TensorError>
    {
        let  	rank = self._Shape.Rank();
        if rank == 0
        {
            return Err( TensorError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	d0 = self._Shape.Dim( 0);
        if i >= d0
        {
            return Err( TensorError::IndexOutOfBounds {
                axis:  0,
                index: i,
                dim:   d0,
            });
        }
        let  	subShape = self._Shape.SubShape()?;
        let  	subCount = subShape.TotalElements();
        let  	start = i * subCount;
        let  	mut stash = Stash::WithCapacity( subCount);
        let  	mut k = 0u32;
        while k < subCount
        {
            stash.Push( self._Elems[start + k]);
            k += 1;
        }
        return Ok( Self {
            _Shape: subShape,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	Row( &self, r: u32) -> Result< Self, TensorError>
    {
        if self.Rank() != 2
        {
            return Err( TensorError::RankMismatch { expected: 2, actual: self.Rank() });
        }
        return self.SubTensor( r);
    }

    pub fn	Col( &self, c: u32) -> Result< Self, TensorError>
    {
        if self.Rank() != 2
        {
            return Err( TensorError::RankMismatch { expected: 2, actual: self.Rank() });
        }
        let  	rows = self.Dim( 0);
        let  	cols = self.Dim( 1);
        if c >= cols
        {
            return Err( TensorError::IndexOutOfBounds {
                axis:  1,
                index: c,
                dim:   cols,
            });
        }
        let  	mut stash = Stash::WithCapacity( rows);
        let  	mut r = 0u32;
        while r < rows
        {
            stash.Push( self._Elems[r * cols + c]);
            r += 1;
        }
        return Ok( Self {
            _Shape: TensorShape::Vector( rows)?,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	Transpose( &self) -> Result< Self, TensorError>
    {
        if self.Rank() != 2
        {
            return Err( TensorError::RankMismatch { expected: 2, actual: self.Rank() });
        }
        let  	rows = self.Dim( 0);
        let  	cols = self.Dim( 1);
        let  	mut stash = Stash::WithCapacity( rows * cols);
        let  	mut c = 0u32;
        while c < cols
        {
            let  	mut r = 0u32;
            while r < rows
            {
                stash.Push( self._Elems[r * cols + c]);
                r += 1;
            }
            c += 1;
        }
        return Ok( Self {
            _Shape: TensorShape::Matrix( cols, rows)?,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	Reshape( &self, newShape: TensorShape) -> Result< Self, TensorError>
    {
        let  	curTotal = self.TotalElements();
        let  	newTotal = newShape.TotalElements();
        if curTotal != newTotal
        {
            return Err( TensorError::DimensionMismatch {
                expected: curTotal,
                actual:   newTotal,
            });
        }
        return Ok( Self {
            _Shape: newShape,
            _Elems: self._Elems.clone(),
        });
    }

    pub fn	Flatten( &self) -> Result< Self, TensorError>
    {
        let  	total = self.TotalElements();
        Ok( Self {
            _Shape: TensorShape::Vector( total)?,
            _Elems: self._Elems.clone(),
        })
    }

    pub fn	VectorDot(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< u32, TensorError>
    {
        if !a._Shape.IsVector() || !b._Shape.IsVector()
        {
            return Err( TensorError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	len = a.Dim( 0);
        if b.Dim( 0) != len
        {
            return Err( TensorError::DimensionMismatch {
                expected: len,
                actual:   b.Dim( 0),
            });
        }
        if len == 0
        {
            return Ok( repos.RealCreate( 0.0));
        }

        let  	mut prodStash = Stash::WithCapacity( len);
        let  	mut k = 0u32;
        while k < len
        {
            let  	prod = repos.MultCreate( a._Elems[k], b._Elems[k]);
            prodStash.Push( prod);
            k += 1;
        }
        let  	prodArr = prodStash.Arr();
        return Ok( repos.SumCreate( prodArr, Arr::Empty()));
    }

    pub fn	MatVecMul(
        repos: &mut ExprRepos,
        mat: &Self,
        vec: &Self,
    ) -> Result< Self, TensorError>
    {
        if mat.Rank() != 2
        {
            return Err( TensorError::RankMismatch { expected: 2, actual: mat.Rank() });
        }
        if !vec._Shape.IsVector()
        {
            return Err( TensorError::RankMismatch { expected: 1, actual: vec.Rank() });
        }
        let  	rows = mat.Dim( 0);
        let  	cols = mat.Dim( 1);
        if vec.Dim( 0) != cols
        {
            return Err( TensorError::DimensionMismatch {
                expected: cols,
                actual:   vec.Dim( 0),
            });
        }

        let  	mut stash = Stash::WithCapacity( rows);
        let  	mut r = 0u32;
        while r < rows
        {
            let  	row = mat.SubTensor( r)?;
            let  	dot = Self::VectorDot( repos, &row, vec)?;
            stash.Push( dot);
            r += 1;
        }
        return Ok( Self {
            _Shape: TensorShape::Vector( rows)?,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	MatMul(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< Self, TensorError>
    {
        if a.Rank() != 2 || b.Rank() != 2
        {
            return Err( TensorError::RankMismatch { expected: 2, actual: 0 });
        }
        let  	rowsA = a.Dim( 0);
        let  	colsA = a.Dim( 1);
        let  	rowsB = b.Dim( 0);
        let  	colsB = b.Dim( 1);
        if colsA != rowsB
        {
            return Err( TensorError::DimensionMismatch {
                expected: colsA,
                actual:   rowsB,
            });
        }

        let  	mut stash = Stash::WithCapacity( rowsA * colsB);
        let  	mut prodStash = Stash::WithCapacity( colsA);
        let  	mut r = 0u32;
        while r < rowsA
        {
            let  	mut c = 0u32;
            while c < colsB
            {
                prodStash.Clear();
                let  	mut k = 0u32;
                while k < colsA
                {
                    let  	ea = a._Elems[r * colsA + k];
                    let  	eb = b._Elems[k * colsB + c];
                    let  	p = repos.MultCreate( ea, eb);
                    prodStash.Push( p);
                    k += 1;
                }
                let  	dot = repos.SumCreate( prodStash.Arr(), Arr::Empty());
                stash.Push( dot);
                c += 1;
            }
            r += 1;
        }
        return Ok( Self {
            _Shape: TensorShape::Matrix( rowsA, colsB)?,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	OuterProduct(
        repos: &mut ExprRepos,
        u: &Self,
        v: &Self,
    ) -> Result< Self, TensorError>
    {
        if !u._Shape.IsVector() || !v._Shape.IsVector()
        {
            return Err( TensorError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	m = u.Dim( 0);
        let  	n = v.Dim( 0);
        let  	mut stash = Stash::WithCapacity( m * n);
        let  	mut i = 0u32;
        while i < m
        {
            let  	mut j = 0u32;
            while j < n
            {
                let  	prod = repos.MultCreate( u._Elems[i], v._Elems[j]);
                stash.Push( prod);
                j += 1;
            }
            i += 1;
        }
        return Ok( Self {
            _Shape: TensorShape::Matrix( m, n)?,
            _Elems: stash.IntoBuff(),
        });
    }

    pub fn	TensorAdd(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< Self, TensorError>
    {
        Self::ElementwiseBinary( repos, a, b, |r, ea, eb| r.AddCreate( ea, eb))
    }

    pub fn	TensorSub(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< Self, TensorError>
    {
        Self::ElementwiseBinary( repos, a, b, |r, ea, eb| r.DiffCreate( ea, eb))
    }

    pub fn	TensorHadamard(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< Self, TensorError>
    {
        Self::ElementwiseBinary( repos, a, b, |r, ea, eb| r.MultCreate( ea, eb))
    }

    pub fn	TensorDiv(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
    ) -> Result< Self, TensorError>
    {
        Self::ElementwiseBinary( repos, a, b, |r, ea, eb| r.DivCreate( ea, eb))
    }

    pub fn	TensorScale(
        repos: &mut ExprRepos,
        a: &Self,
        scalarExpr: u32,
    ) -> Self
    {
        let  	total = a.TotalElements();
        let  	mut stash = Stash::WithCapacity( total);
        let  	mut k = 0u32;
        while k < total
        {
            let  	prod = repos.MultCreate( a._Elems[k], scalarExpr);
            stash.Push( prod);
            k += 1;
        }
        Self {
            _Shape: a._Shape,
            _Elems: stash.IntoBuff(),
        }
    }

    fn	ElementwiseBinary< F>(
        repos: &mut ExprRepos,
        a: &Self,
        b: &Self,
        op: F,
    ) -> Result< Self, TensorError>
    where
        F: Fn( &mut ExprRepos, u32, u32) -> u32,
    {
        let  	outShape = a._Shape.BroadcastWith( &b._Shape)?;
        let  	total = outShape.TotalElements();
        let  	mut stash = Stash::WithCapacity( total);

        let  	rankA = a._Shape.Rank() as usize;
        let  	rankB = b._Shape.Rank() as usize;
        let  	outRank = outShape.Rank() as usize;

        let  	mut k = 0u32;
        while k < total
        {
            let  	targetCoords = outShape.MultiIndex( k)?;
            let  	targetArr = Arr::New( targetCoords.as_ptr(), outRank as u32);
            let  	coordsA = TensorShape::BroadcastCoord( targetArr, &a._Shape);
            let  	coordsB = TensorShape::BroadcastCoord( targetArr, &b._Shape);

            let  	idxA = a._Shape.LinearIndex( Arr::New( coordsA.as_ptr(), rankA as u32))?;
            let  	idxB = b._Shape.LinearIndex( Arr::New( coordsB.as_ptr(), rankB as u32))?;

            let  	resExpr = op( repos, a._Elems[idxA], b._Elems[idxB]);
            stash.Push( resExpr);
            k += 1;
        }

        return Ok( Self {
            _Shape: outShape,
            _Elems: stash.IntoBuff(),
        });
    }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSource!( TensorExpr, _Shape, _Elems );

//-------------------------------------------------------------------------------------------------
