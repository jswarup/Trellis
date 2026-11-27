// document.rs ------------------------------------------------------------------------------------
//! Owned tab payloads and the lifecycle shared by workbench documents.

use crate::fascia::geometry_view::GeometryViewerState;
use crate::fascia::{
    Cask2dViewerState, FrescoViewerState, Plot2dState, PythonConsoleState, SketchViewerState,
    TabId, TabKind, WaveformState,
};
use crate::silo::{Arr, IArr, IArrMut, Stash, USeg};
use iced::widget::text_editor;
use std::sync::atomic::Ordering;

//-------------------------------------------------------------------------------------------------

pub trait IWorkbenchDocument
{
    fn IsDirty(&self) -> bool { return false; }

    fn RequiresCloseConfirmation(&self) -> bool { return self.IsDirty(); }

    fn CancelJobs(&mut self) {}
}

//-------------------------------------------------------------------------------------------------

pub struct EditorDocument
{
    _Content: text_editor::Content,
    _Dirty:   bool,
}

impl EditorDocument
{
    pub fn New(content: text_editor::Content) -> Self
    {
        return Self { _Content: content,
                      _Dirty:   false, };
    }

    pub fn Content(&self) -> &text_editor::Content { return &self._Content; }

    pub fn ContentMut(&mut self) -> &mut text_editor::Content { return &mut self._Content; }

    pub fn SetDirty(&mut self, dirty: bool) { self._Dirty = dirty; }
}

impl IWorkbenchDocument for EditorDocument
{
    fn IsDirty(&self) -> bool { return self._Dirty; }

    // The shell currently prompts only for sketches; retain its existing close policy.
    fn RequiresCloseConfirmation(&self) -> bool { return false; }
}

impl IWorkbenchDocument for SketchViewerState
{
    fn IsDirty(&self) -> bool { return SketchViewerState::IsDirty(self); }

    fn CancelJobs(&mut self) { self.job_cancellation.store(true, Ordering::Release); }
}

impl IWorkbenchDocument for PythonConsoleState
{
    fn CancelJobs(&mut self) { self.cancel(); }
}

impl IWorkbenchDocument for GeometryViewerState
{
    fn CancelJobs(&mut self) { self.Cancellation().store(true, Ordering::Release); }
}

impl IWorkbenchDocument for WaveformState {}
impl IWorkbenchDocument for Cask2dViewerState {}
impl IWorkbenchDocument for FrescoViewerState {}
impl IWorkbenchDocument for Plot2dState {}

//-------------------------------------------------------------------------------------------------

/// One owned payload per document tab. Boxing keeps the store's entries small.
pub enum TabDocument
{
    Editor(Box<EditorDocument>),
    Waveform(Box<WaveformState>),
    PythonConsole(Box<PythonConsoleState>),
    Geometry(Box<GeometryViewerState>),
    Cask2d(Box<Cask2dViewerState>),
    Sketch(Box<SketchViewerState>),
    Fresco(Box<FrescoViewerState>),
    Plot2d(Box<Plot2dState>),
}

macro_rules! DelegateLifecycle {
    ($document:expr, $method:ident) => {
        match $document {
            TabDocument::Editor(state) => state.$method(),
            TabDocument::Waveform(state) => state.$method(),
            TabDocument::PythonConsole(state) => state.$method(),
            TabDocument::Geometry(state) => state.$method(),
            TabDocument::Cask2d(state) => state.$method(),
            TabDocument::Sketch(state) => state.$method(),
            TabDocument::Fresco(state) => state.$method(),
            TabDocument::Plot2d(state) => state.$method(),
        }
    };
}

impl IWorkbenchDocument for TabDocument
{
    fn IsDirty(&self) -> bool { return DelegateLifecycle!(self, IsDirty); }

    fn RequiresCloseConfirmation(&self) -> bool
    {
        return DelegateLifecycle!(self, RequiresCloseConfirmation);
    }

    fn CancelJobs(&mut self)
    {
        DelegateLifecycle!(self, CancelJobs);
    }
}

impl TabDocument
{
    pub fn cancel(&mut self) { self.CancelJobs(); }

    pub fn MatchesKind(&self, kind: TabKind) -> bool
    {
        return matches!((self, kind),
                        (Self::Editor(_), TabKind::FileEditor)
                        | (Self::Waveform(_), TabKind::VcdViewer)
                        | (Self::PythonConsole(_), TabKind::PythonConsole)
                        | (Self::Geometry(_),
                           TabKind::PtsViewer | TabKind::ObjViewer | TabKind::CaskViewer)
                        | (Self::Cask2d(_), TabKind::Cask2dViewer)
                        | (Self::Sketch(_), TabKind::SketchViewer)
                        | (Self::Fresco(_), TabKind::FrescoViewer)
                        | (Self::Plot2d(_), TabKind::Plot2d));
    }
}

//-------------------------------------------------------------------------------------------------

pub(crate) struct DocumentStore
{
    _Entries: Stash<(TabId, TabDocument)>,
}

macro_rules! DocumentAccessors {
    ($get:ident, $getMut:ident, $count:ident, $insert:ident, $variant:ident, $type:ty) => {
        pub fn $get(&self, id: TabId) -> Option<&$type>
        {
            return match self.Get(id) {
                Some(TabDocument::$variant(state)) => Some(state),
                _ => None,
            };
        }

        pub fn $getMut(&mut self, id: TabId) -> Option<&mut $type>
        {
            return match self.GetMut(id) {
                Some(TabDocument::$variant(state)) => Some(state),
                _ => None,
            };
        }

        #[allow(dead_code)]
        pub fn $count(&self) -> u32
        {
            let mut count = 0;
            self._Entries.Arr().Traverse(|(_, document)| {
                                   if matches!(document, TabDocument::$variant(_)) {
                                       count += 1;
                                   }
                               });
            return count;
        }

        pub fn $insert(&mut self, id: TabId, state: $type)
        {
            self.Insert(id, TabDocument::$variant(Box::new(state)));
        }
    };
}

impl DocumentStore
{
    pub fn New() -> Self { return Self { _Entries: Stash::New(), }; }

    pub fn Entries(&self) -> Arr<'_, (TabId, TabDocument)> { return self._Entries.Arr(); }

    pub fn Len(&self) -> u32 { return self._Entries.Size(); }

    fn Find(&self, id: TabId) -> Result<u32, u32>
    {
        return USeg::FromLen(self._Entries.Size()).BinarySearch(|index| {
                                                      self._Entries[index].0.0.cmp(&id.0)
                                                  });
    }

    pub fn Get(&self, id: TabId) -> Option<&TabDocument>
    {
        let index = self.Find(id).ok()?;
        return Some(&self._Entries[index].1);
    }

    pub fn GetMut(&mut self, id: TabId) -> Option<&mut TabDocument>
    {
        let index = self.Find(id).ok()?;
        return Some(&mut self._Entries[index].1);
    }

    pub fn Insert(&mut self, id: TabId, document: TabDocument)
    {
        let index = match self.Find(id) {
            Ok(index) => {
                self._Entries[index].1.CancelJobs();
                self._Entries[index].1 = document;
                return;
            }
            Err(index) => index,
        };
        let count = self._Entries.Size();
        self._Entries.Push((id, document));
        USeg::WithLen(index, count - index).TraverseRev(|at| {
                                               self._Entries.MutArr().Swap(at, at + 1)
                                           });
    }

    pub fn Remove(&mut self, id: TabId) -> Option<TabDocument>
    {
        let index = self.Find(id).ok()?;
        USeg::WithLen(index, self._Entries.Size() - index - 1).Traverse(|at| {
                                                                  self._Entries
                                                                      .MutArr()
                                                                      .Swap(at, at + 1)
                                                              });
        let (_, mut document) = self._Entries.Pop().unwrap();
        document.CancelJobs();
        return Some(document);
    }

    pub fn Clear(&mut self)
    {
        self._Entries
            .MutArr()
            .TraverseMut(|(_, document)| document.CancelJobs());
        self._Entries.Clear();
    }

    pub fn TraverseMut(&mut self, mut action: impl FnMut(&mut TabDocument))
    {
        self._Entries
            .MutArr()
            .TraverseMut(|(_, document)| action(document));
    }

    pub fn Editor(&self, id: TabId) -> Option<&text_editor::Content>
    {
        return match self.Get(id) {
            Some(TabDocument::Editor(state)) => Some(state.Content()),
            _ => None,
        };
    }

    pub fn EditorMut(&mut self, id: TabId) -> Option<&mut text_editor::Content>
    {
        return match self.GetMut(id) {
            Some(TabDocument::Editor(state)) => Some(state.ContentMut()),
            _ => None,
        };
    }

    pub fn EditorCount(&self) -> u32
    {
        let mut count = 0;
        self._Entries.Arr().Traverse(|(_, document)| {
                               if matches!(document, TabDocument::Editor(_)) {
                                   count += 1;
                               }
                           });
        return count;
    }

    pub fn InsertEditor(&mut self, id: TabId, content: text_editor::Content)
    {
        self.Insert(id,
                    TabDocument::Editor(Box::new(EditorDocument::New(content))));
    }

    pub fn SetEditorDirty(&mut self, id: TabId, dirty: bool)
    {
        if let Some(TabDocument::Editor(state)) = self.GetMut(id) {
            state.SetDirty(dirty);
        }
    }

    DocumentAccessors!(Waveform,
                       WaveformMut,
                       WaveformCount,
                       InsertWaveform,
                       Waveform,
                       WaveformState);
    DocumentAccessors!(Console,
                       ConsoleMut,
                       ConsoleCount,
                       InsertConsole,
                       PythonConsole,
                       PythonConsoleState);
    DocumentAccessors!(Geometry,
                       GeometryMut,
                       GeometryCount,
                       InsertGeometry,
                       Geometry,
                       GeometryViewerState);
    DocumentAccessors!(Cask2d,
                       Cask2dMut,
                       Cask2dCount,
                       InsertCask2d,
                       Cask2d,
                       Cask2dViewerState);
    DocumentAccessors!(Sketch,
                       SketchMut,
                       SketchCount,
                       InsertSketch,
                       Sketch,
                       SketchViewerState);
    DocumentAccessors!(Fresco,
                       FrescoMut,
                       FrescoCount,
                       InsertFresco,
                       Fresco,
                       FrescoViewerState);
    DocumentAccessors!(Plot2d,
                       Plot2dMut,
                       Plot2dCount,
                       InsertPlot2d,
                       Plot2d,
                       Plot2dState);
}

impl Drop for DocumentStore
{
    fn drop(&mut self) { self.Clear(); }
}

//-------------------------------------------------------------------------------------------------
