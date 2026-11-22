// math_inspector.rs --------------------------------------------------------------------------------
//! Native view of the read-only Fresco property projection.

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::fresco_document::FrescoDocument;
use crate::fenst::fresco_properties::{FrescoProperties, FrescoTermValue};
use crate::fenst::math_layout::MathLayout;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

//-------------------------------------------------------------------------------------------------

pub fn ViewMathInspector<'a, Message: 'static>(
    doc: &'a FrescoDocument, rootIndex: u32, selectedOcc: u32, layout: Option<&MathLayout>,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    let occurrence = layout.and_then(|layout| layout.OccurrenceAt(selectedOcc));
    return ViewMathProperties(FrescoProperties::Read(doc, rootIndex, occurrence), None, palette);
}

pub(super) fn ViewMathProperties<'a, Message: 'static>(
    properties: Option<FrescoProperties<'a>>, preview: Option<Element<'a, Message>>,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    let mut content = column![Label("Properties", 15, palette, false)].spacing(12).padding(14);
    if let Some(properties) = properties {
        let kind = match properties.Shape {
            None => "Scalar expression".to_string(),
            Some(shape) => match shape.Rank() {
                1 => format!("Vector [{}]", shape.Dim(0)),
                2 => format!("Matrix [{} × {}]", shape.Dim(0), shape.Dim(1)),
                rank => format!("Tensor (rank {rank})"),
            },
        };
        let mut root = column![Label("Expression", 12, palette, true),
                               Label(properties.Name, 14, palette, false),
                               PropertyRow("Type", kind, palette)].spacing(5);
        if let Some(preview) = preview {
            root = root.push(preview);
        }
        root = root.push(Label(format!("Doc #{} · Rev {}", properties.DocumentId,
                                      properties.Revision), 11, palette, true));
        content = content.push(container(root).width(Length::Fill).padding(10)
            .style(move |_| FasciaStyle::content_container(palette)));

        let mut term = column![Label("Selected term", 12, palette, true)].spacing(5);
        if let Some(selected) = properties.Term {
            term = term.push(PropertyRow("Expr ID", format!("#{}", selected.ExprId), palette))
                       .push(PropertyRow("Occurrence", format!("#{}", selected.OccurrenceId), palette));
            if let Some(coords) = selected.Coordinates {
                term = term.push(PropertyRow("Coordinates", coords.to_string(), palette));
            }
            let value = match selected.Value {
                Ok(FrescoTermValue::Constant(value)) => {
                    column![PropertyRow("Kind", "Numeric constant", palette),
                            PropertyRow("Value", value.to_string(), palette)]
                }
                Ok(FrescoTermValue::Variable { name, index, nonZero }) => {
                    column![PropertyRow("Kind", "Variable", palette),
                            PropertyRow("Symbol", name, palette),
                            PropertyRow("Variable index", index.to_string(), palette),
                            PropertyRow("Nonzero", if nonZero { "Yes" } else { "Unspecified" }, palette)]
                }
                Ok(FrescoTermValue::Sum { add, subtract }) => {
                    column![PropertyRow("Kind", "Sum", palette),
                            PropertyRow("Terms", format!("{add} add, {subtract} subtract"), palette)]
                }
                Ok(FrescoTermValue::Product { numerator, denominator }) => {
                    let kind = if denominator == 0 { "Product" } else { "Fraction" };
                    column![PropertyRow("Kind", kind, palette),
                            PropertyRow("Factors", format!("{numerator} num, {denominator} den"), palette)]
                }
                Ok(FrescoTermValue::Power) => column![PropertyRow("Kind", "Power", palette)],
                Ok(FrescoTermValue::Function(name)) => {
                    column![PropertyRow("Kind", "Function", palette),
                            PropertyRow("Function", name, palette)]
                }
                Ok(FrescoTermValue::Logic) => column![PropertyRow("Kind", "Logic", palette)],
                Ok(FrescoTermValue::Conditional) => {
                    column![PropertyRow("Kind", "Conditional", palette)]
                }
                Err(error) => column![Label(error.to_string(), 12, palette, true)],
            };
            term = term.push(value.spacing(5));
        } else {
            term = term.push(Label("Select a term or matrix cell to inspect it.", 12, palette, true));
        }
        content = content.push(container(term).width(Length::Fill).padding(10)
            .style(move |_| FasciaStyle::content_container(palette)));
    }

    return container(scrollable(content)).width(Length::Fixed(260.0))
        .height(Length::Fill).style(move |_| FasciaStyle::sidebar_container(palette)).into();
}

fn Label<'a>(value: impl text::IntoFragment<'a>, size: u32,
             palette: ThemePalette, secondary: bool) -> iced::widget::Text<'a>
{
    let color = if secondary { palette.text_secondary } else { palette.text_primary };
    return text(value).size(size as f32).style(move |_| text::Style { color: Some(color) });
}

fn PropertyRow<'a, Message: 'static>(
    label: &'static str, value: impl text::IntoFragment<'a>, palette: ThemePalette,
) -> Element<'a, Message>
{
    return row![Label(label, 12, palette, true).width(Length::FillPortion(2)),
                Label(value, 12, palette, false).width(Length::FillPortion(3))].spacing(8).into();
}

//-------------------------------------------------------------------------------------------------
