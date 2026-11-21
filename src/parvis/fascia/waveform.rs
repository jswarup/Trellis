//-- waveform.rs ----------------------------------------------------------------------------------------------------
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::waveform::{CursorMarker, WaveformModel, WaveformRadix, WaveformSignal};
use iced::widget::{Space, button, canvas, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, mouse};
use std::sync::Arc;

const K_VISIBLE_SIGNALS: u32 = 64;
const K_WAVEFORM_ROW_HEIGHT: f32 = 28.0;

//---------------------------------------------------------------------------------------------------------------------------------

struct EmptyWaveform;
impl WaveformModel for EmptyWaveform
{
    fn TimeMin(&self) -> u64 { 0 }
    fn TimeMax(&self) -> u64 { 0 }
    fn SignalCount(&self) -> u32 { 0 }
    fn Signal(&self, _index: u32) -> Option<&dyn WaveformSignal> { None }
    fn FindSignal(&self, _name: &str) -> u32 { u32::MAX }
    fn NextTransition(&self, _time: u64) -> Option<u64> { None }
    fn PreviousTransition(&self, _time: u64) -> Option<u64> { None }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveformAction
{
    Fit,
    ZoomIn,
    ZoomOut,
    PreviousChange,
    NextChange,
    SelectSignal(u32),
    SetCursorA(u64),
    SetCursorB(u64),
    ToggleActiveMarker,
    CycleRadix,
    SetRadix(WaveformRadix),
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct WaveformState
{
    _Model:          Arc<dyn WaveformModel>,
    _ViewStart:      u64,
    _ViewEnd:        u64,
    _CursorA:        u64,
    _CursorB:        u64,
    _ActiveMarker:   CursorMarker,
    _Radix:          WaveformRadix,
    _SelectedSignal: u32,
    _Error:          Option<String>,
}

impl WaveformState
{
    pub fn New(model: Arc<dyn WaveformModel>) -> Self
    {
        let viewStart = model.TimeMin();
        let viewEnd = model.TimeMax().max(viewStart.saturating_add(1));
        Self {
            _Model:          model,
            _ViewStart:      viewStart,
            _ViewEnd:        viewEnd,
            _CursorA:        viewStart,
            _CursorB:        viewEnd,
            _ActiveMarker:   CursorMarker::MarkerA,
            _Radix:          WaveformRadix::Hex,
            _SelectedSignal: u32::MAX,
            _Error:          None,
        }
    }

    pub fn FromError(error: String) -> Self
    {
        Self {
            _Model:          Arc::new(EmptyWaveform),
            _ViewStart:      0,
            _ViewEnd:        1,
            _CursorA:        0,
            _CursorB:        1,
            _ActiveMarker:   CursorMarker::MarkerA,
            _Radix:          WaveformRadix::Hex,
            _SelectedSignal: u32::MAX,
            _Error:          Some(error),
        }
    }

    pub fn Model(&self) -> &dyn WaveformModel { self._Model.as_ref() }

    pub fn CursorTime(&self) -> u64
    {
        match self._ActiveMarker {
            CursorMarker::MarkerA => self._CursorA,
            CursorMarker::MarkerB => self._CursorB,
        }
    }

    pub fn CursorA(&self) -> u64 { self._CursorA }
    pub fn CursorB(&self) -> u64 { self._CursorB }
    pub fn DeltaTime(&self) -> u64 { self._CursorB.abs_diff(self._CursorA) }
    pub fn ActiveMarker(&self) -> CursorMarker { self._ActiveMarker }
    pub fn Radix(&self) -> WaveformRadix { self._Radix }
    pub fn SelectedSignal(&self) -> u32 { self._SelectedSignal }
    pub fn ViewStart(&self) -> u64 { self._ViewStart }
    pub fn ViewEnd(&self) -> u64 { self._ViewEnd }

    pub fn Frequency(&self) -> Option<f64>
    {
        let delta = self.DeltaTime();
        if delta == 0 {
            return None;
        }
        let timescale = self._Model.Timescale().trim();
        let stepSec = parse_timescale_seconds(timescale);
        let periodSec = (delta as f64) * stepSec;
        if periodSec > 0.0 {
            Some(1.0 / periodSec)
        } else {
            None
        }
    }

    pub fn Update(&mut self, action: WaveformAction)
    {
        match action {
            WaveformAction::Fit => {
                self._ViewStart = self._Model.TimeMin();
                self._ViewEnd = self._Model.TimeMax().max(self._ViewStart.saturating_add(1));
                self._CursorA = self._CursorA.clamp(self._ViewStart, self._ViewEnd);
                self._CursorB = self._CursorB.clamp(self._ViewStart, self._ViewEnd);
            }
            WaveformAction::ZoomIn => self.Zoom(1, 2),
            WaveformAction::ZoomOut => self.Zoom(2, 1),
            WaveformAction::PreviousChange => self.MoveActiveCursor(false),
            WaveformAction::NextChange => self.MoveActiveCursor(true),
            WaveformAction::SelectSignal(index) => {
                if index < self._Model.SignalCount() {
                    self._SelectedSignal = index;
                }
            }
            WaveformAction::SetCursorA(time) => {
                self._CursorA = time.clamp(self._Model.TimeMin(), self._Model.TimeMax());
            }
            WaveformAction::SetCursorB(time) => {
                self._CursorB = time.clamp(self._Model.TimeMin(), self._Model.TimeMax());
            }
            WaveformAction::ToggleActiveMarker => {
                self._ActiveMarker = self._ActiveMarker.Other();
            }
            WaveformAction::CycleRadix => {
                self._Radix = self._Radix.Next();
            }
            WaveformAction::SetRadix(radix) => {
                self._Radix = radix;
            }
        }
    }

    fn Zoom(&mut self, numerator: u64, denominator: u64)
    {
        let span = self._ViewEnd.saturating_sub(self._ViewStart).max(1);
        let newSpan = (span.saturating_mul(numerator) / denominator).max(1);
        let curTime = self.CursorTime();
        let relative = curTime.saturating_sub(self._ViewStart);
        self._ViewStart = curTime.saturating_sub(relative.saturating_mul(newSpan) / span);
        self._ViewEnd = self._ViewStart.saturating_add(newSpan);
    }

    fn MoveActiveCursor(&mut self, forward: bool)
    {
        let index = self._SelectedSignal;
        let Some(signal) = self._Model.Signal(index) else {
            return;
        };
        let curTime = self.CursorTime();
        let target = if forward {
            signal.NextChange(curTime)
        } else {
            signal.PreviousChange(curTime)
        };
        let nextTime = target.unwrap_or(curTime);
        match self._ActiveMarker {
            CursorMarker::MarkerA => self._CursorA = nextTime,
            CursorMarker::MarkerB => self._CursorB = nextTime,
        }
    }
}

fn parse_timescale_seconds(ts: &str) -> f64
{
    let lower = ts.to_lowercase();
    let digits: String = lower.chars().take_while(|c| c.is_ascii_digit()).collect();
    let mult = digits.parse::<f64>().unwrap_or(1.0);
    let unit: String = lower.chars().skip_while(|c| c.is_ascii_digit() || c.is_whitespace()).collect();
    let unitSec = match unit.as_str() {
        "fs" => 1e-15,
        "ps" => 1e-12,
        "ns" => 1e-9,
        "us" => 1e-6,
        "ms" => 1e-3,
        "s" => 1.0,
        _ => 1e-9,
    };
    mult * unitSec
}

//---------------------------------------------------------------------------------------------------------------------------------

struct WaveformLane<'a>
{
    _Signal:    &'a dyn WaveformSignal,
    _ViewStart: u64,
    _ViewEnd:   u64,
    _CursorA:   u64,
    _CursorB:   u64,
    _Radix:     WaveformRadix,
    _Palette:   ThemePalette,
}

impl WaveformLane<'_>
{
    fn time_to_x(&self, time: u64, width: f32) -> f32
    {
        let span = self._ViewEnd.saturating_sub(self._ViewStart).max(1) as f32;
        time.saturating_sub(self._ViewStart) as f32 * width / span
    }

    fn level(value: &str, height: f32) -> f32
    {
        match value {
            "1" => height * 0.25,
            "0" => height * 0.75,
            _ => height * 0.5,
        }
    }

    fn draw_single_bit(&self, frame: &mut canvas::Frame, bounds: Rectangle, stroke: canvas::Stroke)
    {
        let changes = self._Signal.ChangesBetween(self._ViewStart, self._ViewEnd);
        let mut previous_time = self._ViewStart;
        let mut previous_value: &str = self._Signal.ValueAt(self._ViewStart);
        changes.USeg().Traverse(|index| {
            let (time, value) = &changes[index];
            if *time <= self._ViewStart || *time > self._ViewEnd {
                return;
            }
            let x = self.time_to_x(*time, bounds.width);
            frame.stroke(
                &canvas::Path::line(
                    Point::new(self.time_to_x(previous_time, bounds.width), Self::level(previous_value, bounds.height)),
                    Point::new(x, Self::level(previous_value, bounds.height)),
                ),
                stroke,
            );
            frame.stroke(
                &canvas::Path::line(
                    Point::new(x, Self::level(previous_value, bounds.height)),
                    Point::new(x, Self::level(value, bounds.height)),
                ),
                stroke,
            );
            previous_time = *time;
            previous_value = value.as_str();
        });
        frame.stroke(
            &canvas::Path::line(
                Point::new(self.time_to_x(previous_time, bounds.width), Self::level(previous_value, bounds.height)),
                Point::new(bounds.width, Self::level(previous_value, bounds.height)),
            ),
            stroke,
        );
    }

    fn draw_multi_bit(&self, frame: &mut canvas::Frame, bounds: Rectangle, stroke: canvas::Stroke)
    {
        let topY = bounds.height * 0.22;
        let botY = bounds.height * 0.78;
        let midY = bounds.height * 0.50;
        let dW = 4.0f32;

        let changes = self._Signal.ChangesBetween(self._ViewStart, self._ViewEnd);
        let mut prevX = 0.0f32;
        let mut prevVal = self._Signal.ValueAt(self._ViewStart);

        changes.USeg().Traverse(|index| {
            let (time, value) = &changes[index];
            if *time <= self._ViewStart || *time > self._ViewEnd {
                return;
            }
            let transX = self.time_to_x(*time, bounds.width);

            let leftRail = (prevX + dW).min(transX - dW);
            let rightRail = (transX - dW).max(leftRail);
            if rightRail > leftRail {
                frame.stroke(&canvas::Path::line(Point::new(leftRail, topY), Point::new(rightRail, topY)), stroke);
                frame.stroke(&canvas::Path::line(Point::new(leftRail, botY), Point::new(rightRail, botY)), stroke);
            }

            let dLeft = (transX - dW).max(0.0);
            let dRight = (transX + dW).min(bounds.width);
            frame.stroke(&canvas::Path::line(Point::new(dLeft, topY), Point::new(transX, midY)), stroke);
            frame.stroke(&canvas::Path::line(Point::new(transX, midY), Point::new(dRight, topY)), stroke);
            frame.stroke(&canvas::Path::line(Point::new(dLeft, botY), Point::new(transX, midY)), stroke);
            frame.stroke(&canvas::Path::line(Point::new(transX, midY), Point::new(dRight, botY)), stroke);

            let cellW = rightRail - leftRail;
            if cellW >= 28.0 {
                let formatted = self._Radix.Format(prevVal, self._Signal.Bits());
                let textX = (leftRail + rightRail) * 0.5;
                frame.fill_text(canvas::Text {
                    content: formatted,
                    position: Point::new(textX, midY - 5.0),
                    color: Color::from_rgb8(220, 225, 235),
                    size: 10.0.into(),
                    ..Default::default()
                });
            }

            prevX = transX;
            prevVal = value.as_str();
        });

        let tailLeft = (prevX + dW).min(bounds.width);
        if bounds.width > tailLeft {
            frame.stroke(&canvas::Path::line(Point::new(tailLeft, topY), Point::new(bounds.width, topY)), stroke);
            frame.stroke(&canvas::Path::line(Point::new(tailLeft, botY), Point::new(bounds.width, botY)), stroke);

            let cellW = bounds.width - tailLeft;
            if cellW >= 28.0 {
                let formatted = self._Radix.Format(prevVal, self._Signal.Bits());
                let textX = (tailLeft + bounds.width) * 0.5;
                frame.fill_text(canvas::Text {
                    content: formatted,
                    position: Point::new(textX, midY - 5.0),
                    color: Color::from_rgb8(220, 225, 235),
                    size: 10.0.into(),
                    ..Default::default()
                });
            }
        }
    }

    fn draw_cursors(&self, frame: &mut canvas::Frame, bounds: Rectangle)
    {
        if self._CursorA >= self._ViewStart && self._CursorA <= self._ViewEnd {
            let xA = self.time_to_x(self._CursorA, bounds.width);
            let strokeA = canvas::Stroke::default().with_color(Color::from_rgb8(56, 189, 248)).with_width(1.5);
            frame.stroke(&canvas::Path::line(Point::new(xA, 0.0), Point::new(xA, bounds.height)), strokeA);
        }

        if self._CursorB >= self._ViewStart && self._CursorB <= self._ViewEnd {
            let xB = self.time_to_x(self._CursorB, bounds.width);
            let strokeB = canvas::Stroke::default().with_color(Color::from_rgb8(245, 158, 11)).with_width(1.5);
            frame.stroke(&canvas::Path::line(Point::new(xB, 0.0), Point::new(xB, bounds.height)), strokeB);
        }
    }
}

impl<Message> canvas::Program<Message> for WaveformLane<'_>
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry>
    {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self._Palette.content_bg);
        let wave_color = if self._Signal.IsSingleBit() {
            Color::from_rgb8(111, 210, 150)
        } else {
            Color::from_rgb8(96, 205, 255)
        };
        let stroke = canvas::Stroke::default().with_color(wave_color).with_width(1.5);

        if self._Signal.IsSingleBit() {
            self.draw_single_bit(&mut frame, bounds, stroke);
        } else {
            self.draw_multi_bit(&mut frame, bounds, stroke);
        }

        self.draw_cursors(&mut frame, bounds);
        vec![frame.into_geometry()]
    }
}

pub fn view_waveform<'a, Message: 'static + Clone>(
    state: &'a WaveformState,
    palette: ThemePalette,
    map_action: impl Fn(WaveformAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    if let Some(error) = &state._Error {
        return container(
            column![
                text("VCD Parse Error").size(18),
                Space::new().height(Length::Fixed(8.0)),
                text(error).size(13),
            ]
            .padding(24)
            .spacing(6),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::content_container(palette))
        .into();
    }
    let model = state.Model();
    let activeMarkerStr = match state.ActiveMarker() {
        CursorMarker::MarkerA => "Marker [A]",
        CursorMarker::MarkerB => "Marker [B]",
    };
    let header = row![
        text("WAVEFORM").size(14),
        Space::new().width(Length::Fixed(12.0)),
        text(format!("{} signals", model.SignalCount())).size(12),
        Space::new().width(Length::Fixed(10.0)),
        text(format!("timescale: {}", model.Timescale())).size(12),
        Space::new().width(Length::Fill),
        button(text(activeMarkerStr).size(12)).on_press(map_action(WaveformAction::ToggleActiveMarker)),
        button(text(format!("Radix: {}", state.Radix().AsStr())).size(12)).on_press(map_action(WaveformAction::CycleRadix)),
        button(text("Previous").size(12)).on_press(map_action(WaveformAction::PreviousChange)),
        button(text("Next").size(12)).on_press(map_action(WaveformAction::NextChange)),
        button(text("Zoom -").size(12)).on_press(map_action(WaveformAction::ZoomOut)),
        button(text("Zoom +").size(12)).on_press(map_action(WaveformAction::ZoomIn)),
        button(text("Fit").size(12)).on_press(map_action(WaveformAction::Fit)),
    ]
    .spacing(5)
    .padding([8, 12])
    .align_y(Alignment::Center);

    let freqStr = if let Some(freq) = state.Frequency() {
        if freq >= 1e9 {
            format!(" ({:.2} GHz)", freq / 1e9)
        } else if freq >= 1e6 {
            format!(" ({:.2} MHz)", freq / 1e6)
        } else if freq >= 1e3 {
            format!(" ({:.2} kHz)", freq / 1e3)
        } else {
            format!(" ({:.2} Hz)", freq)
        }
    } else {
        String::new()
    };
    let ruler = row![
        text("SIGNAL").size(11),
        Space::new().width(Length::Fixed(190.0)),
        text(format!(
            "A: #{}   B: #{}   ΔT: #{}{}   View: [#{}-#{}]",
            state.CursorA(),
            state.CursorB(),
            state.DeltaTime(),
            freqStr,
            state.ViewStart(),
            state.ViewEnd()
        ))
        .size(11),
    ]
    .padding([6, 12]);

    let mut rows = column![].spacing(1);
    let visible = model.SignalCount().min(K_VISIBLE_SIGNALS);
    let mut index = 0;
    while index < visible {
        let signal = model.Signal(index).expect("Visible signal index is valid");
        let selected = state.SelectedSignal() == index;
        let name = format!("{} [{}]", signal.FullName(), signal.Bits());
        let signalButton = button(text(name).size(12))
            .width(Length::Fixed(250.0))
            .padding([5, 8])
            .style(move |_, status| FasciaStyle::tree_row_button(palette, selected, status))
            .on_press(map_action(WaveformAction::SelectSignal(index)));

        let rawVal = signal.ValueAt(state.CursorTime());
        let dispVal = if signal.IsSingleBit() {
            rawVal.to_string()
        } else {
            state.Radix().Format(rawVal, signal.Bits())
        };

        let waveform: Element<'a, Message> = canvas(WaveformLane {
            _Signal:    signal,
            _ViewStart: state.ViewStart(),
            _ViewEnd:   state.ViewEnd(),
            _CursorA:   state.CursorA(),
            _CursorB:   state.CursorB(),
            _Radix:     state.Radix(),
            _Palette:   palette,
        })
        .width(Length::Fill)
        .height(Length::Fixed(K_WAVEFORM_ROW_HEIGHT))
        .into();

        rows = rows.push(row![signalButton, text(dispVal).size(12).width(Length::Fixed(80.0)), waveform].align_y(Alignment::Center));
        index += 1;
    }
    let body = scrollable(rows.padding([2, 8])).height(Length::Fill);
    container(column![header, ruler, body].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::content_container(palette))
        .into()
}
