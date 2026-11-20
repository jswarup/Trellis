// math_inspector.rs --------------------------------------------------------------------------------
//! Inspector sidebar displaying mathematical metadata, shapes, variable facts, and assumptions.

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fenst::math_layout::MathLayout;
use crate::fresco::{
    CondExpr, ExprEntry, FuncExpr, LogicExpr, PowExpr, ProdExpr, RealExpr, SumExpr, VarExpr,
    VarKind,
};
use iced::widget::{Column, Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

//-------------------------------------------------------------------------------------------------

/// Renders the Math Inspector sidebar panel.
pub fn ViewMathInspector<'a, Message: 'static + Clone>(doc: &'a FrescoDocument, rootIndex: u32,
                                                       selectedOcc: u32,
                                                       layout: Option<&'a MathLayout>,
                                                       palette: ThemePalette)
                                                       -> Element<'a, Message>
{
    let mut content = Column::new().spacing(12).padding(14);

    // 1. Header
    let header =
        text("Math Inspector").size(15)
                              .style(move |_| text::Style { color: Some(palette.text_primary), });
    content = content.push(header);

    // 2. Active Root Card
    let rootCard = ViewRootSection(doc, rootIndex, palette);
    content = content.push(rootCard);

    // 3. Selected Term Card
    let termCard = ViewTermSection(doc, selectedOcc, layout, palette);
    content = content.push(termCard);

    container(scrollable(content)).width(Length::Fixed(260.0))
                                  .height(Length::Fill)
                                  .style(move |_| FasciaStyle::sidebar_container(palette))
                                  .into()
}

fn ViewRootSection<'a, Message: 'static + Clone>(doc: &'a FrescoDocument, rootIndex: u32,
                                                 palette: ThemePalette)
                                                 -> Element<'a, Message>
{
    let root = match doc.RootAt(rootIndex) {
        Some(r) => r,
        None => return Column::new().into(),
    };

    let kindStr = match root.Kind() {
        FrescoRootKind::Scalar { .. } => "Scalar Expression".to_string(),
        FrescoRootKind::Tensor { _Shape, .. } => {
            let rank = _Shape.Rank();
            if rank == 1 {
                format!("Vector [{}]", _Shape.Dim(0))
            } else if rank == 2 {
                format!("Matrix [{} \u{00D7} {}]", _Shape.Dim(0), _Shape.Dim(1))
            } else {
                format!("Tensor (rank {})", rank)
            }
        }
    };

    let docInfo = format!("Doc #{}  \u{2022}  Rev {}", doc.Id(), doc.Revision());

    let cardContent = column![
        text("ACTIVE ROOT")
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
        text(root.Name())
            .size(14)
            .style(move |_| text::Style { color: Some(palette.text_primary) }),
        Space::new().height(Length::Fixed(4.0)),
        row![
            text("Type: ")
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
            text(kindStr)
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_primary) }),
        ],
        row![
            text("Info: ")
                .size(11)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
            text(docInfo)
                .size(11)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
        ],
    ].spacing(4);

    container(cardContent).padding(10)
                          .width(Length::Fill)
                          .style(move |_| FasciaStyle::content_container(palette))
                          .into()
}

fn ViewTermSection<'a, Message: 'static + Clone>(doc: &'a FrescoDocument, selectedOcc: u32,
                                                 layout: Option<&'a MathLayout>,
                                                 palette: ThemePalette)
                                                 -> Element<'a, Message>
{
    let Some(occurrence) = layout.and_then(|l| l.OccurrenceAt(selectedOcc)) else {
        return ViewNoSelectionPlaceholder(palette);
    };
    let occId = selectedOcc;

    let repos = doc.Repos();
    let exprId = occurrence._ExprId;

    let mut details = Column::new().spacing(6);

    details = details.push(
        text("SELECTED TERM")
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    // Occurrence & Expression ID
    details = details.push(row![
            text("Expr ID: ")
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
            text(format!("#{}", exprId))
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_primary) }),
            Space::new().width(Length::Fixed(12.0)),
            text("Occ ID: ")
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
            text(format!("#{}", occId))
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_primary) }),
        ].align_y(Alignment::Center));

    // Coords if tensor cell
    if let Some(coords) = &occurrence._Coords {
        let coord_str = coords.to_string();
        details = details.push(row![
            text("Coordinates: ").size(12).style(move |_| {
                text::Style { color: Some(palette.text_secondary), }
            }),
            text(coord_str).size(12).style(move |_| {
                text::Style { color: Some(palette.accent), }
            }),
        ].align_y(Alignment::Center));
    }

    // Node specifics
    if repos.IsValidExprId(exprId) {
        match repos.ExprEntryAt(exprId) {
            ExprEntry::Empty => {
                details = details.push(
                    text("Empty node")
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                );
            }
            ExprEntry::Expr(expr) => {
                let any = expr.Any();
                if let Some(real) = any.downcast_ref::<RealExpr>() {
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Numeric Constant").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                    details = details.push(row![
                        text("Value: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(format!("{}", real.Value())).size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                } else if let Some(var) = any.downcast_ref::<VarExpr>() {
                    let vIdx = var.VarIndex();
                    let vName = repos.VarNameAt(vIdx);
                    let attr = repos.VarAttrAt(vIdx);
                    let nonZero = attr.HasBits(VarKind::NonZero);

                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Variable").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                    details = details.push(row![
                        text("Symbol: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(vName).size(12)
                                   .style(move |_| text::Style { color: Some(palette.accent), }),
                    ]);
                    details = details.push(row![
                        text("Var Index: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(format!("{}", vIdx)).size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                    let nonZeroStr = if nonZero { "Yes" } else { "None" };
                    details = details.push(row![
                        text("NonZero: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(nonZeroStr).size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                } else if let Some(sum) = any.downcast_ref::<SumExpr>() {
                    let poly = sum.Poly();
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Sum (n-ary)").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                    details = details.push(row![
                        text("Terms: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(format!(
                            "{} add, {} sub",
                            poly.CommonSize(),
                            poly.SizeChild() - poly.CommonSize()
                        )).size(12)
                          .style(move |_| text::Style { color: Some(palette.text_primary), }),
                    ]);
                } else if let Some(prod) = any.downcast_ref::<ProdExpr>() {
                    let poly = prod.Poly();
                    let co = poly.CommonSize();
                    let denom = poly.SizeChild() - co;
                    let kindDesc = if denom > 0 { "Fraction" } else { "Product" };
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text(kindDesc).size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                    details = details.push(row![
                        text("Factors: ")
                            .size(12)
                            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                        text(format!("{} num, {} den", co, denom))
                            .size(12)
                            .style(move |_| text::Style { color: Some(palette.text_primary) }),
                    ]);
                } else if any.downcast_ref::<PowExpr>().is_some() {
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Power / Exponent").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                } else if let Some(func) = any.downcast_ref::<FuncExpr>() {
                    details = details.push(row![
                        text("Kind: ")
                            .size(12)
                            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                        text(format!("Function: {}()", func.Name()))
                            .size(12)
                            .style(move |_| text::Style { color: Some(palette.text_primary) }),
                    ]);
                } else if any.downcast_ref::<LogicExpr>().is_some() {
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Logic Comparison").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                } else if any.downcast_ref::<CondExpr>().is_some() {
                    details = details.push(row![
                        text("Kind: ").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_secondary), }
                        }),
                        text("Conditional (Piecewise)").size(12).style(move |_| {
                            text::Style { color: Some(palette.text_primary), }
                        }),
                    ]);
                }
            }
        }
    }

    container(details).padding(10)
                      .width(Length::Fill)
                      .style(move |_| FasciaStyle::content_container(palette))
                      .into()
}

fn ViewNoSelectionPlaceholder<'a, Message: 'static + Clone>(palette: ThemePalette)
                                                            -> Element<'a, Message>
{
    let content = column![
        text("SELECTED TERM")
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
        Space::new().height(Length::Fixed(4.0)),
        text("Click on any term or matrix cell in the math view to inspect details.")
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    ].spacing(4);

    container(content).padding(10)
                      .width(Length::Fill)
                      .style(move |_| FasciaStyle::content_container(palette))
                      .into()
}

//-------------------------------------------------------------------------------------------------
