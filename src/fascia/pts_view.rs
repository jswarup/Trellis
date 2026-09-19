//-- pts_view.rs ----------------------------------------------------------------------------------------------------
//! Interactive .pts point-cloud viewport, following Kosh's geometry-tab selection behavior.
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	crate::fleck::PtsCloud;
use	crate::silo::IArr;
use	iced::widget::{ Space, button, canvas, column, container, row, text };
use	iced::{ Alignment, Color, Element, Length, Point, Rectangle, mouse };

#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtsViewAction { RotateLeft, RotateRight, ZoomIn, ZoomOut, Reset }

#[derive( Clone)]
pub struct PtsViewerState { _Cloud: PtsCloud, _Yaw: f32, _Pitch: f32, _Zoom: f32, _Error: Option< String> }
impl PtsViewerState {
    pub fn New( cloud: PtsCloud) -> Self { Self { _Cloud: cloud, _Yaw: 0.65, _Pitch: -0.35, _Zoom: 0.9, _Error: None } }
    pub fn FromError( error: String) -> Self { Self { _Cloud: PtsCloud::New(), _Yaw: 0.0, _Pitch: 0.0, _Zoom: 1.0, _Error: Some( error) } }
    pub fn Cloud( &self) -> &PtsCloud { &self._Cloud }
    pub fn Error( &self) -> Option< &str> { self._Error.as_deref() }
    pub fn Update( &mut self, action: PtsViewAction) {
        match action {
            PtsViewAction::RotateLeft => self._Yaw -= 0.2,
            PtsViewAction::RotateRight => self._Yaw += 0.2,
            PtsViewAction::ZoomIn => self._Zoom *= 1.2,
            PtsViewAction::ZoomOut => self._Zoom = ( self._Zoom / 1.2).max( 0.05),
            PtsViewAction::Reset => { self._Yaw = 0.65; self._Pitch = -0.35; self._Zoom = 0.9; }
        }
    }
}

struct PtsCanvas< 'a> { cloud: &'a PtsCloud, yaw: f32, pitch: f32, zoom: f32, palette: ThemePalette }
impl< Message> canvas::Program< Message> for PtsCanvas< '_> {
    type State = ();
    fn draw( &self, _state: &Self::State, renderer: &iced::Renderer, _theme: &iced::Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec< canvas::Geometry> {
        let mut frame = canvas::Frame::new( renderer, bounds.size());
        frame.fill_rectangle( Point::ORIGIN, bounds.size(), self.palette.content_bg);
        let ( bbox_min, bbox_max) = self.cloud.BoundingBox();
        let center_x = ( bbox_min[0] + bbox_max[0]) * 0.5;
        let center_y = ( bbox_min[1] + bbox_max[1]) * 0.5;
        let center_z = ( bbox_min[2] + bbox_max[2]) * 0.5;
        let span = ( bbox_max[0] - bbox_min[0]).max( bbox_max[1] - bbox_min[1]).max( bbox_max[2] - bbox_min[2]).max( 1.0);
        let scale = bounds.width.min( bounds.height) * 0.42 * self.zoom / span;
        let ( sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let ( sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let stride = ( self.cloud.Count().max( 1) / 100_000).max( 1);
        let points = self.cloud.Points().Arr();
        let mut point_index = 0u32;
        points.Traverse( |point| {
            if point_index % stride != 0 {
                point_index += 1;
                return;
            }
            point_index += 1;
            let point = point._Pos;
            let ( x, y, z) = ( point._X - center_x, point._Y - center_y, point._Z - center_z);
            let rotated_x = x * cos_yaw - z * sin_yaw;
            let rotated_z = x * sin_yaw + z * cos_yaw;
            let rotated_y = y * cos_pitch - rotated_z * sin_pitch;
            let depth = y * sin_pitch + rotated_z * cos_pitch;
            let perspective = 1.0 / ( 1.0 + ( depth / span) * 0.25).max( 0.2);
            let screen = Point::new( bounds.width * 0.5 + rotated_x * scale * perspective, bounds.height * 0.5 - rotated_y * scale * perspective);
            frame.fill( &canvas::Path::circle( screen, 1.25), Color::from_rgb8( 137, 220, 235));
        });
        vec![frame.into_geometry()]
    }
}

pub fn view_pts_viewer< 'a, Message: 'static + Clone>( state: &'a PtsViewerState, palette: ThemePalette, map_action: impl Fn( PtsViewAction) -> Message + Copy + 'static) -> Element< 'a, Message> {
    if let Some( error) = state.Error() {
        return container( column![text( "PTS Parse Error").size( 18), text( error).size( 13)].spacing( 8).padding( 24))
            .width( Length::Fill).height( Length::Fill).style( move |_| FasciaStyle::content_container( palette)).into();
    }
    let cloud = state.Cloud();
    let ( bbox_min, bbox_max) = cloud.BoundingBox();
    let header = row![
        text( "POINT CLOUD").size( 14), Space::new().width( Length::Fixed( 12.0)), text( format!( "{} points", cloud.Count())).size( 12), Space::new().width( Length::Fixed( 12.0)),
        text( format!( "bounds: ({:.2}, {:.2}, {:.2}) – ({:.2}, {:.2}, {:.2})", bbox_min[0], bbox_min[1], bbox_min[2], bbox_max[0], bbox_max[1], bbox_max[2])).size( 12), Space::new().width( Length::Fill),
        button( text( "Rotate ←").size( 12)).on_press( map_action( PtsViewAction::RotateLeft)), button( text( "Rotate →").size( 12)).on_press( map_action( PtsViewAction::RotateRight)), button( text( "Zoom −").size( 12)).on_press( map_action( PtsViewAction::ZoomOut)), button( text( "Zoom +").size( 12)).on_press( map_action( PtsViewAction::ZoomIn)), button( text( "Reset").size( 12)).on_press( map_action( PtsViewAction::Reset)),
    ].spacing( 5).padding( [8, 12]).align_y( Alignment::Center);
    let viewport = canvas( PtsCanvas { cloud, yaw: state._Yaw, pitch: state._Pitch, zoom: state._Zoom, palette }).width( Length::Fill).height( Length::Fill);
    container( column![header, viewport].height( Length::Fill)).width( Length::Fill).height( Length::Fill).style( move |_| FasciaStyle::content_container( palette)).into()
}
