// geometry_view.rs -------------------------------------------------------------------------------
//! Shared OBJ/PTS document state, controls and Iced GPU viewport adapter.
use	crate::fascia::camera::ViewCamera;
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	crate::fleck::geometry::GeometryAsset;
use	crate::swarm::viewport::{ RenderMode, ViewFrame, ViewportRenderer };
use	iced::widget::{ Space, button, column, container, pick_list, row, shader, slider, text };
use	iced::{ Alignment, Element, Event, Length, Point, Rectangle, keyboard, mouse };
use	std::sync::atomic::{ AtomicBool, Ordering };
use	std::sync::{ Arc, Mutex };

//-------------------------------------------------------------------------------------------------

#[derive( Debug, Clone)]
pub enum GeometryAction {
    Orbit( f32, f32),
    Pan( f32, f32),
    Zoom( f32),
    Fit,
    Reset,
    Projection,
    Axis( u32),
    Resize( f32, f32),
    Mode( RenderMode),
    Color( PointColor),
    PointSize( f32),
    Cancel,
    GpuError( String),
}
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointColor {
    Rgb,
    Intensity,
    Height,
}
impl std::fmt::Display for PointColor {
    fn	fmt( &self, f: &mut std::fmt::Formatter< '_>) -> std::fmt::Result
    {
        f.write_str( match self {
            Self::Rgb => "RGB",
            Self::Intensity => "Intensity",
            Self::Height => "Height",
        })
    }
}
pub struct GeometryViewerState
{
    _Asset:         Option< Arc< GeometryAsset>>,
    _Error:         Option< String>,
    _Cancelled:     Arc< AtomicBool>,
    _Camera:        ViewCamera,
    _Size:          [f32; 2],
    _Mode:          RenderMode,
    _Color:         PointColor,
    _PointSize:     f32,
    _GpuError:      Arc< Mutex< Option< String>>>,
}
impl Default for GeometryViewerState {
    fn	default() -> Self
    {
        Self {
            _Asset:         None,
            _Error:         None,
            _Cancelled:     Arc::new( AtomicBool::new( false)),
            _Camera:        ViewCamera::default(),
            _Size:          [0.0, 0.0],
            _Mode:          RenderMode::ShadedWire,
            _Color:         PointColor::Rgb,
            _PointSize:     3.0,
            _GpuError:      Arc::new( Mutex::new( None)),
        }
    }
}
impl Drop for GeometryViewerState {
    fn	drop( &mut self)
    {
        self._Cancelled.store( true, Ordering::Release);
    }
}
impl GeometryViewerState
{
    pub fn	Cancellation( &self) -> Arc< AtomicBool>
    {
        self._Cancelled.clone()
    }
    pub fn	Asset( &self) -> Option< &GeometryAsset>
    {
        self._Asset.as_deref()
    }
    pub fn	Error( &self) -> Option< &str>
    {
        self._Error.as_deref()
    }
    pub fn	Complete( &mut self, result: Result< Arc< GeometryAsset>, String>)
    {
        if self._Cancelled.load( Ordering::Acquire) {
            return;
        }
        match result {
            Ok( asset) => {
                self._Mode = if asset.IsPointCloud() {
                    RenderMode::Points
                }
                else {
                    RenderMode::ShadedWire
                };
                self._Asset = Some( asset);
                if self._Size[0] > 0.0 {
                    self._Camera.Fit( self._Size[0] / self._Size[1].max( 1.0));
                }
            }
            Err( error) => self._Error = Some( error),
        }
    }
    pub fn	Update( &mut self, action: GeometryAction)
    {
        let  	aspect = self._Size[0] / self._Size[1].max( 1.0);
        match action {
            GeometryAction::Orbit( dx, dy) => self._Camera.Orbit( dx, dy),
            GeometryAction::Pan( dx, dy) => self._Camera.Pan( dx, dy, self._Size[1]),
            GeometryAction::Zoom( steps) => self._Camera.Zoom( steps),
            GeometryAction::Fit => self._Camera.Fit( aspect),
            GeometryAction::Reset => self._Camera.Reset( aspect),
            GeometryAction::Projection => self._Camera.ToggleProjection(),
            GeometryAction::Axis( axis) => self._Camera.SetAxis( axis),
            GeometryAction::Resize( w, h) => {
                if self._Size[0] == 0.0 {
                    self._Camera.Fit( w / h.max( 1.0));
                }
                self._Size = [w, h];
            }
            GeometryAction::Mode( mode) => self._Mode = mode,
            GeometryAction::Color( color) => self._Color = color,
            GeometryAction::PointSize( size) => self._PointSize = size.clamp( 1.0, 12.0),
            GeometryAction::GpuError( error) => self._Error = Some( error),
            GeometryAction::Cancel => {
                self._Cancelled.store( true, Ordering::Release);
                self._Error = Some( "Loading cancelled. Close this tab to release it.".into());
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub fn	ViewGeometry< 'a, Message: Clone + 'static>( 
    id: u64, state: &'a GeometryViewerState, palette: ThemePalette,
    map: impl Fn( GeometryAction) -> Message + Copy + 'static,
) -> Element< 'a, Message>
{
    if let  	Some( error) = state.Error() {
        return container( 
            column![
                text( "Unable to display geometry").size( 20),
                text( error).size( 14)
            ]
            .spacing( 12)
            .padding( 28),
        )
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into();
    }
    let  	Some( asset) = &state._Asset else {
        return container( 
            column![
                text( "Loading geometry...").size( 20),
                text( "Reading and preparing the file in the background.").size( 13),
                button( "Cancel").on_press( map( GeometryAction::Cancel))
            ]
            .spacing( 12),
        )
        .center( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into();
    };
    let  	control = |label, action| {
        let  	selected = matches!( &action, GeometryAction::Mode( mode) if *mode == state._Mode);
        button( text( label).size( 12))
            .padding( [6, 10])
            .style( move |_, status| {
                let  	mut style = FasciaStyle::toolbar_button( palette, status);
                if selected {
                    style.text_color = palette.accent;
                    style.border.color = palette.accent;
                    style.border.width = 1.0;
                }
                style
            })
            .on_press( map( action))
    };
    let  	mut modes = row![].spacing( 3);
    if !asset.IsPointCloud() {
        modes = modes
            .push( control( "Solid", GeometryAction::Mode( RenderMode::Solid)))
            .push( control( 
                "Edges",
                GeometryAction::Mode( RenderMode::ShadedWire),
            ))
            .push( control( "Wire", GeometryAction::Mode( RenderMode::Wireframe)));
    }
    modes = modes.push( control( "Points", GeometryAction::Mode( RenderMode::Points)));
    let  	toolbar = row![
        text( if asset.IsPointCloud() {
            "POINT CLOUD"
        }
        else {
            "WAVEFRONT"
        })
        .size( 12),
        modes,
        Space::new().width( Length::Fill),
        control( 
            if state._Camera.IsOrthographic() {
                "Orthographic"
            }
            else {
                "Perspective"
            },
            GeometryAction::Projection
        ),
        control( "Fit  F", GeometryAction::Fit),
        control( "Reset", GeometryAction::Reset)
    ]
    .spacing( 12)
    .padding( [6, 12])
    .align_y( Alignment::Center);
    let  	attributes: Element< '_, Message> =
        if asset.IsPointCloud() || state._Mode == RenderMode::Points {
            row![
                text( "Color").size( 12),
                pick_list( 
                    [PointColor::Rgb, PointColor::Intensity, PointColor::Height],
                    Some( state._Color),
                    move |c| map( GeometryAction::Color( c))
                )
                .text_size( 12),
                text( "Point size").size( 12),
                slider( 1.0..=12.0, state._PointSize, move |s| map( 
                    GeometryAction::PointSize( s)
                ))
                .width( 130),
                text( format!( "{:.0} px", state._PointSize)).size( 12)
            ]
            .spacing( 12)
            .padding( [4, 12])
            .align_y( Alignment::Center)
            .into()
        }
        else {
            Space::new().height( 0).into()
        };
    let  	viewport = shader( GeometryProgram {
        _Id:        id,
        _View:      state,
        _Palette:   palette,
        _Map:       map,
    })
    .width( Length::Fill)
    .height( Length::Fill);
    let  	footer = row![
        text( format!( 
            "{} vertices  |  {} faces  |  {:?}",
            asset.VertexCount(),
            asset.FaceCount(),
            state._Mode
        ))
        .size( 11),
        Space::new().width( Length::Fill),
        text( "Drag: orbit   Shift-drag: pan   Wheel: zoom   F: fit").size( 11)
    ]
    .spacing( 12)
    .padding( [6, 12]);
    container( column![toolbar, attributes, viewport, footer])
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into()
}

//-------------------------------------------------------------------------------------------------

#[derive( Default)]
struct Interaction
{
    _Id:            Option< u64>,
    _Drag:          Option< ( mouse::Button, Point)>,
    _Modifiers:     keyboard::Modifiers,
    _Size:          [f32; 2],
    _Probes:        u8,
}
struct GeometryProgram< 'a, F>
{
    _Id:        u64,
    _View:      &'a GeometryViewerState,
    _Palette:   ThemePalette,
    _Map:       F,
}
impl< Message, F: Fn( GeometryAction) -> Message> shader::Program< Message>
    for GeometryProgram< '_, F> {
    type State = Interaction;
    type Primitive = GeometryPrimitive;
    fn	update( 
        &self, state: &mut Interaction, event: &Event, bounds: Rectangle, cursor: mouse::Cursor,
    ) -> Option< shader::Action< Message>>
    {
        if state._Id != Some( self._Id) {
            *state = Interaction {
                _Id: Some( self._Id),
                ..Default::default()
            };
        }
        let  	publish = |action| Some( shader::Action::publish( ( self._Map)( action)).and_capture());
        match event {
            Event::Window( iced::window::Event::RedrawRequested( _)) => {
                if let  	Some( error) = self._View._GpuError.lock().unwrap().take() {
                    return publish( GeometryAction::GpuError( error));
                }
                if state._Size != [bounds.width, bounds.height] {
                    state._Size = [bounds.width, bounds.height];
                    return publish( GeometryAction::Resize( bounds.width, bounds.height));
                }
                // Give GPU preparation errors one following frame in which to reach the UI.
                if state._Probes < 2 {
                    state._Probes += 1;
                    return Some( shader::Action::request_redraw());
                }
            }
            Event::Window( iced::window::Event::Unfocused)
            | Event::Mouse( mouse::Event::CursorLeft) => {
                state._Drag = None;
                state._Modifiers = keyboard::Modifiers::default();
            }
            Event::Keyboard( keyboard::Event::ModifiersChanged( modifiers)) => {
                state._Modifiers = *modifiers
            }
            Event::Mouse( mouse::Event::ButtonPressed( button)) if cursor.is_over( bounds) => {
                if matches!( 
                    button,
                    mouse::Button::Left | mouse::Button::Middle | mouse::Button::Right
                )
                {
                    state._Drag = cursor.position().map( |p| ( *button, p));
                    return Some( shader::Action::capture());
                }
            }
            Event::Mouse( mouse::Event::ButtonReleased( button)) => {
                if state._Drag.is_some_and( |( b, _)| b == *button) {
                    state._Drag = None;
                    return Some( shader::Action::capture());
                }
            }
            Event::Mouse( mouse::Event::CursorMoved { position }) => {
                if let  	Some( ( button, previous)) = state._Drag {
                    state._Drag = Some( ( button, *position));
                    let  	delta = *position - previous;
                    return publish( 
                        if button == mouse::Button::Middle || state._Modifiers.shift() {
                            GeometryAction::Pan( delta.x, delta.y)
                        } else if button == mouse::Button::Right {
                            GeometryAction::Zoom( -delta.y * 0.05)
                        }
                        else {
                            GeometryAction::Orbit( delta.x, delta.y)
                        },
                    );
                }
            }
            Event::Mouse( mouse::Event::WheelScrolled { delta }) if cursor.is_over( bounds) => {
                let  	steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                return publish( GeometryAction::Zoom( steps));
            }
            Event::Keyboard( keyboard::Event::KeyPressed { key, modifiers, .. })
                if cursor.is_over( bounds) && !modifiers.command() && !modifiers.alt() => {
                match key.as_ref() {
                    keyboard::Key::Character( "f" | "F") => return publish( GeometryAction::Fit),
                    keyboard::Key::Character( "1") => return publish( GeometryAction::Axis( 0)),
                    keyboard::Key::Character( "3") => return publish( GeometryAction::Axis( 1)),
                    keyboard::Key::Character( "7") => return publish( GeometryAction::Axis( 2)),
                    keyboard::Key::Character( "5") => return publish( GeometryAction::Projection),
                    keyboard::Key::Named( keyboard::key::Named::Home) => {
                        return publish( GeometryAction::Reset);
                    }
                    keyboard::Key::Named( keyboard::key::Named::Escape) => {
                        state._Drag = None;
                        return Some( shader::Action::capture());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }
    fn	draw( 
        &self, _state: &Interaction, _cursor: mouse::Cursor, _bounds: Rectangle,
    ) -> GeometryPrimitive
    {
        GeometryPrimitive {
            _Id:            self._Id,
            _Asset:         self._View._Asset.as_ref().unwrap().clone(),
            _Camera:        self._View._Camera,
            _Mode:          self._View._Mode,
            _Color:         self._View._Color,
            _PointSize:     self._View._PointSize,
            _Clear:         self._Palette.content_bg.into_linear(),
            _Error:         self._View._GpuError.clone(),
        }
    }
    fn	mouse_interaction( 
        &self, state: &Interaction, bounds: Rectangle, cursor: mouse::Cursor,
    ) -> mouse::Interaction
    {
        if state._Drag.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over( bounds) {
            mouse::Interaction::Grab
        }
        else {
            mouse::Interaction::default()
        }
    }
}
#[derive( Debug)]
struct GeometryPrimitive
{
    _Id:            u64,
    _Asset:         Arc< GeometryAsset>,
    _Camera:        ViewCamera,
    _Mode:          RenderMode,
    _Color:         PointColor,
    _PointSize:     f32,
    _Clear:         [f32; 4],
    _Error:         Arc< Mutex< Option< String>>>,
}
impl shader::Pipeline for ViewportRenderer {
    fn	new( device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self
    {
        Self::New( device, format)
    }
    fn	trim( &mut self)
    {
        self.Trim();
    }
}
impl shader::Primitive for GeometryPrimitive {
    type Pipeline = ViewportRenderer;
    fn	prepare( 
        &self, pipeline: &mut ViewportRenderer, device: &wgpu::Device, queue: &wgpu::Queue,
        bounds: &Rectangle, viewport: &shader::Viewport,
    )
    {
        if bounds.width < 1.0 || bounds.height < 1.0 {
            return;
        }
        let  	scale = viewport.scale_factor();
        let  	window = viewport.physical_size();
        let  	size = [
            ( bounds.width * scale).ceil() as u32,
            ( bounds.height * scale).ceil() as u32,
        ];
        let  	region = [
            bounds.x * scale / window.width.max( 1) as f32,
            bounds.y * scale / window.height.max( 1) as f32,
            bounds.width * scale / window.width.max( 1) as f32,
            bounds.height * scale / window.height.max( 1) as f32,
        ];
        let  	frame = ViewFrame::New( 
            self._Camera.Matrix( bounds.width / bounds.height),
            size,
            region,
            self._Clear,
            self._Mode,
            self._Color as u32,
            self._PointSize * scale,
        );
        if let  	Err( error) = pipeline.Prepare( self._Id, &self._Asset, device, queue, frame) {
            *self._Error.lock().unwrap() = Some( error);
        }
    }
    fn	render( 
        &self, pipeline: &ViewportRenderer, encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView, clip: &Rectangle< u32>,
    )
    {
        pipeline.Render( 
            self._Id,
            encoder,
            target,
            [clip.x, clip.y, clip.width, clip.height],
        );
    }
}

//-------------------------------------------------------------------------------------------------
