//-- waveform.rs ----------------------------------------------------------------------------------------------------
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	crate::rube::{ VcdDisplayModel, VcdSignal };
use	iced::widget::{ Space, button, canvas, column, container, row, scrollable, text };
use	iced::{ Alignment, Color, Element, Length, Point, Rectangle, mouse };
const K_VISIBLE_SIGNALS: u32 = 64;
const K_WAVEFORM_ROW_HEIGHT: f32 = 28.0;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Debug, Clone, PartialEq, Eq)]
pub enum WaveformAction {
    Fit,
    ZoomIn,
    ZoomOut,
    PreviousChange,
    NextChange,
    SelectSignal( u32),
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Debug, Clone)]
pub struct WaveformState
{
    _Model: VcdDisplayModel,
    _ViewStart: u64,
    _ViewEnd: u64,
    _CursorTime: u64,
    _SelectedSignal: Option< u32>,
    _Error: Option< String>,
}
impl WaveformState
{
    pub fn	New( model: VcdDisplayModel) -> Self
    {
        let  	viewStart = model._TimeMin;
        let  	viewEnd = model._TimeMax.max( viewStart + 1);
        Self {
            _Model: model,
            _ViewStart: viewStart,
            _ViewEnd: viewEnd,
            _CursorTime: viewStart,
            _SelectedSignal: None,
            _Error: None,
        }
    }
    pub fn	FromError( error: String) -> Self
    {
        Self {
            _Model: VcdDisplayModel::New(),
            _ViewStart: 0,
            _ViewEnd: 1,
            _CursorTime: 0,
            _SelectedSignal: None,
            _Error: Some( error),
        }
    }
    pub fn	Model( &self) -> &VcdDisplayModel
    {
        &self._Model
    }
    pub fn	CursorTime( &self) -> u64
    {
        self._CursorTime
    }
    pub fn	SelectedSignal( &self) -> Option< u32>
    {
        self._SelectedSignal
    }
    pub fn	ViewStart( &self) -> u64
    {
        self._ViewStart
    }
    pub fn	ViewEnd( &self) -> u64
    {
        self._ViewEnd
    }
    pub fn	Update( &mut self, action: WaveformAction)
    {
        match action {
            WaveformAction::Fit => {
                self._ViewStart = self._Model._TimeMin;
                self._ViewEnd = self._Model._TimeMax.max( self._ViewStart + 1);
                self._CursorTime = self._CursorTime.clamp( self._ViewStart, self._ViewEnd);
            }
            WaveformAction::ZoomIn => self.Zoom( 1, 2),
            WaveformAction::ZoomOut => self.Zoom( 2, 1),
            WaveformAction::PreviousChange => self.MoveCursor( false),
            WaveformAction::NextChange => self.MoveCursor( true),
            WaveformAction::SelectSignal( index) => {
                if index < self._Model.SignalCount() {
                    self._SelectedSignal = Some( index);
                }
            }
        }
    }
    fn	Zoom( &mut self, numerator: u64, denominator: u64)
    {
        let  	span = self._ViewEnd.saturating_sub( self._ViewStart).max( 1);
        let  	newSpan = ( span.saturating_mul( numerator) / denominator).max( 1);
        let  	relative = self._CursorTime.saturating_sub( self._ViewStart);
        self._ViewStart = self
            ._CursorTime
            .saturating_sub( relative.saturating_mul( newSpan) / span);
        self._ViewEnd = self._ViewStart.saturating_add( newSpan);
    }
    fn	MoveCursor( &mut self, forward: bool)
    {
        let  	Some( index) = self._SelectedSignal else {
            return;
        };
        let  	Some( signal) = self._Model.Signal( index) else {
            return;
        };
        let  	changes = signal._Changes.AsArr();
        let  	mut target = self._CursorTime;
        changes.USeg().Traverse( |changeIndex| {
            let  	time = changes[changeIndex].0;
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

struct WaveformLane
{
    signal: VcdSignal,
    view_start: u64,
    view_end: u64,
    cursor_time: u64,
    palette: ThemePalette,
}
impl WaveformLane
{
    fn	time_to_x( &self, time: u64, width: f32) -> f32
    {
        let  	span = self.view_end.saturating_sub( self.view_start).max( 1) as f32;
        time.saturating_sub( self.view_start) as f32 * width / span
    }
    fn	level( value: &str, height: f32) -> f32
    {
        match value {
            "1" => height * 0.25,
            "0" => height * 0.75,
            _ => height * 0.5,
        }
    }
}
impl< Message> canvas::Program< Message> for WaveformLane {
    type State = ();
    fn	draw( 
        &self, _state: &Self::State, renderer: &iced::Renderer, _theme: &iced::Theme,
        bounds: Rectangle, _cursor: mouse::Cursor,
    ) -> Vec< canvas::Geometry>
    {
        let  	mut frame = canvas::Frame::new( renderer, bounds.size());
        frame.fill_rectangle( Point::ORIGIN, bounds.size(), self.palette.content_bg);
        let  	wave_color = if self.signal.IsSingleBit() {
            Color::from_rgb8( 111, 210, 150)
        } else {
            Color::from_rgb8( 96, 205, 255)
        };
        let  	stroke = canvas::Stroke::default()
            .with_color( wave_color)
            .with_width( 1.5);
        let  	changes = self.signal._Changes.AsArr();
        if self.signal.IsSingleBit() {
            let  	mut previous_time = self.view_start;
            let  	mut previous_value = self.signal.ValueAt( self.view_start);
            changes.USeg().Traverse( |index| {
                let  	( time, value) = &changes[index];
                if *time <= self.view_start || *time > self.view_end {
                    return;
                }
                let  	x = self.time_to_x( *time, bounds.width);
                frame.stroke( 
                    &canvas::Path::line( 
                        Point::new( 
                            self.time_to_x( previous_time, bounds.width),
                            Self::level( previous_value, bounds.height),
                        ),
                        Point::new( x, Self::level( previous_value, bounds.height)),
                    ),
                    stroke,
                );
                frame.stroke( 
                    &canvas::Path::line( 
                        Point::new( x, Self::level( previous_value, bounds.height)),
                        Point::new( x, Self::level( value, bounds.height)),
                    ),
                    stroke,
                );
                previous_time = *time;
                previous_value = value;
            });
            frame.stroke( 
                &canvas::Path::line( 
                    Point::new( 
                        self.time_to_x( previous_time, bounds.width),
                        Self::level( previous_value, bounds.height),
                    ),
                    Point::new( bounds.width, Self::level( previous_value, bounds.height)),
                ),
                stroke,
            );
        } else {
            let  	center = bounds.height * 0.5;
            frame.stroke( 
                &canvas::Path::line( Point::new( 0.0, center), Point::new( bounds.width, center)),
                stroke,
            );
            changes.USeg().Traverse( |index| {
                let  	( time, _) = &changes[index];
                if *time > self.view_start && *time <= self.view_end {
                    let  	x = self.time_to_x( *time, bounds.width);
                    frame.stroke( 
                        &canvas::Path::line( 
                            Point::new( x, bounds.height * 0.25),
                            Point::new( x, bounds.height * 0.75),
                        ),
                        stroke,
                    );
                }
            });
        }
        if self.cursor_time >= self.view_start && self.cursor_time <= self.view_end {
            let  	x = self.time_to_x( self.cursor_time, bounds.width);
            frame.stroke( 
                &canvas::Path::line( Point::new( x, 0.0), Point::new( x, bounds.height)),
                canvas::Stroke::default()
                    .with_color( self.palette.accent)
                    .with_width( 1.0),
            );
        }
        vec![frame.into_geometry()]
    }
}
pub fn	view_waveform< 'a, Message: 'static + Clone>( 
    state: &'a WaveformState, palette: ThemePalette,
    map_action: impl Fn( WaveformAction) -> Message + Copy + 'static,
) -> Element< 'a, Message> {
    if let  	Some( error) = &state._Error {
        return container( 
            column![
                text( "VCD Parse Error").size( 18),
                Space::new().height( Length::Fixed( 8.0)),
                text( error).size( 13),
            ]
            .padding( 24)
            .spacing( 6),
        )
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into();
    }
    let  	model = state.Model();
    let  	header = row![
        text( "WAVEFORM").size( 14),
        Space::new().width( Length::Fixed( 12.0)),
        text( format!( "{} signals", model.SignalCount())).size( 12),
        Space::new().width( Length::Fixed( 10.0)),
        text( format!( "timescale: {}", model._Timescale)).size( 12),
        Space::new().width( Length::Fill),
        button( text( "Previous").size( 12)).on_press( map_action( WaveformAction::PreviousChange)),
        button( text( "Next").size( 12)).on_press( map_action( WaveformAction::NextChange)),
        button( text( "Zoom -").size( 12)).on_press( map_action( WaveformAction::ZoomOut)),
        button( text( "Zoom +").size( 12)).on_press( map_action( WaveformAction::ZoomIn)),
        button( text( "Fit").size( 12)).on_press( map_action( WaveformAction::Fit)),
    ]
    .spacing( 5)
    .padding( [8, 12])
    .align_y( Alignment::Center);
    let  	ruler = row![
        text( "SIGNAL").size( 11),
        Space::new().width( Length::Fixed( 190.0)),
        text( format!( 
            "TIME  #{}  [#{} - #{}]",
            state.CursorTime(),
            state.ViewStart(),
            state.ViewEnd()
        ))
        .size( 11),
    ]
    .padding( [6, 12]);
    let  	mut rows = column![].spacing( 1);
    let  	visible = model.SignalCount().min( K_VISIBLE_SIGNALS);
    for index in 0..visible {
        let  	Some( signal) = model.Signal( index) else {
            continue;
        };
        let  	selected = state.SelectedSignal() == Some( index);
        let  	name = format!( "{} [{}]", signal._FullName, signal._Bits);
        let  	signalButton = button( text( name).size( 12))
            .width( Length::Fixed( 250.0))
            .padding( [5, 8])
            .style( move |_, status| FasciaStyle::tree_row_button( palette, selected, status))
            .on_press( map_action( WaveformAction::SelectSignal( index)));
        let  	value = signal.ValueAt( state.CursorTime());
        let  	waveform: Element< 'a, Message> = canvas(WaveformLane {
            signal: signal.clone(),
            view_start: state.ViewStart(),
            view_end: state.ViewEnd(),
            cursor_time: state.CursorTime(),
            palette,
        })
        .width( Length::Fill)
        .height( Length::Fixed( K_WAVEFORM_ROW_HEIGHT))
        .into();
        rows = rows.push( 
            row![
                signalButton,
                text( value).size( 12).width( Length::Fixed( 80.0)),
                waveform
            ]
            .align_y( Alignment::Center),
        );
    }
    let  	body = scrollable( rows.padding( [2, 8])).height( Length::Fill);
    container( column![header, ruler, body].height( Length::Fill))
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into()
}
