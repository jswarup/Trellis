// fresco_properties.rs ----------------------------------------------------------------------------
//! Read-only Fresco properties shared by workbench front ends.

use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fenst::math_layout::{MathCoordinates, MathOccurrence};
use crate::fresco::{
    EvalCounters, EvalError, EvalLimits, ExprReadError, ExprView, TensorShape, VarKind,
};
use crate::silo::{Arr, Stash};

//-------------------------------------------------------------------------------------------------

pub struct FrescoProperties<'a>
{
    pub DocumentId: u64,
    pub Revision:   u64,
    pub Name:       &'a str,
    pub Shape:      Option<&'a TensorShape>,
    pub Term:       Option<FrescoTermProperties<'a>>,
}

pub struct FrescoTermProperties<'a>
{
    pub ExprId:       u32,
    pub OccurrenceId: u32,
    pub Coordinates:  Option<MathCoordinates>,
    pub Value:        Result<FrescoTermValue<'a>, ExprReadError>,
}

pub enum FrescoTermValue<'a>
{
    Constant(f64),
    Variable { name: &'a str, index: u32, nonZero: bool },
    Sum { add: u32, subtract: u32 },
    Product { numerator: u32, denominator: u32 },
    Power,
    Function(&'a str),
    Logic,
    Conditional,
}

impl<'a> FrescoProperties<'a>
{
    /// Resolves a semantic root or tensor component without constructing a canvas layout.
    pub fn ReadSelection(doc: &'a FrescoDocument, rootName: Option<&str>, coordinates: Arr<'_, u32>)
                         -> Result<Self, String>
    {
        let root = match rootName {
            None => doc.RootAt(0),
            Some(name) => {
                let mut found = None;
                doc.Roots().USeg().Span(|index| {
                    let root = &doc.Roots()[index];
                    if root.Name() == name {
                        found = Some(root);
                        return false;
                    }
                    return true;
                });
                found
            }
        }.ok_or_else(|| format!("Fresco root not found: {}", rootName.unwrap_or("<first root>")))?;
        let (exprId, component) = match root.Kind() {
            FrescoRootKind::Scalar { _ExprId } if coordinates.IsEmpty() => (Some(*_ExprId), None),
            FrescoRootKind::Scalar { .. } => return Err("A scalar root has no coordinates".into()),
            FrescoRootKind::Tensor { .. } if coordinates.IsEmpty() => (None, None),
            FrescoRootKind::Tensor { _Shape, _Elems, .. } => {
                let index = _Shape.LinearIndex(coordinates).map_err(|error| error.to_string())?;
                let component = match coordinates.Size() {
                    1 => MathCoordinates::Vector(coordinates[0]),
                    2 => MathCoordinates::Matrix(coordinates[0], coordinates[1]),
                    _ => return Err("Component inspection supports vectors and matrices".into()),
                };
                (Some(_Elems[index]), Some(component))
            }
        };
        let term = exprId.map(|id| Self::ReadTerm(doc, id, u32::MAX, component));
        return Ok(Self { DocumentId: doc.Id(), Revision: doc.Revision(), Name: root.Name(),
                         Shape: root.Shape(), Term: term });
    }

    pub fn ShapeLabel(&self) -> Option<String>
    {
        return self.Shape.map(|shape| match shape.Rank() {
            0 => "Scalar".to_string(),
            1 => format!("Vector [{}]", shape.Dim(0)),
            2 => format!("Matrix [{} × {}]", shape.Dim(0), shape.Dim(1)),
            rank => format!("Tensor (rank {rank})"),
        });
    }

    /// The occurrence, if supplied, comes from the layout for this document revision.
    pub fn Read(doc: &'a FrescoDocument, rootIndex: u32, occurrence: Option<&MathOccurrence>)
                -> Option<Self>
    {
        let root = doc.RootAt(rootIndex)?;
        let term = occurrence.filter(|occ| occ._RootIndex == rootIndex).map(|occ| {
            Self::ReadTerm(doc, occ._ExprId, occ._Id, occ._Coords)
        });
        return Some(Self { DocumentId: doc.Id(),
                           Revision:   doc.Revision(),
                           Name:       root.Name(),
                           Shape:      root.Shape(),
                           Term:       term, });
    }

    fn ReadTerm(doc: &'a FrescoDocument, exprId: u32, occurrenceId: u32,
                coordinates: Option<MathCoordinates>) -> FrescoTermProperties<'a>
    {
        let repos = doc.Repos();
        let value = repos.Inspect(exprId).map(|view| match view {
            ExprView::Real(value) => FrescoTermValue::Constant(value),
            ExprView::Var(index) => FrescoTermValue::Variable {
                name: repos.VarNameAt(index),
                index,
                nonZero: repos.VarAttrAt(index).HasBits(VarKind::NonZero),
            },
            ExprView::Sum(poly) => FrescoTermValue::Sum {
                add:      poly.CommonSize(),
                subtract: poly.SizeChild() - poly.CommonSize(),
            },
            ExprView::Prod(poly) => FrescoTermValue::Product {
                numerator:   poly.CommonSize(),
                denominator: poly.SizeChild() - poly.CommonSize(),
            },
            ExprView::Pow(_) => FrescoTermValue::Power,
            ExprView::Func { name, .. } => FrescoTermValue::Function(name),
            ExprView::Logic { .. } => FrescoTermValue::Logic,
            ExprView::Cond { .. } => FrescoTermValue::Conditional,
        });
        return FrescoTermProperties { ExprId: exprId, OccurrenceId: occurrenceId,
                                      Coordinates: coordinates, Value: value };
    }
}

impl FrescoTermProperties<'_>
{
    pub fn Summary(&self) -> String
    {
        return match &self.Value {
            Ok(FrescoTermValue::Constant(value)) => format!("Const({value})"),
            Ok(FrescoTermValue::Variable { name, .. }) => format!("Var({name})"),
            Ok(FrescoTermValue::Sum { add, subtract }) => format!("Sum({add} add, {subtract} sub)"),
            Ok(FrescoTermValue::Product { numerator, denominator }) => format!("Prod({numerator}/{denominator})"),
            Ok(FrescoTermValue::Power) => "Power".into(),
            Ok(FrescoTermValue::Function(name)) => format!("Func({name})"),
            Ok(FrescoTermValue::Logic) => "Logic".into(),
            Ok(FrescoTermValue::Conditional) => "Conditional".into(),
            Err(error) => format!("Error({error})"),
        };
    }
}

//-------------------------------------------------------------------------------------------------

/// Stale evaluation errors rejecting responses whose generation does not match the active session.
#[derive(Clone, Debug, PartialEq)]
pub enum StaleEvalError
{
    DocumentMismatch,
    StaleDocumentRevision
    {
        current:  u64,
        received: u64,
    },
    StaleBindingGeneration
    {
        current:  u64,
        received: u64,
    },
    StaleRequestGeneration
    {
        latest_completed: u64,
        received:         u64,
    },
    EvalFailed(EvalError),
}

impl core::fmt::Display for StaleEvalError
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        match self
        {
            Self::DocumentMismatch => write!(f, "Document ID mismatch"),
            Self::StaleDocumentRevision { current, received } =>
            {
                write!(f, "Stale document revision: current {}, received {}", current, received)
            }
            Self::StaleBindingGeneration { current, received } =>
            {
                write!(f, "Stale binding generation: current {}, received {}", current, received)
            }
            Self::StaleRequestGeneration { latest_completed, received } =>
            {
                write!(f, "Stale request generation: latest completed {}, received {}", latest_completed, received)
            }
            Self::EvalFailed(err) => write!(f, "Evaluation failed: {}", err),
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Request for numerical evaluation tagged with generation counters.
#[derive(Clone, Debug)]
pub struct EvalRequest
{
    pub _DocumentId:        u64,
    pub _DocumentRevision:  u64,
    pub _BindingGeneration: u64,
    pub _RequestGeneration: u64,
    pub _RootIndex:         u32,
    pub _Coords:            Option<MathCoordinates>,
}

//-------------------------------------------------------------------------------------------------

/// Response containing evaluated numerical results and generation tags.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalResponse
{
    pub _DocumentId:        u64,
    pub _DocumentRevision:  u64,
    pub _BindingGeneration: u64,
    pub _RequestGeneration: u64,
    pub _RootIndex:         u32,
    pub _Coords:            Option<MathCoordinates>,
    pub _Result:            Result<f64, EvalError>,
    pub _Counters:          EvalCounters,
}

//-------------------------------------------------------------------------------------------------

/// State controller managing interactive numeric evaluation, variable bindings, and stale-result rejection.
pub struct FrescoEvalController
{
    _DocumentId:          u64,
    _DocumentRevision:    u64,
    _BindingGeneration:   u64,
    _LatestRequestGen:    u64,
    _CompletedRequestGen: u64,
    _Bindings:            Stash<(String, f64)>,
}

impl FrescoEvalController
{
    pub fn New(doc: &FrescoDocument) -> Self
    {
        Self {
            _DocumentId:          doc.Id(),
            _DocumentRevision:    doc.Revision(),
            _BindingGeneration:   1,
            _LatestRequestGen:    0,
            _CompletedRequestGen: 0,
            _Bindings:            Stash::New(),
        }
    }

    pub fn SetBinding(&mut self, name: impl Into<String>, val: f64) -> u64
    {
        let nameStr = name.into();
        let mut found = false;
        let count = self._Bindings.Size();
        let mut i = 0u32;
        while i < count
        {
            if self._Bindings[i].0 == nameStr
            {
                self._Bindings[i].1 = val;
                found = true;
                break;
            }
            i += 1;
        }
        if !found
        {
            self._Bindings.Push((nameStr, val));
        }
        self._BindingGeneration = self._BindingGeneration.saturating_add(1);
        return self._BindingGeneration;
    }

    pub fn SyncDocument(&mut self, doc: &FrescoDocument)
    {
        if self._DocumentId != doc.Id()
        {
            self._DocumentId = doc.Id();
            self._BindingGeneration = self._BindingGeneration.saturating_add(1);
        }
        self._DocumentRevision = doc.Revision();
    }

    pub fn CreateRequest(&mut self, rootIndex: u32, coords: Option<MathCoordinates>) -> EvalRequest
    {
        self._LatestRequestGen = self._LatestRequestGen.saturating_add(1);
        return EvalRequest {
            _DocumentId:        self._DocumentId,
            _DocumentRevision:  self._DocumentRevision,
            _BindingGeneration: self._BindingGeneration,
            _RequestGeneration: self._LatestRequestGen,
            _RootIndex:         rootIndex,
            _Coords:            coords,
        };
    }

    pub fn EvaluateRequest(
        &self,
        doc: &FrescoDocument,
        request: &EvalRequest,
        limits: &EvalLimits<'_>,
    ) -> EvalResponse
    {
        let root = match doc.RootAt(request._RootIndex)
        {
            Some(r) => r,
            None =>
            {
                return EvalResponse {
                    _DocumentId:        request._DocumentId,
                    _DocumentRevision:  request._DocumentRevision,
                    _BindingGeneration: request._BindingGeneration,
                    _RequestGeneration: request._RequestGeneration,
                    _RootIndex:         request._RootIndex,
                    _Coords:            request._Coords,
                    _Result:            Err(EvalError::InvalidNode(u32::MAX)),
                    _Counters:          EvalCounters::default(),
                };
            }
        };

        let targetExprId = match &root._Kind
        {
            FrescoRootKind::Scalar { _ExprId } => *_ExprId,
            FrescoRootKind::Tensor { _Shape, _Elems, .. } =>
            {
                match request._Coords
                {
                    Some(coords) if coords.Rank() == 1 && coords.Indices()[0] < _Elems.Size() =>
                    {
                        _Elems[coords.Indices()[0]]
                    }
                    Some(coords)
                        if coords.Rank() == 2
                            && coords.Indices()[0] < _Shape.Dim(0)
                            && coords.Indices()[1] < _Shape.Dim(1) =>
                    {
                        let r = coords.Indices()[0];
                        let c = coords.Indices()[1];
                        let idx = r * _Shape.Dim(1) + c;
                        if idx < _Elems.Size() { _Elems[idx] } else { u32::MAX }
                    }
                    _ =>
                    {
                        if _Elems.Size() > 0 { _Elems[0] } else { u32::MAX }
                    }
                }
            }
        };

        let mut bindingsStash = Stash::<(&str, f64)>::WithCapacity(self._Bindings.Size());
        let count = self._Bindings.Size();
        let mut i = 0u32;
        while i < count
        {
            bindingsStash.Push((self._Bindings[i].0.as_str(), self._Bindings[i].1));
            i += 1;
        }

        let evalResult = doc.Repos().EvaluateNamedWithLimits(targetExprId, bindingsStash.Arr(), limits);
        match evalResult
        {
            Ok(outcome) =>
            {
                return EvalResponse {
                    _DocumentId:        request._DocumentId,
                    _DocumentRevision:  request._DocumentRevision,
                    _BindingGeneration: request._BindingGeneration,
                    _RequestGeneration: request._RequestGeneration,
                    _RootIndex:         request._RootIndex,
                    _Coords:            request._Coords,
                    _Result:            Ok(outcome._Value),
                    _Counters:          outcome._Counters,
                };
            }
            Err(err) =>
            {
                return EvalResponse {
                    _DocumentId:        request._DocumentId,
                    _DocumentRevision:  request._DocumentRevision,
                    _BindingGeneration: request._BindingGeneration,
                    _RequestGeneration: request._RequestGeneration,
                    _RootIndex:         request._RootIndex,
                    _Coords:            request._Coords,
                    _Result:            Err(err),
                    _Counters:          EvalCounters::default(),
                };
            }
        }
    }

    pub fn AcceptResponse(&mut self, response: &EvalResponse) -> Result<f64, StaleEvalError>
    {
        if response._DocumentId != self._DocumentId
        {
            return Err(StaleEvalError::DocumentMismatch);
        }
        if response._DocumentRevision != self._DocumentRevision
        {
            return Err(StaleEvalError::StaleDocumentRevision {
                current:  self._DocumentRevision,
                received: response._DocumentRevision,
            });
        }
        if response._BindingGeneration != self._BindingGeneration
        {
            return Err(StaleEvalError::StaleBindingGeneration {
                current:  self._BindingGeneration,
                received: response._BindingGeneration,
            });
        }
        if response._RequestGeneration <= self._CompletedRequestGen
        {
            return Err(StaleEvalError::StaleRequestGeneration {
                latest_completed: self._CompletedRequestGen,
                received:         response._RequestGeneration,
            });
        }
        self._CompletedRequestGen = response._RequestGeneration;
        return match &response._Result
        {
            Ok(v) => Ok(*v),
            Err(err) => Err(StaleEvalError::EvalFailed(err.clone())),
        };
    }
}

//-------------------------------------------------------------------------------------------------
