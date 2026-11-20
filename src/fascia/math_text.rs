// math_text.rs ------------------------------------------------------------------------------------
//! Shared paragraph configuration for measuring and painting mathematical text.

use crate::fenst::cask::Rect;
use crate::fenst::math_layout::{IMathFontMetrics, MathTextMetrics};
use cosmic_text::{Command, SwashCache};
use iced::advanced::{
    graphics::text::Paragraph,
    text::{self, Paragraph as _},
};
use iced::font::{Style, Weight};
use iced::widget::canvas::Text;
use iced::{Font, Size};
use std::sync::{Mutex, OnceLock};

//-------------------------------------------------------------------------------------------------

static SWASH: OnceLock<Mutex<SwashCache>> = OnceLock::new();

/// Uses the same font database, shaping, alignment and line height as canvas Text.
#[derive(Clone, Copy, Debug, Default)]
pub struct CosmicMathFontMetrics;

impl CosmicMathFontMetrics
{
    pub fn New() -> Self { return Self; }

    pub fn Font(isBold: bool, isItalic: bool) -> Font
    {
        return Font { weight: if isBold { Weight::Bold } else { Weight::Normal },
                      style: if isItalic {
                          Style::Italic
                      } else {
                          Style::Normal
                      },
                      ..Font::DEFAULT };
    }

    pub fn CanvasText(content: String, fontSize: f32, isBold: bool, isItalic: bool) -> Text
    {
        return Text { content,
                      font: Self::Font(isBold, isItalic),
                      size: fontSize.into(),
                      line_height: text::LineHeight::Relative(1.25),
                      shaping: text::Shaping::Advanced,
                      align_x: text::Alignment::Left,
                      align_y: iced::alignment::Vertical::Top,
                      ..Text::default() };
    }

    /// Construct exactly the paragraph configuration used by canvas Text.
    pub fn Paragraph(text: &Text) -> Paragraph
    {
        return Paragraph::with_text(text::Text { content:     text.content.as_str(),
                                                 bounds:      Size::new(text.max_width,
                                                                        f32::INFINITY),
                                                 size:        text.size,
                                                 line_height: text.line_height,
                                                 font:        text.font,
                                                 align_x:     text.align_x,
                                                 align_y:     text.align_y,
                                                 shaping:     text.shaping,
                                                 wrapping:    text::Wrapping::default(), });
    }

    fn Metrics(paragraph: &Paragraph, fontSize: f32) -> MathTextMetrics
    {
        let baseline = paragraph.buffer()
                                .layout_runs()
                                .next()
                                .map(|run| run.line_y)
                                .unwrap_or(fontSize * 0.8);
        // Use the same font IDs as the shaped paragraph. A separate FontSystem
        // can map these IDs to entirely different fonts.
        let mut fonts = iced::advanced::graphics::text::font_system().write()
                                                                     .unwrap_or_else(|poisoned| {
                                                                         poisoned.into_inner()
                                                                     });
        let mut swash = SWASH.get_or_init(|| Mutex::new(SwashCache::new()))
                             .lock()
                             .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut minX = f32::INFINITY;
        let mut minY = f32::INFINITY;
        let mut maxX = f32::NEG_INFINITY;
        let mut maxY = f32::NEG_INFINITY;
        let mut include = |x: f32, y: f32| {
            minX = minX.min(x);
            minY = minY.min(y);
            maxX = maxX.max(x);
            maxY = maxY.max(y);
        };
        paragraph.buffer().layout_runs().for_each(|run| {
            run.glyphs.iter().for_each(|glyph| {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                // Raster bounds cover native/color glyphs without visiting pixels.
                if let Some(image) = swash.get_image(fonts.raw(), physical.cache_key)
                    && image.placement.width > 0 && image.placement.height > 0 {
                    let x = physical.x as f32 + image.placement.left as f32;
                    let y = run.line_y + physical.y as f32 - image.placement.top as f32 - baseline;
                    include(x, y);
                    include(x + image.placement.width as f32, y + image.placement.height as f32);
                }
                // The control-point hull conservatively includes the vector path
                // used by canvas Text, including negative side bearings.
                if let Some(commands) = swash.get_outline_commands(fonts.raw(), physical.cache_key) {
                    let x = glyph.x + glyph.x_offset;
                    let y = run.line_y + glyph.y_offset - baseline;
                    commands.iter().for_each(|command| match command {
                        Command::MoveTo(p) | Command::LineTo(p) => include(x + p.x, y - p.y),
                        Command::QuadTo(a, p) => {
                            include(x + a.x, y - a.y); include(x + p.x, y - p.y);
                        }
                        Command::CurveTo(a, b, p) => {
                            include(x + a.x, y - a.y); include(x + b.x, y - b.y);
                            include(x + p.x, y - p.y);
                        }
                        Command::Close => {},
                    });
                }
            });
        });
        let inkBounds = if minX.is_finite() {
            Rect::New(minX, minY, maxX - minX, maxY - minY)
        } else {
            Rect::Zero()
        };
        return MathTextMetrics { width: paragraph.min_width(),
                                 inkBounds,
                                 ascent: baseline,
                                 descent: (paragraph.min_height() - baseline).max(0.0),
                                 paragraphBaselineOffset: baseline };
    }
}

impl IMathFontMetrics for CosmicMathFontMetrics
{
    fn MeasureAtom(&self, text: &str, fontSize: f32, isBold: bool, isItalic: bool)
                   -> MathTextMetrics
    {
        let atom = Self::CanvasText(text.to_string(), fontSize, isBold, isItalic);
        return Self::Metrics(&Self::Paragraph(&atom), fontSize);
    }
}

//-------------------------------------------------------------------------------------------------
