// output.rs --------------------------------------------------------------------------------------
//! Bounded diagnostics log, asynchronous job runner tracking, and result links.

use crate::fascia::icons::{IconId, ViewIcon};
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::silo::Arr;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

//-------------------------------------------------------------------------------------------------

/// Stable identifier for a workbench job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

impl core::fmt::Display for JobId
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        return write!(f, "job-{}", self.0);
    }
}

//-------------------------------------------------------------------------------------------------

/// Execution status of a workbench background job.
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus
{
    Pending,
    Running { progress: f32, stage: String },
    Completed { summary: String, result_uri: Option<String> },
    Failed { error: String },
    Cancelled,
}

impl JobStatus
{
    pub fn IsTerminal(&self) -> bool
    {
        return matches!(self, Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled);
    }
}

//-------------------------------------------------------------------------------------------------

/// Record tracking a background job's lifecycle and emitted output.
#[derive(Debug, Clone)]
pub struct JobRecord
{
    pub id:           JobId,
    pub title:        String,
    pub uri:          Option<String>,
    pub status:       JobStatus,
    pub cancellation: Arc<AtomicBool>,
    pub started_at:   Instant,
    pub ended_at:     Option<Instant>,
    pub waveform:     Option<String>,
}

//-------------------------------------------------------------------------------------------------

/// Severity classification for workbench diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity
{
    Info,
    Warning,
    Error,
}

impl DiagnosticSeverity
{
    pub fn Badge(&self) -> &'static str
    {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERR ",
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// A single timestamped diagnostic log entry with optional result URI link.
#[derive(Debug, Clone)]
pub struct OutputEntry
{
    pub id:        u64,
    pub timestamp: Instant,
    pub severity:  DiagnosticSeverity,
    pub channel:   String,
    pub message:   String,
    pub link_uri:  Option<String>,
}

//-------------------------------------------------------------------------------------------------

/// Bounded store for workbench diagnostics, error counters, and background jobs.
pub struct OutputStore
{
    _Entries:      Vec<OutputEntry>,
    _Jobs:         Vec<JobRecord>,
    _NextJobId:    u64,
    _NextEntryId:  u64,
    _ErrorCount:   u32,
    _WarningCount: u32,
    _MaxEntries:   u32,
}

impl Default for OutputStore
{
    fn default() -> Self { Self::New() }
}

impl OutputStore
{
    pub const DEFAULT_MAX_ENTRIES: u32 = 2000;

    pub fn New() -> Self
    {
        return Self {
            _Entries:      Vec::new(),
            _Jobs:         Vec::new(),
            _NextJobId:    1,
            _NextEntryId:  1,
            _ErrorCount:   0,
            _WarningCount: 0,
            _MaxEntries:   Self::DEFAULT_MAX_ENTRIES,
        };
    }

    pub fn SetMaxEntries(&mut self, max: u32)
    {
        self._MaxEntries = max;
        while self._Entries.len() > self._MaxEntries as usize {
            self._Entries.remove(0);
        }
    }

    pub fn Log(
        &mut self,
        severity: DiagnosticSeverity,
        channel: impl Into<String>,
        message: impl Into<String>,
    )
    {
        self.LogWithLink(severity, channel, message, None::<String>);
    }

    pub fn LogWithLink(
        &mut self,
        severity: DiagnosticSeverity,
        channel: impl Into<String>,
        message: impl Into<String>,
        link: Option<impl Into<String>>,
    )
    {
        match severity {
            DiagnosticSeverity::Error => self._ErrorCount += 1,
            DiagnosticSeverity::Warning => self._WarningCount += 1,
            DiagnosticSeverity::Info => {}
        }

        let entry = OutputEntry {
            id:        self._NextEntryId,
            timestamp: Instant::now(),
            severity,
            channel:   channel.into(),
            message:   message.into(),
            link_uri:  link.map(|l| l.into()),
        };
        self._NextEntryId += 1;
        self._Entries.push(entry);

        // Bounded capacity retention: FIFO eviction (remove oldest entry from front)
        while self._Entries.len() > self._MaxEntries as usize {
            self._Entries.remove(0);
        }
    }

    pub fn StartJob(&mut self, title: impl Into<String>, uri: Option<String>) -> JobId
    {
        let id = JobId(self._NextJobId);
        self._NextJobId += 1;
        let title_str = title.into();
        self.Log(DiagnosticSeverity::Info, "Jobs", format!("Started job {}: {}", id, title_str));
        self._Jobs.push(JobRecord {
            id,
            title: title_str,
            uri,
            status: JobStatus::Pending,
            cancellation: Arc::new(AtomicBool::new(false)),
            started_at: Instant::now(),
            ended_at: None,
            waveform: None,
        });
        return id;
    }

    pub fn JobCancellation(&self, id: JobId) -> Option<Arc<AtomicBool>>
    {
        for job in &self._Jobs {
            if job.id == id {
                return Some(job.cancellation.clone());
            }
        }
        return None;
    }

    pub fn ActiveJobCount(&self) -> u32
    {
        return self._Jobs.iter().filter(|j| !j.status.IsTerminal()).count() as u32;
    }

    pub fn UpdateJobProgress(&mut self, id: JobId, progress: f32, stage: impl Into<String>)
    {
        let stage_str = stage.into();
        for job in &mut self._Jobs {
            if job.id == id && !job.status.IsTerminal() {
                job.status = JobStatus::Running {
                    progress: progress.clamp(0.0, 1.0),
                    stage:    stage_str.clone(),
                };
            }
        }
    }

    pub fn CompleteJob(&mut self, id: JobId, summary: impl Into<String>, result_uri: Option<String>)
    {
        let summary_str = summary.into();
        let Some(job) = self.PendingJobMut(id) else { return; };
        let msg = format!("Completed job {}: {} ({})", id, job.title, summary_str);
        job.status = JobStatus::Completed { summary: summary_str, result_uri: result_uri.clone() };
        job.ended_at = Some(Instant::now());
        if let Some(uri) = result_uri {
            self.LogWithLink(DiagnosticSeverity::Info, "Jobs", msg, Some(uri));
        } else {
            self.Log(DiagnosticSeverity::Info, "Jobs", msg);
        }
    }

    pub fn CompleteWaveform(&mut self, id: JobId, waveform: String)
    {
        let Some(job) = self.PendingJobMut(id) else { return; };
        job.waveform = Some(waveform);
        self.CompleteJob(id, "Recorded Rube waveform", Some(format!("waveform://job/{}", id.0)));
    }

    fn PendingJobMut(&mut self, id: JobId) -> Option<&mut JobRecord>
    {
        return self._Jobs.iter_mut().find(|job| job.id == id && !job.status.IsTerminal());
    }

    pub fn FailJob(&mut self, id: JobId, error: impl Into<String>)
    {
        let error_str = error.into();
        let Some(job) = self.PendingJobMut(id) else { return; };
        let message = format!("Failed job {}: {} - {}", id, job.title, error_str);
        job.status = JobStatus::Failed { error: error_str };
        job.ended_at = Some(Instant::now());
        self.Log(DiagnosticSeverity::Error, "Jobs", message);
    }

    pub fn CancelJob(&mut self, id: JobId)
    {
        let Some(job) = self.PendingJobMut(id) else { return; };
        let message = format!("Cancelled job {}: {}", id, job.title);
        job.cancellation.store(true, Ordering::Release);
        job.status = JobStatus::Cancelled;
        job.ended_at = Some(Instant::now());
        self.Log(DiagnosticSeverity::Warning, "Jobs", message);
    }

    pub fn StoppableJob(&self) -> Option<JobId>
    {
        return self._Jobs.iter().find(|job| {
            !job.status.IsTerminal()
                && !job.cancellation.load(Ordering::Acquire)
                && job.uri.as_deref() == Some(crate::fenst::rube_resource::DADDA_EXAMPLE_URI)
        }).map(|job| job.id);
    }

    pub fn RequestCancellation(&mut self, id: JobId)
    {
        let Some(job) = self.PendingJobMut(id) else { return; };
        job.cancellation.store(true, Ordering::Release);
        self.Log(DiagnosticSeverity::Info, "Jobs", format!("Stop requested for {id}"));
    }

    pub fn GetJob(&self, id: JobId) -> Option<&JobRecord>
    {
        return self._Jobs.iter().find(|job| job.id == id);
    }

    pub fn ActiveJob(&self) -> Option<&JobRecord>
    {
        return self._Jobs.iter().find(|job| !job.status.IsTerminal());
    }

    pub fn Entries(&self) -> Arr<'_, OutputEntry> { return self._Entries.as_slice().into(); }
    pub fn Jobs(&self) -> Arr<'_, JobRecord> { return self._Jobs.as_slice().into(); }
    pub fn ErrorCount(&self) -> u32 { return self._ErrorCount; }
    pub fn WarningCount(&self) -> u32 { return self._WarningCount; }

    pub fn Clear(&mut self)
    {
        self._Entries.clear();
        self._ErrorCount = 0;
        self._WarningCount = 0;
    }
}

//-------------------------------------------------------------------------------------------------

/// Renders the visual Output tab widget showing active Heist jobs, diagnostics, and clickable result links.
pub fn view_output<'a, Message: 'static + Clone>(
    store: &OutputStore,
    palette: ThemePalette,
    on_link: impl Fn(String) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    return view_output_with_cancel(store, palette, on_link, None::<fn(JobId) -> Message>);
}

/// Renders the Output tab with optional per-job cancellation buttons for active Heist jobs.
pub fn view_output_with_cancel<'a, Message: 'static + Clone>(
    store: &OutputStore,
    palette: ThemePalette,
    on_link: impl Fn(String) -> Message + Copy + 'static,
    on_cancel: Option<impl Fn(JobId) -> Message + Copy + 'static>,
) -> Element<'a, Message>
{
    let mut log_col = column![].spacing(4).width(Length::Fill);

    let jobs = store.Jobs();
    for j in 0..jobs.Len() {
        let job = &jobs[j];
        if !job.status.IsTerminal() {
            let (pct, stage) = match &job.status {
                JobStatus::Pending => (0.05, "Queued in Heist...".to_string()),
                JobStatus::Running { progress, stage } => (*progress, stage.clone()),
                _ => (1.0, String::new()),
            };
            let mut job_row = row![
                ViewIcon(IconId::ExampleBolt, 12.0, palette.accent),
                text(format!("{} — {} ({}%)", job.title, stage, (pct * 100.0) as u32))
                    .size(11)
                    .style(move |_| text::Style { color: Some(palette.text_primary) }),
                Space::new().width(Length::Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            if let Some(cancel_cb) = on_cancel {
                let id = job.id;
                job_row = job_row.push(
                    button(
                        row![
                            ViewIcon(IconId::Stop, 10.0, palette.status_error()),
                            Space::new().width(4),
                            text("Cancel").size(10).style(move |_| text::Style {
                                color: Some(palette.status_error()),
                            }),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .padding([2, 6])
                    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                    .on_press(cancel_cb(id)),
                );
            }
            log_col = log_col.push(
                container(job_row)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(move |_| FasciaStyle::card_container(palette)),
            );
        }
    }

    if store.Entries().IsEmpty() {
        log_col = log_col.push(
            text("No output generated yet.")
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_muted) }),
        );
    } else {
        let entries = store.Entries();
        for i in 0..entries.Len() {
            let entry = &entries[i];
            let badge_color = match entry.severity {
                DiagnosticSeverity::Info => palette.accent,
                DiagnosticSeverity::Warning => iced::Color::from_rgb(0.9, 0.7, 0.2),
                DiagnosticSeverity::Error => iced::Color::from_rgb(0.9, 0.3, 0.3),
            };

            let badge = text(entry.severity.Badge())
                .size(11)
                .style(move |_| text::Style { color: Some(badge_color) });

            let channel = text(format!("[{}]", entry.channel))
                .size(11)
                .style(move |_| text::Style { color: Some(palette.text_muted) });

            let msg = text(entry.message.clone())
                .size(12)
                .style(move |_| text::Style { color: Some(palette.text_primary) });

            let mut line = row![badge, channel, msg].spacing(6).align_y(Alignment::Center);

            if let Some(ref uri) = entry.link_uri {
                let link_target = uri.clone();
                let link_btn = button(
                    text("Open Result ↗")
                        .size(11)
                        .style(move |_| text::Style { color: Some(palette.accent) }),
                )
                .padding([1, 6])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(on_link(link_target));

                line = line.push(Space::new().width(Length::Fill)).push(link_btn);
            }

            log_col = log_col.push(line);
        }
    }

    return container(
        scrollable(log_col)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(6)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| FasciaStyle::content_container(palette))
    .into();
}
