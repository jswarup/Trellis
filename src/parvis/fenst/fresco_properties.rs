// fresco_properties.rs ----------------------------------------------------------------------------
//! Read-only Fresco properties shared by workbench front ends.

use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fenst::math_layout::{MathCoordinates, MathOccurrence};
use crate::fresco::{ExprReadError, ExprView, TensorShape, VarKind};
use crate::silo::Arr;

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
            FrescoRootKind::Tensor { _Shape, _Elems } => {
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
