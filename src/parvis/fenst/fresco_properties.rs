// fresco_properties.rs ----------------------------------------------------------------------------
//! Read-only Fresco properties shared by workbench front ends.

use crate::fenst::fresco_document::FrescoDocument;
use crate::fenst::math_layout::{MathCoordinates, MathOccurrence};
use crate::fresco::{ExprReadError, ExprView, TensorShape, VarKind};

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
    /// The occurrence, if supplied, comes from the layout for this document revision.
    pub fn Read(doc: &'a FrescoDocument, rootIndex: u32, occurrence: Option<&MathOccurrence>)
                -> Option<Self>
    {
        let root = doc.RootAt(rootIndex)?;
        let repos = doc.Repos();
        let term = occurrence.filter(|occ| occ._RootIndex == rootIndex).map(|occ| {
            let value = repos.Inspect(occ._ExprId).map(|view| match view {
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
            return FrescoTermProperties { ExprId:       occ._ExprId,
                                          OccurrenceId: occ._Id,
                                          Coordinates:  occ._Coords,
                                          Value:        value, };
        });
        return Some(Self { DocumentId: doc.Id(),
                           Revision:   doc.Revision(),
                           Name:       root.Name(),
                           Shape:      root.Shape(),
                           Term:       term, });
    }
}

//-------------------------------------------------------------------------------------------------
