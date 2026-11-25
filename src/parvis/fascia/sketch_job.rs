// sketch_job.rs ----------------------------------------------------------------------------------

//! Bounded background job execution for heavy sketch operations (extrude, revolve, wireframe, SVG, save).
use crate::fenst::geometry::GeometryAsset;
use crate::fenst::sketch::SketchDocument;
use crate::flux::FluxValue;
use crate::heist::Atelier;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

//-------------------------------------------------------------------------------------------------

/// Owned immutable snapshot of a sketch document for background processing.
#[derive(Clone, Debug)]
pub struct SketchDocumentSnapshot
{
    pub doc:      Arc<SketchDocument>,
    pub revision: u64,
    pub metadata: Option<FluxValue>,
    pub path:     Option<PathBuf>,
    pub title:    String,
}

impl SketchDocumentSnapshot
{
    pub fn New(
        doc: &SketchDocument,
        metadata: Option<&FluxValue>,
        path: Option<PathBuf>,
        title: String,
    ) -> Self
    {
        Self {
            doc:      Arc::new(doc.clone()),
            revision: doc.Revision(),
            metadata: metadata.cloned(),
            path,
            title,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Kind of background job to execute.
#[derive(Clone, Debug)]
pub enum SketchJobKind
{
    Wireframe,
    Extrude(f64),
    Revolve
    {
        axis:      crate::fleck::RevolveAxis,
        angle_deg: f64,
    },
    ExportSvg(PathBuf),
    Save(PathBuf),
}

//-------------------------------------------------------------------------------------------------

/// Structured outcome of a background sketch job.
#[derive(Clone, Debug)]
pub enum SketchJobOutcome
{
    Wireframe(Result<Arc<GeometryAsset>, String>),
    Extrude
    {
        depth: f64,
        asset: Result<Arc<GeometryAsset>, String>,
    },
    Revolve
    {
        angle_deg: f64,
        asset:     Result<Arc<GeometryAsset>, String>,
    },
    ExportSvg(Result<PathBuf, String>),
    Save
    {
        path:     PathBuf,
        metadata: Result<FluxValue, String>,
    },
    Cancelled,
}

//-------------------------------------------------------------------------------------------------

/// Execute a heavy sketch operation on the shared persistent Heist worker runtime.
pub fn RunSketchJobOnHeist(
    runtime: &crate::fascia::heist_bridge::HeistUiRuntime,
    job_id: crate::fascia::output::JobId,
    tab_id: crate::fascia::tabs::TabId,
    generation: u64,
    snapshot: SketchDocumentSnapshot,
    kind: SketchJobKind,
    cancelled: Arc<AtomicBool>,
) -> crate::heist::token::CancellationToken
{
    let title = format!("Sketch {:?}: {}", kind, snapshot.title);
    let snapshot_rev = snapshot.revision;
    return runtime.Submit(
        job_id,
        crate::heist::scheduler::JobPriority::High,
        title,
        move |token, reporter| {
            if cancelled.load(Ordering::Acquire) || token.IsCancelled() {
                reporter.Emit(crate::fascia::heist_bridge::HeistUiEvent::SketchCompleted {
                    tab_id,
                    generation,
                    snapshot_rev,
                    cancellation: cancelled,
                    outcome: SketchJobOutcome::Cancelled,
                });
                return;
            }
            reporter.ReportProgress(0.2, "Running sketch kernel");
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                RunJobInternal(&snapshot, kind, &cancelled)
            }))
            .unwrap_or(SketchJobOutcome::Wireframe(Err("Job panicked during computation".into())));
            reporter.ReportProgress(1.0, "Complete");
            reporter.Emit(crate::fascia::heist_bridge::HeistUiEvent::SketchCompleted {
                tab_id,
                generation,
                snapshot_rev,
                cancellation: cancelled,
                outcome,
            });
        },
    );
}

/// Execute a heavy sketch operation in a bounded Heist worker off the UI thread.
pub async fn ExecuteSketchJob(
    snapshot: SketchDocumentSnapshot,
    kind: SketchJobKind,
    cancelled: Arc<AtomicBool>,
) -> SketchJobOutcome
{
    if cancelled.load(Ordering::Acquire) {
        return SketchJobOutcome::Cancelled;
    }

    let (reply, receiver) = iced::futures::channel::oneshot::channel();
    let thread_token = cancelled.clone();

    let worker = std::thread::Builder::new()
        .name("sketch-job".into())
        .spawn(move || {
            let atelier = Atelier::New(1);
            atelier.MainMaestro().Post(move |_| {
                if thread_token.load(Ordering::Acquire) {
                    let _ = reply.send(SketchJobOutcome::Cancelled);
                    return;
                }
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    RunJobInternal(&snapshot, kind, &thread_token)
                }))
                .unwrap_or(SketchJobOutcome::Wireframe(Err("Job panicked during computation".into())));

                let _ = reply.send(outcome);
            });
            atelier.DoLaunch();
        });

    if let Err(e) = worker {
        return SketchJobOutcome::Wireframe(Err(format!("Could not spawn worker thread: {e}")));
    }

    receiver.await.unwrap_or(SketchJobOutcome::Cancelled)
}

//-------------------------------------------------------------------------------------------------

fn RunJobInternal(
    snapshot: &SketchDocumentSnapshot,
    kind: SketchJobKind,
    cancelled: &Arc<AtomicBool>,
) -> SketchJobOutcome
{
    if cancelled.load(Ordering::Acquire) {
        return SketchJobOutcome::Cancelled;
    }

    match kind {
        SketchJobKind::Wireframe => {
            let opts = crate::fleck::Sketch3dProjectionOptions::default();
            let res = crate::fleck::Sketch3dProjection::ProjectDocument(&snapshot.doc, opts)
                .map(Arc::new);
            SketchJobOutcome::Wireframe(res)
        }
        SketchJobKind::Extrude(depth) => {
            let res = crate::fleck::SketchExtrude::ExtrudeDocument(&snapshot.doc, depth)
                .map(Arc::new);
            SketchJobOutcome::Extrude { depth, asset: res }
        }
        SketchJobKind::Revolve { axis, angle_deg } => {
            let opts = crate::fleck::SketchRevolveOptions {
                axis,
                angle_deg,
                segments: 36,
            };
            let res = crate::fleck::SketchRevolve::RevolveDocument(&snapshot.doc, opts)
                .map(Arc::new);
            SketchJobOutcome::Revolve { angle_deg, asset: res }
        }
        SketchJobKind::ExportSvg(target_path) => {
            let svg = crate::fleck::SketchIo::ExportSvg(&snapshot.doc);
            if cancelled.load(Ordering::Acquire) {
                return SketchJobOutcome::Cancelled;
            }
            let res = std::fs::write(&target_path, svg)
                .map(|()| target_path)
                .map_err(|e| e.to_string());
            SketchJobOutcome::ExportSvg(res)
        }
        SketchJobKind::Save(target_path) => {
            let file_res = crate::fleck::SketchFlux::File(
                &snapshot.doc,
                snapshot.metadata.as_ref(),
            );
            if cancelled.load(Ordering::Acquire) {
                return SketchJobOutcome::Cancelled;
            }
            let outcome_meta = match file_res {
                Ok(file) => match file.Save(&target_path) {
                    Ok(()) => Ok(file.Metadata().clone()),
                    Err(e) => Err(e.to_string()),
                },
                Err(e) => Err(e),
            };
            SketchJobOutcome::Save {
                path: target_path,
                metadata: outcome_meta,
            }
        }
    }
}
