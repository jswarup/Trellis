// geometry_load.rs -----------------------------------------------------------------------------------------------------------

//! Bounded background import queue. Heist executes each job off the application thread.
use crate::fleck::geometry::GeometryAsset;
use crate::fleck::{ParsePts, ParseWaveObj};
use crate::heist::Atelier;
use iced::futures::channel::oneshot;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, mpsc};

//-----------------------------------------------------------------------------------------------------------------------------

type LoadResult = Result<Arc<GeometryAsset>, String>;
struct LoadJob {
    _Path: PathBuf,
    _Cancelled: Arc<AtomicBool>,
    _Reply: oneshot::Sender<LoadResult>,
}

//-----------------------------------------------------------------------------------------------------------------------------

fn Queue() -> &'static Result<mpsc::SyncSender<LoadJob>, String> {
    static QUEUE: OnceLock<Result<mpsc::SyncSender<LoadJob>, String>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<LoadJob>(8);
        std::thread::Builder::new()
            .name("geometry-import".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    if job._Cancelled.load(Ordering::Acquire) {
                        continue;
                    }
                    let atelier = Atelier::New(1);
                    atelier.MainMaestro().Post(move |_| {
                        // An input failure must not unwind through the scheduler and strand its jobs.
                        let result =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Read(&job)))
                                .unwrap_or_else(|_| {
                                    Err("The geometry parser rejected this file.".into())
                                });
                        let _ = job._Reply.send(result);
                    });
                    atelier.DoLaunch();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(sender)
    })
}

//-----------------------------------------------------------------------------------------------------------------------------

fn Read(job: &LoadJob) -> LoadResult {
    let check = || {
        if job._Cancelled.load(Ordering::Acquire) {
            Err("Loading cancelled.".to_string())
        } else {
            Ok(())
        }
    };
    check()?;
    let content = std::fs::read_to_string(&job._Path).map_err(|e| e.to_string())?;
    check()?;
    let ext = job._Path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let asset = if ext.eq_ignore_ascii_case("pts") {
        let cloud = ParsePts(&content)?;
        check()?;
        GeometryAsset::FromPts(cloud)?
    } else if ext.eq_ignore_ascii_case("obj") {
        let model = ParseWaveObj(&content)?;
        check()?;
        GeometryAsset::FromObj(model)?
    } else {
        return Err("Unsupported geometry format.".into());
    };
    check()?;
    Ok(Arc::new(asset))
}

//-----------------------------------------------------------------------------------------------------------------------------

pub async fn Load(path: PathBuf, cancelled: Arc<AtomicBool>) -> LoadResult {
    let (reply, receiver) = oneshot::channel();
    Queue()
        .as_ref()
        .map_err(Clone::clone)?
        .try_send(LoadJob {
            _Path: path,
            _Cancelled: cancelled,
            _Reply: reply,
        })
        .map_err(|_| {
            "The import queue is busy. Close this tab and try again shortly.".to_string()
        })?;
    receiver
        .await
        .map_err(|_| "Loading cancelled or the import worker stopped.".to_string())?
}

//-------------------------------------------------------------------------------------------------
