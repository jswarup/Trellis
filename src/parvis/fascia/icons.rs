// icons.rs ---------------------------------------------------------------------------------------
//! Crisp, theme-tinted vector icons rendered via `iced::widget::canvas`.

use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Size, mouse};

//-------------------------------------------------------------------------------------------------

/// Semantic icon identifiers used across Fascia's workbench chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId
{
    Files,
    Search,
    Sliders,
    SidebarLeft,
    PanelBottom,
    PanelRight,
    Terminal,
    OutputLog,
    FileText,
    FileCode,
    FolderClosed,
    FolderOpen,
    Drive,
    Sketch,
    MathFx,
    Waveform,
    Mesh3d,
    ExampleBolt,
    Plus,
    Save,
    Undo,
    Redo,
    Play,
    Stop,
    Refresh,
    Filter,
    Copy,
    Trash,
    Pin,
    Close,
    ChevronRight,
    ChevronDown,
    Check,
    AlertWarn,
    AlertError,
    Palette,
    Monitor,
    Branch,
    Welcome,
}

//-------------------------------------------------------------------------------------------------

struct VectorIconProgram
{
    _Icon:  IconId,
    _Color: Color,
}

impl<Message> Program<Message> for VectorIconProgram
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry>
    {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let s = w.min(h);
        let stroke = Stroke::default()
            .with_color(self._Color)
            .with_width((s * 0.10).max(1.25))
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round);

        let p = |nx: f32, ny: f32| -> Point { Point::new(nx * s, ny * s) };

        match self._Icon {
            IconId::Files => {
                let back = Path::rectangle(p(0.18, 0.14), Size::new(0.46 * s, 0.58 * s));
                let front = Path::rectangle(p(0.36, 0.28), Size::new(0.46 * s, 0.58 * s));
                frame.stroke(&back, stroke);
                frame.stroke(&front, stroke);
            }
            IconId::Search => {
                let circle = Path::circle(p(0.44, 0.44), 0.26 * s);
                let handle = Path::line(p(0.63, 0.63), p(0.84, 0.84));
                frame.stroke(&circle, stroke);
                frame.stroke(&handle, stroke);
            }
            IconId::Sliders => {
                for &(y, knob_x) in &[(0.26, 0.34), (0.50, 0.66), (0.74, 0.44)] {
                    frame.stroke(&Path::line(p(0.16, y), p(0.84, y)), stroke);
                    frame.fill(&Path::circle(p(knob_x, y), 0.08 * s), self._Color);
                }
            }
            IconId::SidebarLeft => {
                frame.stroke(&Path::rectangle(p(0.14, 0.18), Size::new(0.72 * s, 0.64 * s)), stroke);
                frame.stroke(&Path::line(p(0.38, 0.18), p(0.38, 0.82)), stroke);
            }
            IconId::PanelBottom => {
                frame.stroke(&Path::rectangle(p(0.14, 0.18), Size::new(0.72 * s, 0.64 * s)), stroke);
                frame.stroke(&Path::line(p(0.14, 0.60), p(0.86, 0.60)), stroke);
            }
            IconId::PanelRight => {
                frame.stroke(&Path::rectangle(p(0.14, 0.18), Size::new(0.72 * s, 0.64 * s)), stroke);
                frame.stroke(&Path::line(p(0.62, 0.18), p(0.62, 0.82)), stroke);
            }
            IconId::Terminal => {
                frame.stroke(&Path::rectangle(p(0.14, 0.20), Size::new(0.72 * s, 0.60 * s)), stroke);
                let prompt = Path::new(|b| {
                    b.move_to(p(0.28, 0.38));
                    b.line_to(p(0.42, 0.50));
                    b.line_to(p(0.28, 0.62));
                });
                frame.stroke(&prompt, stroke);
                frame.stroke(&Path::line(p(0.50, 0.62), p(0.70, 0.62)), stroke);
            }
            IconId::OutputLog => {
                frame.stroke(&Path::rectangle(p(0.18, 0.14), Size::new(0.64 * s, 0.72 * s)), stroke);
                frame.stroke(&Path::line(p(0.30, 0.34), p(0.70, 0.34)), stroke);
                frame.stroke(&Path::line(p(0.30, 0.50), p(0.70, 0.50)), stroke);
                frame.stroke(&Path::line(p(0.30, 0.66), p(0.56, 0.66)), stroke);
            }
            IconId::FileText => {
                let outline = Path::new(|b| {
                    b.move_to(p(0.22, 0.14));
                    b.line_to(p(0.62, 0.14));
                    b.line_to(p(0.78, 0.30));
                    b.line_to(p(0.78, 0.86));
                    b.line_to(p(0.22, 0.86));
                    b.close();
                });
                frame.stroke(&outline, stroke);
                frame.stroke(&Path::line(p(0.34, 0.46), p(0.66, 0.46)), stroke);
                frame.stroke(&Path::line(p(0.34, 0.64), p(0.60, 0.64)), stroke);
            }
            IconId::FileCode => {
                let left = Path::new(|b| {
                    b.move_to(p(0.36, 0.30));
                    b.line_to(p(0.18, 0.50));
                    b.line_to(p(0.36, 0.70));
                });
                let right = Path::new(|b| {
                    b.move_to(p(0.64, 0.30));
                    b.line_to(p(0.82, 0.50));
                    b.line_to(p(0.64, 0.70));
                });
                frame.stroke(&left, stroke);
                frame.stroke(&right, stroke);
                frame.stroke(&Path::line(p(0.54, 0.24), p(0.46, 0.76)), stroke);
            }
            IconId::FolderClosed | IconId::FolderOpen => {
                let folder = Path::new(|b| {
                    b.move_to(p(0.14, 0.26));
                    b.line_to(p(0.42, 0.26));
                    b.line_to(p(0.50, 0.36));
                    b.line_to(p(0.86, 0.36));
                    b.line_to(p(0.86, 0.78));
                    b.line_to(p(0.14, 0.78));
                    b.close();
                });
                frame.stroke(&folder, stroke);
                if matches!(self._Icon, IconId::FolderOpen) {
                    frame.stroke(&Path::line(p(0.22, 0.52), p(0.78, 0.52)), stroke);
                }
            }
            IconId::Drive => {
                frame.stroke(&Path::rectangle(p(0.14, 0.30), Size::new(0.72 * s, 0.40 * s)), stroke);
                frame.fill(&Path::circle(p(0.70, 0.50), 0.06 * s), self._Color);
            }
            IconId::Sketch => {
                let pen = Path::new(|b| {
                    b.move_to(p(0.20, 0.80));
                    b.line_to(p(0.28, 0.56));
                    b.line_to(p(0.68, 0.16));
                    b.line_to(p(0.84, 0.32));
                    b.line_to(p(0.44, 0.72));
                    b.close();
                });
                frame.stroke(&pen, stroke);
            }
            IconId::MathFx => {
                let sigma = Path::new(|b| {
                    b.move_to(p(0.74, 0.22));
                    b.line_to(p(0.26, 0.22));
                    b.line_to(p(0.54, 0.50));
                    b.line_to(p(0.26, 0.78));
                    b.line_to(p(0.74, 0.78));
                });
                frame.stroke(&sigma, stroke);
            }
            IconId::Waveform => {
                let wave = Path::new(|b| {
                    b.move_to(p(0.12, 0.66));
                    b.line_to(p(0.30, 0.66));
                    b.line_to(p(0.30, 0.32));
                    b.line_to(p(0.54, 0.32));
                    b.line_to(p(0.54, 0.66));
                    b.line_to(p(0.72, 0.66));
                    b.line_to(p(0.72, 0.32));
                    b.line_to(p(0.88, 0.32));
                });
                frame.stroke(&wave, stroke);
            }
            IconId::Mesh3d => {
                let hex = Path::new(|b| {
                    b.move_to(p(0.50, 0.14));
                    b.line_to(p(0.82, 0.32));
                    b.line_to(p(0.82, 0.68));
                    b.line_to(p(0.50, 0.86));
                    b.line_to(p(0.18, 0.68));
                    b.line_to(p(0.18, 0.32));
                    b.close();
                });
                frame.stroke(&hex, stroke);
                frame.stroke(&Path::line(p(0.18, 0.32), p(0.50, 0.50)), stroke);
                frame.stroke(&Path::line(p(0.82, 0.32), p(0.50, 0.50)), stroke);
                frame.stroke(&Path::line(p(0.50, 0.50), p(0.50, 0.86)), stroke);
            }
            IconId::ExampleBolt => {
                let bolt = Path::new(|b| {
                    b.move_to(p(0.56, 0.14));
                    b.line_to(p(0.24, 0.54));
                    b.line_to(p(0.50, 0.54));
                    b.line_to(p(0.42, 0.86));
                    b.line_to(p(0.76, 0.44));
                    b.line_to(p(0.50, 0.44));
                    b.close();
                });
                frame.stroke(&bolt, stroke);
            }
            IconId::Plus => {
                frame.stroke(&Path::line(p(0.50, 0.20), p(0.50, 0.80)), stroke);
                frame.stroke(&Path::line(p(0.20, 0.50), p(0.80, 0.50)), stroke);
            }
            IconId::Save => {
                frame.stroke(&Path::rectangle(p(0.18, 0.16), Size::new(0.64 * s, 0.68 * s)), stroke);
                frame.stroke(&Path::rectangle(p(0.32, 0.16), Size::new(0.36 * s, 0.22 * s)), stroke);
                frame.stroke(&Path::rectangle(p(0.30, 0.56), Size::new(0.40 * s, 0.28 * s)), stroke);
            }
            IconId::Undo => {
                let arc = Path::new(|b| {
                    b.move_to(p(0.24, 0.42));
                    b.line_to(p(0.64, 0.42));
                    b.line_to(p(0.78, 0.56));
                    b.line_to(p(0.64, 0.72));
                    b.line_to(p(0.36, 0.72));
                });
                let arrow = Path::new(|b| {
                    b.move_to(p(0.38, 0.26));
                    b.line_to(p(0.22, 0.42));
                    b.line_to(p(0.38, 0.58));
                });
                frame.stroke(&arc, stroke);
                frame.stroke(&arrow, stroke);
            }
            IconId::Redo => {
                let arc = Path::new(|b| {
                    b.move_to(p(0.76, 0.42));
                    b.line_to(p(0.36, 0.42));
                    b.line_to(p(0.22, 0.56));
                    b.line_to(p(0.36, 0.72));
                    b.line_to(p(0.64, 0.72));
                });
                let arrow = Path::new(|b| {
                    b.move_to(p(0.62, 0.26));
                    b.line_to(p(0.78, 0.42));
                    b.line_to(p(0.62, 0.58));
                });
                frame.stroke(&arc, stroke);
                frame.stroke(&arrow, stroke);
            }
            IconId::Play => {
                let tri = Path::new(|b| {
                    b.move_to(p(0.28, 0.20));
                    b.line_to(p(0.80, 0.50));
                    b.line_to(p(0.28, 0.80));
                    b.close();
                });
                frame.fill(&tri, self._Color);
            }
            IconId::Stop => {
                frame.fill(&Path::rectangle(p(0.24, 0.24), Size::new(0.52 * s, 0.52 * s)), self._Color);
            }
            IconId::Refresh => {
                frame.stroke(&Path::circle(p(0.50, 0.50), 0.28 * s), stroke);
                let arrow = Path::new(|b| {
                    b.move_to(p(0.66, 0.20));
                    b.line_to(p(0.80, 0.34));
                    b.line_to(p(0.62, 0.40));
                });
                frame.stroke(&arrow, stroke);
            }
            IconId::Filter => {
                let funnel = Path::new(|b| {
                    b.move_to(p(0.16, 0.22));
                    b.line_to(p(0.84, 0.22));
                    b.line_to(p(0.58, 0.52));
                    b.line_to(p(0.58, 0.80));
                    b.line_to(p(0.42, 0.72));
                    b.line_to(p(0.42, 0.52));
                    b.close();
                });
                frame.stroke(&funnel, stroke);
            }
            IconId::Copy => {
                frame.stroke(&Path::rectangle(p(0.32, 0.32), Size::new(0.50 * s, 0.52 * s)), stroke);
                let back = Path::new(|b| {
                    b.move_to(p(0.20, 0.64));
                    b.line_to(p(0.20, 0.18));
                    b.line_to(p(0.64, 0.18));
                });
                frame.stroke(&back, stroke);
            }
            IconId::Trash => {
                frame.stroke(&Path::line(p(0.20, 0.28), p(0.80, 0.28)), stroke);
                frame.stroke(&Path::line(p(0.38, 0.18), p(0.62, 0.18)), stroke);
                frame.stroke(&Path::rectangle(p(0.28, 0.28), Size::new(0.44 * s, 0.56 * s)), stroke);
            }
            IconId::Pin => {
                frame.stroke(&Path::circle(p(0.50, 0.34), 0.16 * s), stroke);
                frame.stroke(&Path::line(p(0.50, 0.50), p(0.50, 0.86)), stroke);
            }
            IconId::Close => {
                frame.stroke(&Path::line(p(0.24, 0.24), p(0.76, 0.76)), stroke);
                frame.stroke(&Path::line(p(0.76, 0.24), p(0.24, 0.76)), stroke);
            }
            IconId::ChevronRight => {
                let chev = Path::new(|b| {
                    b.move_to(p(0.36, 0.24));
                    b.line_to(p(0.66, 0.50));
                    b.line_to(p(0.36, 0.76));
                });
                frame.stroke(&chev, stroke);
            }
            IconId::ChevronDown => {
                let chev = Path::new(|b| {
                    b.move_to(p(0.24, 0.36));
                    b.line_to(p(0.50, 0.66));
                    b.line_to(p(0.76, 0.36));
                });
                frame.stroke(&chev, stroke);
            }
            IconId::Check => {
                let tick = Path::new(|b| {
                    b.move_to(p(0.20, 0.54));
                    b.line_to(p(0.42, 0.74));
                    b.line_to(p(0.80, 0.28));
                });
                frame.stroke(&tick, stroke);
            }
            IconId::AlertWarn => {
                let tri = Path::new(|b| {
                    b.move_to(p(0.50, 0.16));
                    b.line_to(p(0.86, 0.80));
                    b.line_to(p(0.14, 0.80));
                    b.close();
                });
                frame.stroke(&tri, stroke);
                frame.stroke(&Path::line(p(0.50, 0.40), p(0.50, 0.58)), stroke);
                frame.fill(&Path::circle(p(0.50, 0.70), 0.045 * s), self._Color);
            }
            IconId::AlertError => {
                frame.stroke(&Path::circle(p(0.50, 0.50), 0.34 * s), stroke);
                frame.stroke(&Path::line(p(0.36, 0.36), p(0.64, 0.64)), stroke);
                frame.stroke(&Path::line(p(0.64, 0.36), p(0.36, 0.64)), stroke);
            }
            IconId::Palette => {
                frame.stroke(&Path::circle(p(0.50, 0.50), 0.34 * s), stroke);
                frame.fill(&Path::circle(p(0.38, 0.40), 0.06 * s), self._Color);
                frame.fill(&Path::circle(p(0.58, 0.38), 0.06 * s), self._Color);
                frame.fill(&Path::circle(p(0.62, 0.58), 0.06 * s), self._Color);
            }
            IconId::Monitor => {
                frame.stroke(&Path::rectangle(p(0.14, 0.20), Size::new(0.72 * s, 0.48 * s)), stroke);
                frame.stroke(&Path::line(p(0.36, 0.82), p(0.64, 0.82)), stroke);
                frame.stroke(&Path::line(p(0.50, 0.68), p(0.50, 0.82)), stroke);
            }
            IconId::Branch => {
                frame.stroke(&Path::circle(p(0.32, 0.26), 0.09 * s), stroke);
                frame.stroke(&Path::circle(p(0.32, 0.74), 0.09 * s), stroke);
                frame.stroke(&Path::circle(p(0.70, 0.42), 0.09 * s), stroke);
                frame.stroke(&Path::line(p(0.32, 0.35), p(0.32, 0.65)), stroke);
                let curve = Path::new(|b| {
                    b.move_to(p(0.32, 0.56));
                    b.line_to(p(0.70, 0.51));
                });
                frame.stroke(&curve, stroke);
            }
            IconId::Welcome => {
                frame.stroke(&Path::circle(p(0.50, 0.50), 0.34 * s), stroke);
                frame.stroke(&Path::line(p(0.50, 0.26), p(0.50, 0.74)), stroke);
                frame.stroke(&Path::line(p(0.26, 0.50), p(0.74, 0.50)), stroke);
            }
        }

        return vec![frame.into_geometry()];
    }
}

/// Renders a crisp vector icon tinted with `color` at `size x size` logical pixels.
pub fn ViewIcon<'a, Message: 'a>(icon: IconId, size: f32, color: Color) -> Element<'a, Message>
{
    return iced::widget::canvas(VectorIconProgram {
        _Icon:  icon,
        _Color: color,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .into();
}
