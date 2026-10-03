// geometry_view.rs -------------------------------------------------------------------------------
//! Shared OBJ/PTS document state, controls and Iced GPU viewport adapter.
use crate::fascia::camera::ViewCamera;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fleck::geometry::GeometryAsset;
use crate::swarm::viewport::{RenderMode, ViewFrame, ViewportRenderer};
use iced::widget::{
    Space, button, checkbox, column, container, pick_list, row, shader, slider, text,
};
use iced::{Alignment, Element, Event, Length, Point, Rectangle, keyboard, mouse};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

//-------------------------------------------------------------------------------------------------

#[derive( Debug, Clone)]
pub enum GeometryAction
{
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
    Opacity( f32),
    Directional( bool),
    MaxDepth( u32),
    Refresh,
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
    _Loading:       bool,
    _Asset:         Option< Arc< GeometryAsset>>,
    _Error:         Option< String>,
    _ReloadError:   Option< String>,
    _Cancelled:     Arc< AtomicBool>,
    _Camera:        ViewCamera,
    _Size:          [f32; 2],
    _Mode:          RenderMode,
    _Color:         PointColor,
    _PointSize:     f32,
    _Directional: bool,
    _Opacity:       f32,
    _MaxDepth:      u32,
    _RootDepth:     u32,
    _GpuError:      Arc< Mutex< Option< String>>>,
}
impl Default for GeometryViewerState
{
    fn default() -> Self
    {
        Self {
            _Loading:       true,
            _Asset:         None,
            _Error:         None,
            _ReloadError:   None,
            _Cancelled:     Arc::new( AtomicBool::new( false)),
            _Camera:        ViewCamera::default(),
            _Size:          [0.0, 0.0],
            _Mode:          RenderMode::ShadedWire,
            _Color:         PointColor::Rgb,
            _PointSize:     3.0,
            _Directional: false,
            _Opacity:       1.0,
            _MaxDepth:      0,
            _RootDepth:     0,
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
    pub fn BeginReload( &mut self) -> Arc<AtomicBool>
    {
        self._Cancelled.store( true, Ordering::Release);
        self._Cancelled = Arc::new( AtomicBool::new( false));
        self._Error = None;
        self._ReloadError = None;
        self._Loading = true;
        self.Cancellation()
    }
    pub fn MaxDepth( &self) -> u32 { self._MaxDepth }
    pub fn RootDepth( &self) -> u32 { self._RootDepth }
    pub fn Opacity( &self) -> f32 { self._Opacity }

    pub fn    ReloadError( &self) -> Option<&str>
    {
        self._ReloadError.as_deref()
    }

    pub fn    IsLoading( &self) -> bool
    {
        self._Loading
    }

    pub fn CameraMatrix( &self) -> [f32; 16] { self._Camera.Matrix( 1.0) }
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
    pub fn CurrentViewBox( &self) -> Option<crate::fenst::cask_scene::ViewBox>
    {
        let asset = self._Asset.as_ref()?;
        if self._Size[0] <= 0.0 || self._Size[1] <= 0.0 {
            return None;
        }
        return Some( self._Camera.ViewBox( self._Size, asset.Bounds()));
    }
    pub fn Complete( &mut self, result: Result<Arc<GeometryAsset>, String>)
    {
        if self._Cancelled.load( Ordering::Acquire) {
            return;
        }
        self._Loading = false;
        match result {
            Ok( asset) => {
                self._Error = None;
                self._ReloadError = None;
                let first = self._Asset.is_none();
                self._RootDepth = asset.MaxDepth();
                if self._RootDepth > 0 {
                    self._MaxDepth = if first { self._RootDepth.min( 4) }
                        else { self._MaxDepth.clamp( 1, self._RootDepth) };
                    if first { self._Opacity = 0.0; self._Directional = true; }
                }
                if first { self._Mode = if asset.IsPointCloud() {
                    RenderMode::Points
                }
                else {
                    RenderMode::ShadedWire
                }; }
                self._Asset = Some( asset);
                if first && self._Size[0] > 0.0 {
                    self._Camera.Fit( self._Size[0] / self._Size[1].max( 1.0));
                }
            }
            Err( error) =>
            {
                if self._Asset.is_some()
                {
                    self._ReloadError = Some( error);
                }
                else
                {
                    self._Error = Some( error);
                }
            }
        }
    }
    pub fn Update( &mut self, action: GeometryAction)
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
            GeometryAction::Directional( enabled) => self._Directional = enabled,
            GeometryAction::Opacity( value) => self._Opacity = value.clamp( 0.0, 1.0),
            GeometryAction::MaxDepth( value) => self._MaxDepth = value.clamp( 1, self._RootDepth.max( 1)),
            GeometryAction::GpuError( error) => self._Error = Some( error),
            GeometryAction::Refresh => {},
            GeometryAction::Cancel => {
                self._Loading = false;
                self._Cancelled.store( true, Ordering::Release);
                if self._Asset.is_some()
                {
                    self._ReloadError = Some( "Refresh cancelled. Showing the previous scene.".into());
                }
                else
                {
                    self._Error = Some( "Loading cancelled. Retry to load this document.".into());
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub fn ViewGeometry<'a, Message: Clone + 'static>( id: u64, state: &'a GeometryViewerState,
                                                  palette: ThemePalette,
                                                  map: impl Fn( GeometryAction) -> Message
                                                  + Copy
                                                  + 'static)
                                                  -> Element<'a, Message>
{
    if let  	Some( error) = state.Error() {
        return container( 
            column![
                text( "Unable to display geometry").size( 20),
                text( error).size( 14),
                button( "Retry").on_press( map( GeometryAction::Refresh))
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
    let toolbar = ViewToolbar( state, palette, map);
    let inspector = ViewInspector( state, palette, map);
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
            "{} {}  |  {} faces  |  {:?}",
            if state._Mode == RenderMode::Points { asset.PointCount() } else { asset.VertexCount() },
            if state._Mode == RenderMode::Points { "points" } else { "vertices" },
            asset.FaceCount(),
            state._Mode
        ))
        .size( 11),
        Space::new().width( Length::Fill),
        text( "Drag: orbit   Shift-drag: pan   Wheel: zoom   F: fit").size( 11)
    ]
    .spacing( 12)
    .padding( [6, 12]);
    container( column![toolbar, row![viewport, inspector].height( Length::Fill), footer])
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into()
}

fn ViewToolbar<'a, Message: Clone + 'static>( state: &'a GeometryViewerState,
                                                palette: ThemePalette,
                                                map: impl Fn( GeometryAction) -> Message + Copy + 'static)
                                                -> Element<'a, Message>
{
    let asset = state._Asset.as_ref().unwrap();
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
        text( if state._RootDepth > 0 {
            "CASK 3D"
        }
        else if asset.IsPointCloud() {
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
        control( "Reset", GeometryAction::Reset),
        control( "Refresh", GeometryAction::Refresh)
    ]
    .spacing( 12)
    .padding( [6, 12])
    .align_y( Alignment::Center);
    toolbar.into()
}

fn ViewInspector<'a, Message: Clone + 'static>( state: &'a GeometryViewerState,
                                                palette: ThemePalette,
                                                map: impl Fn( GeometryAction) -> Message + Copy + 'static)
                                                -> Element<'a, Message>
{
    let asset = state._Asset.as_ref().unwrap();
    let pointControls: Element<'_, Message> =
        if asset.IsPointCloud() || state._Mode == RenderMode::Points {
            column![text( "Point color").size( 12),
                    pick_list( [PointColor::Rgb, PointColor::Intensity, PointColor::Height],
                              Some( state._Color),
                              move |color| map( GeometryAction::Color( color))).text_size( 12),
                    text( format!( "Point size: {:.0} px", state._PointSize)).size( 12),
                    slider( 1.0..=12.0, state._PointSize, move |size| {
                        map( GeometryAction::PointSize( size))
                    })].spacing( 8)
                       .into()
        } else {
            Space::new().height( 0).into()
        };
    let transparency = 1.0 - state._Opacity;
    let mut inspector = column![text( "Display").size( 14),
                                text( format!( "{}: {:.1}%",
                                             if state._Directional {
                                                 "Surface cutaway"
                                             } else {
                                                 "Transparency"
                                             },
                                             transparency * 100.0)).size( 12),
                                slider( 0.0..=1.0, transparency, move |value| {
                                    map( GeometryAction::Opacity( 1.0 - value))
                                }).step( 0.005_f32),
                                pointControls,].spacing( 12)
                                               .padding( 12)
                                               .width( 210);
    if !asset.IsPointCloud() {
        inspector =
            inspector.push( checkbox( state._Directional).label( "Directional surfaces")
                                                       .on_toggle( move |enabled| {
                                                           map( GeometryAction::Directional( enabled))
                                                       })
                                                       .text_size( 12));
        if state._Directional {
            inspector = inspector.push( text( "Facing walls fade; far walls remain. Point samples stay visible.").size( 11));
        }
    }
    if state._RootDepth > 0 {
        inspector = inspector.push( text( format!( "Max depth: {} / {}",
                                                state._MaxDepth, state._RootDepth)).size( 12));
        if state._RootDepth > 1 {
            inspector =
                inspector.push( slider( 1..=state._RootDepth, state._MaxDepth, move |depth| {
                                   map( GeometryAction::MaxDepth( depth))
                               }).step( 1_u32));
        }
        inspector =
            inspector.push( text( "Root is level 1. The maximum includes all leaves.").size( 11));
    }
    if state._Loading {
        inspector = inspector
            .push( text( "Refreshing...").size( 12))
            .push( button( "Cancel refresh").on_press( map( GeometryAction::Cancel)));
    }
    if let Some( error) = state.ReloadError()
    {
        inspector = inspector
            .push( text( "Showing the previous scene").size( 12))
            .push( text( error).size( 11))
            .push( button( "Retry refresh").on_press( map( GeometryAction::Refresh)));
    }
    container( inspector).height( Length::Fill)
        .style( move |_| FasciaStyle::sidebar_container( palette)).into()
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
impl<Message, F: Fn( GeometryAction) -> Message> shader::Program<Message> for GeometryProgram<'_, F>
{
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
    fn draw( &self, _state: &Interaction, _cursor: mouse::Cursor, _bounds: Rectangle)
            -> GeometryPrimitive
    {
        GeometryPrimitive {
            _Id:            self._Id,
            _Asset:         self._View._Asset.as_ref().unwrap().clone(),
            _Camera:        self._View._Camera,
            _Mode:          self._View._Mode,
            _Color:         self._View._Color,
            _PointSize:     self._View._PointSize,
            _Directional:   self._View._Directional,
            _Opacity:       self._View._Opacity,
            _MaxDepth:      self._View._MaxDepth,
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
    _Directional: bool,
    _MaxDepth:      u32,
    _Id:            u64,
    _Asset:         Arc< GeometryAsset>,
    _Camera:        ViewCamera,
    _Mode:          RenderMode,
    _Color:         PointColor,
    _PointSize:     f32,
    _Opacity:       f32,
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
impl shader::Primitive for GeometryPrimitive
{
    type Pipeline = ViewportRenderer;
    fn prepare( &self, pipeline: &mut ViewportRenderer, device: &wgpu::Device, queue: &wgpu::Queue,
               bounds: &Rectangle, viewport: &shader::Viewport)
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
        let frame = ViewFrame::New( self._Camera.Matrix( bounds.width / bounds.height),
                                   size,
                                   region,
                                   self._Clear,
                                   self._Mode,
                                   self._Color as u32,
                                   self._PointSize * scale).WithOpacity( self._Opacity)
                                                           .WithDepth( self._MaxDepth)
                                                           .WithDirectional( self._Directional);
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
