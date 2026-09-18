//-- waveform.rs ----------------------------------------------------------------------------------------------------

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::rube::VcdDisplayModel;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

const K_VISIBLE_SIGNALS: u32 = 64;
const K_VISIBLE_CHANGES: u32 = 12;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveformAction {
    Fit,
    ZoomIn,
    ZoomOut,
    PreviousChange,
    NextChange,
    SelectSignal(u32),
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WaveformState {
    _Model: VcdDisplayModel,
    _ViewStart: u64,
    _ViewEnd: u64,
    _CursorTime: u64,
    _SelectedSignal: Option<u32>,
    _Error: Option<String>,
}

impl WaveformState {
    pub fn New(model: VcdDisplayModel) -> Self {
        let viewStart = model._TimeMin;
        let viewEnd = model._TimeMax.max(viewStart + 1);
        Self {
            _Model: model,
            _ViewStart: viewStart,
            _ViewEnd: viewEnd,
            _CursorTime: viewStart,
            _SelectedSignal: None,
            _Error: None,
        }
    }

    pub fn FromError(error: String) -> Self {
        Self {
            _Model: VcdDisplayModel::New(),
            _ViewStart: 0,
            _ViewEnd: 1,
            _CursorTime: 0,
            _SelectedSignal: None,
            _Error: Some(error),
        }
    }

    pub fn Model(&self) -> &VcdDisplayModel {
        &self._Model
    }
    pub fn CursorTime(&self) -> u64 {
        self._CursorTime
    }
    pub fn SelectedSignal(&self) -> Option<u32> {
        self._SelectedSignal
    }
    pub fn ViewStart(&self) -> u64 {
        self._ViewStart
    }
    pub fn ViewEnd(&self) -> u64 {
        self._ViewEnd
    }

    pub fn Update(&mut self, action: WaveformAction) {
        match action {
            WaveformAction::Fit => {
                self._ViewStart = self._Model._TimeMin;
                self._ViewEnd = self._Model._TimeMax.max(self._ViewStart + 1);
                self._CursorTime = self._CursorTime.clamp(self._ViewStart, self._ViewEnd);
            }
            WaveformAction::ZoomIn => self.Zoom(1, 2),
            WaveformAction::ZoomOut => self.Zoom(2, 1),
            WaveformAction::PreviousChange => self.MoveCursor(false),
            WaveformAction::NextChange => self.MoveCursor(true),
            WaveformAction::SelectSignal(index) => {
                if index < self._Model.SignalCount() {
                    self._SelectedSignal = Some(index);
                }
            }
        }
    }

    fn Zoom(&mut self, numerator: u64, denominator: u64) {
        let span = self._ViewEnd.saturating_sub(self._ViewStart).max(1);
        let newSpan = (span.saturating_mul(numerator) / denominator).max(1);
        let relative = self._CursorTime.saturating_sub(self._ViewStart);
        self._ViewStart = self
            ._CursorTime
            .saturating_sub(relative.saturating_mul(newSpan) / span);
        self._ViewEnd = self._ViewStart.saturating_add(newSpan);
    }

    fn MoveCursor(&mut self, forward: bool) {
        let Some(index) = self._SelectedSignal else {
            return;
        };
        let Some(signal) = self._Model.Signal(index) else {
            return;
        };
        let changes = signal._Changes.AsArr();
        let mut target = self._CursorTime;
        changes.USeg().Traverse(|changeIndex| {
            let time = changes[changeIndex].0;
            if forward && time > self._CursorTime && target == self._CursorTime {
                target = time;
            }
            if !forward && time < self._CursorTime {
                target = time;
            }
        });
        self._CursorTime = target;
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

fn signal_timeline(state: &WaveformState, signalIndex: u32) -> String {
    let Some(signal) = state.Model().Signal(signalIndex) else {
        return String::new();
    };
    let changes = signal._Changes.AsArr();
    let mut timeline = String::new();
    let mut shown = 0;
    changes.USeg().Traverse(|changeIndex| {
        let (time, value) = &changes[changeIndex];
        if *time >= state.ViewStart() && *time <= state.ViewEnd() && shown < K_VISIBLE_CHANGES {
            if !timeline.is_empty() {
                timeline.push_str("  |  ");
            }
            timeline.push_str(&format!("#{} {}", time, value));
            shown += 1;
        }
    });
    if timeline.is_empty() {
        format!(
            "#{} {}",
            state.CursorTime(),
            signal.ValueAt(state.CursorTime())
        )
    } else {
        timeline
    }
}

pub fn view_waveform<'a, Message: 'static + Clone>(
    state: &'a WaveformState,
    palette: ThemePalette,
    map_action: impl Fn(WaveformAction) -> Message + Copy + 'static,
) -> Element<'a, Message> {
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
    let header = row![
        text("WAVEFORM").size(14),
        Space::new().width(Length::Fixed(12.0)),
        text(format!("{} signals", model.SignalCount())).size(12),
        Space::new().width(Length::Fixed(10.0)),
        text(format!("timescale: {}", model._Timescale)).size(12),
        Space::new().width(Length::Fill),
        button(text("Previous").size(12)).on_press(map_action(WaveformAction::PreviousChange)),
        button(text("Next").size(12)).on_press(map_action(WaveformAction::NextChange)),
        button(text("Zoom -").size(12)).on_press(map_action(WaveformAction::ZoomOut)),
        button(text("Zoom +").size(12)).on_press(map_action(WaveformAction::ZoomIn)),
        button(text("Fit").size(12)).on_press(map_action(WaveformAction::Fit)),
    ]
    .spacing(5)
    .padding([8, 12])
    .align_y(Alignment::Center);

    let ruler = row![
        text("SIGNAL").size(11),
        Space::new().width(Length::Fixed(190.0)),
        text(format!(
            "TIME  #{}  [#{} - #{}]",
            state.CursorTime(),
            state.ViewStart(),
            state.ViewEnd()
        ))
        .size(11),
    ]
    .padding([6, 12]);

    let mut rows = column![].spacing(1);
    let visible = model.SignalCount().min(K_VISIBLE_SIGNALS);
    for index in 0..visible {
        let Some(signal) = model.Signal(index) else {
            continue;
        };
        let selected = state.SelectedSignal() == Some(index);
        let name = format!("{} [{}]", signal._FullName, signal._Bits);
        let signalButton = button(text(name).size(12))
            .width(Length::Fixed(250.0))
            .padding([5, 8])
            .style(move |_, status| FasciaStyle::tree_row_button(palette, selected, status))
            .on_press(map_action(WaveformAction::SelectSignal(index)));
        let value = signal.ValueAt(state.CursorTime());
        let waveform = text(signal_timeline(state, index))
            .size(12)
            .width(Length::Fill);
        rows = rows.push(
            row![
                signalButton,
                text(value).size(12).width(Length::Fixed(80.0)),
                waveform
            ]
            .align_y(Alignment::Center),
        );
    }

    let body = scrollable(rows.padding([2, 8])).height(Length::Fill);
    container(column![header, ruler, body].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::content_container(palette))
        .into()
}
