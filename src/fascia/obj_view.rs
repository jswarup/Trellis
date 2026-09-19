//-- obj_view.rs ----------------------------------------------------------------------------------------------------
//! Wavefront .obj viewport using the same geometry-tab interaction model as Kosh.
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	crate::fleck::{ WaveObjMeshDto, WaveObjModel };
use	crate::silo::IArr;
use	iced::widget::{ Space, button, canvas, column, container, row, text };
use	iced::{ Alignment, Color, Element, Length, Point, Rectangle, mouse };

#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjViewAction { RotateLeft, RotateRight, ZoomIn, ZoomOut, Reset }

#[derive( Clone)]
pub struct ObjViewerState { _Mesh: WaveObjMeshDto, _VertexCount: u32, _FaceCount: u32, _Yaw: f32, _Pitch: f32, _Zoom: f32, _Error: Option< String> }
impl ObjViewerState {
    pub fn New( model: WaveObjModel) -> Self {
        Self { _Mesh: model.ToMeshDto(), _VertexCount: model.VertexCount(), _FaceCount: model.FaceCount(), _Yaw: 0.65, _Pitch: -0.35, _Zoom: 0.9, _Error: None }
    }
    pub fn FromError( error: String) -> Self {
        Self { _Mesh: WaveObjMeshDto::default(), _VertexCount: 0, _FaceCount: 0, _Yaw: 0.0, _Pitch: 0.0, _Zoom: 1.0, _Error: Some( error) }
    }
    pub fn VertexCount( &self) -> u32 { self._VertexCount }
    pub fn FaceCount( &self) -> u32 { self._FaceCount }
    pub fn Error( &self) -> Option< &str> { self._Error.as_deref() }
    pub fn Update( &mut self, action: ObjViewAction) {
        match action {
            ObjViewAction::RotateLeft => self._Yaw -= 0.2,
            ObjViewAction::RotateRight => self._Yaw += 0.2,
            ObjViewAction::ZoomIn => self._Zoom *= 1.2,
            ObjViewAction::ZoomOut => self._Zoom = ( self._Zoom / 1.2).max( 0.05),
            ObjViewAction::Reset => { self._Yaw = 0.65; self._Pitch = -0.35; self._Zoom = 0.9; }
        }
    }
}

struct ObjCanvas< 'a> { mesh: &'a WaveObjMeshDto, yaw: f32, pitch: f32, zoom: f32, palette: ThemePalette }
impl< 'a> ObjCanvas< 'a> {
    fn Project( &self, point: [f32; 3], bounds: Rectangle) -> Point {
        let center = [
            ( self.mesh._BboxMin[0] + self.mesh._BboxMax[0]) * 0.5,
            ( self.mesh._BboxMin[1] + self.mesh._BboxMax[1]) * 0.5,
            ( self.mesh._BboxMin[2] + self.mesh._BboxMax[2]) * 0.5,
        ];
        let span = ( self.mesh._BboxMax[0] - self.mesh._BboxMin[0]).max( self.mesh._BboxMax[1] - self.mesh._BboxMin[1]).max( self.mesh._BboxMax[2] - self.mesh._BboxMin[2]).max( 1.0);
        let ( sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let ( sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let x = point[0] - center[0];
        let y = point[1] - center[1];
        let z = point[2] - center[2];
        let rotated_x = x * cos_yaw - z * sin_yaw;
        let rotated_z = x * sin_yaw + z * cos_yaw;
        let rotated_y = y * cos_pitch - rotated_z * sin_pitch;
        let depth = y * sin_pitch + rotated_z * cos_pitch;
        let scale = bounds.width.min( bounds.height) * 0.42 * self.zoom / span;
        let perspective = 1.0 / ( 1.0 + ( depth / span) * 0.25).max( 0.2);
        Point::new( bounds.width * 0.5 + rotated_x * scale * perspective, bounds.height * 0.5 - rotated_y * scale * perspective)
    }
}
impl< Message> canvas::Program< Message> for ObjCanvas< '_> {
    type State = ();
    fn draw( &self, _state: &Self::State, renderer: &iced::Renderer, _theme: &iced::Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec< canvas::Geometry> {
        let mut frame = canvas::Frame::new( renderer, bounds.size());
        frame.fill_rectangle( Point::ORIGIN, bounds.size(), self.palette.content_bg);
        let points = self.mesh._Points.Arr();
        let stroke = canvas::Stroke::default().with_color( Color::from_rgb8( 203, 166, 247)).with_width( 1.0);
        self.mesh._Edges.Arr().Traverse( |edge| {
            let left = edge[0];
            let right = edge[1];
            if left < self.mesh._Points.Size() && right < self.mesh._Points.Size() {
                frame.stroke( &canvas::Path::line( self.Project( points[left], bounds), self.Project( points[right], bounds)), stroke);
            }
        });
        vec![frame.into_geometry()]
    }
}

pub fn view_obj_viewer< 'a, Message: 'static + Clone>( state: &'a ObjViewerState, palette: ThemePalette, map_action: impl Fn( ObjViewAction) -> Message + Copy + 'static) -> Element< 'a, Message> {
    if let Some( error) = state.Error() {
        return container( column![text( "OBJ Parse Error").size( 18), text( error).size( 13)].spacing( 8).padding( 24))
            .width( Length::Fill).height( Length::Fill).style( move |_| FasciaStyle::content_container( palette)).into();
    }
    let header = row![
        text( "WAVEFRONT 3D").size( 14), Space::new().width( Length::Fixed( 12.0)), text( format!( "{} vertices, {} faces", state.VertexCount(), state.FaceCount())).size( 12), Space::new().width( Length::Fill),
        button( text( "Rotate Left").size( 12)).on_press( map_action( ObjViewAction::RotateLeft)), button( text( "Rotate Right").size( 12)).on_press( map_action( ObjViewAction::RotateRight)), button( text( "Zoom Out").size( 12)).on_press( map_action( ObjViewAction::ZoomOut)), button( text( "Zoom In").size( 12)).on_press( map_action( ObjViewAction::ZoomIn)), button( text( "Reset").size( 12)).on_press( map_action( ObjViewAction::Reset)),
    ].spacing( 5).padding( [8, 12]).align_y( Alignment::Center);
    let viewport = canvas( ObjCanvas { mesh: &state._Mesh, yaw: state._Yaw, pitch: state._Pitch, zoom: state._Zoom, palette }).width( Length::Fill).height( Length::Fill);
    container( column![header, viewport].height( Length::Fill)).width( Length::Fill).height( Length::Fill).style( move |_| FasciaStyle::content_container( palette)).into()
}
