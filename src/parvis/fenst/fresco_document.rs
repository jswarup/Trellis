// fresco_document.rs ------------------------------------------------------------------------------
//! Owned Fresco expression document, named roots, revisions, and validation boundaries.

use crate::fresco::{
    ExprReadError, ExprRepos, ExprView, K_MAX_TENSOR_RANK, LogicOp, TensorShape,
};
use crate::silo::{Buff, Stash, USeg};
use core::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------

static NEXT_DOC_ID: AtomicU64 = AtomicU64::new(1);

/// Allocates a process-unique document ID.
pub fn AllocateDocumentId() -> u64 { NEXT_DOC_ID.fetch_add(1, Ordering::Relaxed) }

//-------------------------------------------------------------------------------------------------

/// Validation limits for inspecting document expression DAGs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrescoDocLimits
{
    pub _MaxDepth:          u32,
    pub _MaxNodes:          u32,
    pub _MaxRoots:          u32,
    pub _MaxStringBytes:    u32,
    pub _MaxTotalTextBytes: u32,
}

impl Default for FrescoDocLimits
{
    fn default() -> Self
    {
        Self { _MaxDepth:          128,
               _MaxNodes:          10_000,
               _MaxRoots:          256,
               _MaxStringBytes:    1024,
               _MaxTotalTextBytes: 65536, }
    }
}

//-------------------------------------------------------------------------------------------------

/// Validation and document integrity errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrescoDocError
{
    NoRoots,
    RootLimitExceeded(u32),
    DuplicateRootName(String),
    InvalidExprId(u32),
    InvalidVarId(u32),
    EmptyNode(u32),
    DepthExceeded(u32),
    NodeBudgetExceeded(u32),
    CycleDetected(u32),
    UnsupportedNode(u32),
    InvalidArity(u32),
    InvalidShape,
    TextBudgetExceeded,
    InvalidTensorElementCount
    {
        expected: u32,
        actual:   u32,
    },
}

impl fmt::Display for FrescoDocError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::NoRoots => write!(f, "Document contains no roots"),
            Self::RootLimitExceeded(max) => write!(f, "Root count exceeds limit of {}", max),
            Self::DuplicateRootName(name) => write!(f, "Duplicate root name '{}'", name),
            Self::InvalidExprId(id) => write!(f, "Invalid expression ID {}", id),
            Self::InvalidVarId(id) => write!(f, "Invalid variable ID {}", id),
            Self::EmptyNode(id) => write!(f, "Empty expression entry at ID {}", id),
            Self::DepthExceeded(max) => write!(f, "Expression depth exceeds limit of {}", max),
            Self::NodeBudgetExceeded(max) => {
                write!(f, "Expression node count exceeds budget of {}", max)
            }
            Self::CycleDetected(id) => write!(f, "Cycle detected at expression ID {}", id),
            Self::UnsupportedNode(id) => write!(f, "Unsupported expression node type at ID {}", id),
            Self::InvalidArity(id) => {
                write!(f,
                       "Invalid operand count or partition at expression ID {}",
                       id)
            }
            Self::InvalidShape => write!(f, "Invalid tensor shape or overflowing element count"),
            Self::TextBudgetExceeded => {
                write!(f,
                       "Document string length or total text byte budget exceeded")
            }
            Self::InvalidTensorElementCount { expected, actual } => {
                write!(f,
                       "Tensor shape requires {} elements, found {}",
                       expected, actual)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// The kind of root presentation: a scalar expression or a tensor component grid.
#[derive(Clone, Debug, PartialEq)]
pub enum FrescoRootKind
{
    Scalar
    {
        _ExprId: u32
    },
    Tensor
    {
        _Shape: TensorShape,
        _Elems: Buff<u32>,
    },
}

//-------------------------------------------------------------------------------------------------

/// Named root in a Fresco document.
#[derive(Clone, Debug, PartialEq)]
pub struct FrescoRoot
{
    pub _Name: String,
    pub _Kind: FrescoRootKind,
}

impl FrescoRoot
{
    pub fn Scalar(name: impl Into<String>, exprId: u32) -> Self
    {
        Self { _Name: name.into(),
               _Kind: FrescoRootKind::Scalar { _ExprId: exprId }, }
    }

    pub fn Tensor(name: impl Into<String>, shape: TensorShape, elems: Buff<u32>) -> Self
    {
        Self { _Name: name.into(),
               _Kind: FrescoRootKind::Tensor { _Shape: shape,
                                               _Elems: elems, }, }
    }

    #[inline]
    pub fn Name(&self) -> &str { &self._Name }

    #[inline]
    pub fn Kind(&self) -> &FrescoRootKind { &self._Kind }

    pub fn IsTensor(&self) -> bool { matches!(self._Kind, FrescoRootKind::Tensor { .. }) }

    pub fn Shape(&self) -> Option<&TensorShape>
    {
        match &self._Kind {
            FrescoRootKind::Scalar { .. } => None,
            FrescoRootKind::Tensor { _Shape, .. } => Some(_Shape),
        }
    }

    pub fn PrimaryExprId(&self) -> u32
    {
        match &self._Kind {
            FrescoRootKind::Scalar { _ExprId } => *_ExprId,
            FrescoRootKind::Tensor { _Elems, .. } => {
                if _Elems.Size() > 0 {
                    _Elems[0]
                } else {
                    u32::MAX
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Owned Fresco document with unique identity, revision, repository, and named roots.
pub struct FrescoDocument
{
    _Id:       u64,
    _Revision: u64,
    _Repos:    ExprRepos,
    _Roots:    Buff<FrescoRoot>,
}

impl FrescoDocument
{
    /// Creates a new document with an allocated identity and revision 1.
    pub fn New(repos: ExprRepos, roots: Buff<FrescoRoot>) -> Result<Self, FrescoDocError>
    {
        let limits = FrescoDocLimits::default();
        let doc = Self { _Id:       AllocateDocumentId(),
                         _Revision: 1,
                         _Repos:    repos,
                         _Roots:    roots, };
        doc.Validate(&limits)?;
        Ok(doc)
    }

    /// Creates a document with a specific ID and revision.
    pub fn WithIdentity(id: u64, revision: u64, repos: ExprRepos, roots: Buff<FrescoRoot>)
                        -> Result<Self, FrescoDocError>
    {
        let limits = FrescoDocLimits::default();
        let doc = Self { _Id:       id,
                         _Revision: revision,
                         _Repos:    repos,
                         _Roots:    roots, };
        doc.Validate(&limits)?;
        Ok(doc)
    }

    #[inline]
    pub const fn Id(&self) -> u64 { self._Id }

    #[inline]
    pub const fn Revision(&self) -> u64 { self._Revision }

    #[inline]
    pub fn Repos(&self) -> &ExprRepos { &self._Repos }

    #[inline]
    pub fn Roots(&self) -> &Buff<FrescoRoot> { &self._Roots }

    #[inline]
    pub const fn RootCount(&self) -> u32 { self._Roots.Size() }

    pub fn RootAt(&self, index: u32) -> Option<&FrescoRoot>
    {
        if index < self._Roots.Size() {
            Some(&self._Roots[index])
        } else {
            None
        }
    }

    /// Validates all roots and their reachable subgraphs against the specified limits.
    pub fn Validate(&self, limits: &FrescoDocLimits) -> Result<(), FrescoDocError>
    {
        let rootCount = self._Roots.Size();
        if rootCount == 0 {
            return Err(FrescoDocError::NoRoots);
        }
        if rootCount > limits._MaxRoots {
            return Err(FrescoDocError::RootLimitExceeded(limits._MaxRoots));
        }

        // Check bytes before comparing, formatting, cloning or shaping names.
        let mut totalTextBytes = 0u32;
        let mut rootIndex = 0u32;
        while rootIndex < rootCount {
            Self::ValidateText(self._Roots[rootIndex].Name(), limits, &mut totalTextBytes)?;
            rootIndex += 1;
        }

        // Check for duplicate root names
        let mut i = 0u32;
        while i < rootCount {
            let nameI = self._Roots[i].Name();
            let mut j = i + 1;
            while j < rootCount {
                if self._Roots[j].Name() == nameI {
                    return Err(FrescoDocError::DuplicateRootName(nameI.to_string()));
                }
                j += 1;
            }
            i += 1;
        }

        // Validate each root's expression DAG using an explicit work stack
        let mut totalNodesVisited = 0u32;
        let mut r = 0u32;
        while r < rootCount {
            match &self._Roots[r]._Kind {
                FrescoRootKind::Scalar { _ExprId } => {
                    self.ValidateSubtree(*_ExprId,
                                         limits,
                                         &mut totalNodesVisited,
                                         &mut totalTextBytes)?;
                }
                FrescoRootKind::Tensor { _Shape, _Elems } => {
                    if _Shape.Rank() == 0 {
                        return Err(FrescoDocError::InvalidShape);
                    }
                    let expected = Self::ValidateShape(_Shape)?;
                    if expected != _Elems.Size() {
                        return Err(FrescoDocError::InvalidTensorElementCount { expected,
                                                                               actual:
                                                                                   _Elems.Size() });
                    }
                    let elemCount = _Elems.Size();
                    let mut e = 0u32;
                    while e < elemCount {
                        self.ValidateSubtree(_Elems[e],
                                             limits,
                                             &mut totalNodesVisited,
                                             &mut totalTextBytes)?;
                        e += 1;
                    }
                }
            }
            r += 1;
        }

        Ok(())
    }

    fn ValidateShape(shape: &TensorShape) -> Result<u32, FrescoDocError>
    {
        if shape.Rank() > K_MAX_TENSOR_RANK as u32 {
            return Err(FrescoDocError::InvalidShape);
        }
        let mut count = 1u32;
        let mut valid = true;
        USeg::FromLen(shape.Rank()).Traverse(|axis| {
                                       let dim = shape.Dim(axis);
                                       match count.checked_mul(dim) {
                                           Some(next) if dim > 0 => count = next,
                                           _ => valid = false,
                                       }
                                   });
        if !valid {
            return Err(FrescoDocError::InvalidShape);
        }
        return Ok(count);
    }

    fn ValidateText(text: &str, limits: &FrescoDocLimits, total: &mut u32)
                    -> Result<(), FrescoDocError>
    {
        let len = u32::try_from(text.len()).map_err(|_| FrescoDocError::TextBudgetExceeded)?;
        if len > limits._MaxStringBytes || len > limits._MaxTotalTextBytes.saturating_sub(*total) {
            return Err(FrescoDocError::TextBudgetExceeded);
        }
        *total += len;
        return Ok(());
    }

    fn ValidateSubtree(&self, rootId: u32, limits: &FrescoDocLimits, totalNodes: &mut u32,
                       totalTextBytes: &mut u32)
                       -> Result<(), FrescoDocError>
    {
        // Work stack item: (node_id, depth)
        let mut stack = Stash::<(u32, u32)>::WithCapacity(64);
        let mut pathAncestors = Stash::<u32>::WithCapacity(64);
        stack.Push((rootId, 0));

        while stack.Size() > 0 {
            let (node_id, depth) = stack.Pop().unwrap();

            if depth > limits._MaxDepth {
                return Err(FrescoDocError::DepthExceeded(limits._MaxDepth));
            }

            *totalNodes = totalNodes.saturating_add(1);
            if *totalNodes > limits._MaxNodes {
                return Err(FrescoDocError::NodeBudgetExceeded(limits._MaxNodes));
            }

            if !self._Repos.IsValidExprId(node_id) {
                return Err(FrescoDocError::InvalidExprId(node_id));
            }

            // Adjust ancestor path tracking to current depth
            pathAncestors.Resize(depth, |_| 0);
            // Check for cycles along current ancestor path
            let mut a = 0u32;
            let ancLen = pathAncestors.Size();
            while a < ancLen {
                if pathAncestors[a] == node_id {
                    return Err(FrescoDocError::CycleDetected(node_id));
                }
                a += 1;
            }
            pathAncestors.Push(node_id);

            let view = match self._Repos.Inspect(node_id) {
                Ok(v) => v,
                Err(ExprReadError::InvalidId(_)) => return Err(FrescoDocError::InvalidExprId(node_id)),
                Err(ExprReadError::EmptyNode(_)) => return Err(FrescoDocError::EmptyNode(node_id)),
                Err(ExprReadError::UnsupportedNode(_)) => return Err(FrescoDocError::UnsupportedNode(node_id)),
            };

            match view {
                ExprView::Real(_) => {
                    // Leaf
                }
                ExprView::Var(varIndex) => {
                    if !self._Repos.IsValidVarId(varIndex) {
                        return Err(FrescoDocError::InvalidVarId(varIndex));
                    }
                    Self::ValidateShape(self._Repos.VarAttrAt(varIndex).Shape())?;
                    Self::ValidateText(self._Repos.VarNameAt(varIndex),
                                       limits,
                                       totalTextBytes)?;
                }
                ExprView::Sum(poly) => {
                    let count = poly.SizeChild();
                    if poly.CommonSize() > count {
                        return Err(FrescoDocError::InvalidArity(node_id));
                    }
                    if count
                       > limits._MaxNodes
                               .saturating_sub(*totalNodes)
                               .saturating_sub(stack.Size())
                    {
                        return Err(FrescoDocError::NodeBudgetExceeded(limits._MaxNodes));
                    }
                    let mut c = 0u32;
                    while c < count {
                        stack.Push((poly.Child(c), depth + 1));
                        c += 1;
                    }
                }
                ExprView::Prod(poly) => {
                    let count = poly.SizeChild();
                    if poly.CommonSize() > count {
                        return Err(FrescoDocError::InvalidArity(node_id));
                    }
                    if count
                       > limits._MaxNodes
                               .saturating_sub(*totalNodes)
                               .saturating_sub(stack.Size())
                    {
                        return Err(FrescoDocError::NodeBudgetExceeded(limits._MaxNodes));
                    }
                    let mut c = 0u32;
                    while c < count {
                        stack.Push((poly.Child(c), depth + 1));
                        c += 1;
                    }
                }
                ExprView::Pow(poly) => {
                    let count = poly.SizeChild();
                    if count != 2 || poly.CommonSize() != 1 {
                        return Err(FrescoDocError::InvalidArity(node_id));
                    }
                    let mut c = 0u32;
                    while c < count {
                        stack.Push((poly.Child(c), depth + 1));
                        c += 1;
                    }
                }
                ExprView::Cond { condition, ifTrue, ifFalse } => {
                    stack.Push((condition, depth + 1));
                    stack.Push((ifTrue, depth + 1));
                    stack.Push((ifFalse, depth + 1));
                }
                ExprView::Func { name, arg } => {
                    Self::ValidateText(name, limits, totalTextBytes)?;
                    stack.Push((arg, depth + 1));
                }
                ExprView::Logic { op, left, right } => {
                    let isUnary = op == LogicOp::Not;
                    stack.Push((left, depth + 1));
                    if !isUnary {
                        stack.Push((right, depth + 1));
                    }
                }
            }
        }

        Ok(())
    }

    /// Creates a built-in demonstration document with rich scalar and tensor expressions.
    pub fn CreateExample() -> Self
    {
        let mut repos = ExprRepos::New();

        // 1. Quadratic discriminant & formula root
        // d = b^2 - 4*a*c
        let a = repos.VarCreate("a".to_string(), true);
        let b = repos.VarCreate("b".to_string(), true);
        let c = repos.VarCreate("c".to_string(), true);
        let two = repos.RealCreate(2.0);
        let four = repos.RealCreate(4.0);

        let bSq = repos.PowerCreate(b, two);
        let ac = repos.MultCreate(a, c);
        let fourAC = repos.MultCreate(four, ac);
        let disc = repos.DiffCreate(bSq, fourAC);

        // num = -b + sqrt(disc) = -b + disc^0.5
        let half = repos.RealCreate(0.5);
        let sqrtDisc = repos.PowerCreate(disc, half);
        let zeroVal = repos.RealCreate(0.0);
        let negB = repos.DiffCreate(zeroVal, b);
        let num = repos.AddCreate(negB, sqrtDisc);
        let denom = repos.MultCreate(two, a);
        let quadRoot = repos.DivCreate(num, denom);

        // 2. Rational function: (x^2 - 1) / (x + 2)
        let x = repos.VarCreate("x".to_string(), true);
        let one = repos.RealCreate(1.0);
        let xSq = repos.PowerCreate(x, two);
        let numRat = repos.DiffCreate(xSq, one);
        let denomRat = repos.AddCreate(x, two);
        let rationalRoot = repos.DivCreate(numRat, denomRat);

        // 3. Conditional ReLU: cond(x > 0, x, 0)
        let zero = repos.RealCreate(0.0);
        let condGt = repos.LogicCreate(crate::fresco::LogicOp::Gt, x, zero);
        let reluRoot = repos.CondCreate(condGt, x, zero);

        // 4. Matrix-vector product components: A (2x2) * v (2x1)
        // A00*v0 + A01*v1
        // A10*v0 + A11*v1
        let a00 = repos.VarCreate("A[0,0]".to_string(), true);
        let a01 = repos.VarCreate("A[0,1]".to_string(), true);
        let a10 = repos.VarCreate("A[1,0]".to_string(), true);
        let a11 = repos.VarCreate("A[1,1]".to_string(), true);
        let v0 = repos.VarCreate("v[0]".to_string(), true);
        let v1 = repos.VarCreate("v[1]".to_string(), true);

        let term00 = repos.MultCreate(a00, v0);
        let term01 = repos.MultCreate(a01, v1);
        let row0 = repos.AddCreate(term00, term01);

        let term10 = repos.MultCreate(a10, v0);
        let term11 = repos.MultCreate(a11, v1);
        let row1 = repos.AddCreate(term10, term11);

        let mut matElems = Stash::<u32>::WithCapacity(4);
        matElems.Push(a00);
        matElems.Push(a01);
        matElems.Push(a10);
        matElems.Push(a11);
        let matShape = TensorShape::Matrix(2, 2).unwrap();

        let mut mvElems = Stash::<u32>::WithCapacity(2);
        mvElems.Push(row0);
        mvElems.Push(row1);
        let vecShape = TensorShape::Vector(2).unwrap();

        let mut roots = Stash::<FrescoRoot>::WithCapacity(5);
        roots.Push(FrescoRoot::Scalar("QuadraticRoot", quadRoot));
        roots.Push(FrescoRoot::Scalar("Discriminant", disc));
        roots.Push(FrescoRoot::Scalar("RationalFormula", rationalRoot));
        roots.Push(FrescoRoot::Scalar("ReLU", reluRoot));
        roots.Push(FrescoRoot::Tensor("Matrix_A", matShape, matElems.IntoBuff()));
        roots.Push(FrescoRoot::Tensor("MatrixVecProd", vecShape, mvElems.IntoBuff()));

        Self { _Id:       AllocateDocumentId(),
               _Revision: 1,
               _Repos:    repos,
               _Roots:    roots.IntoBuff(), }
    }
}
