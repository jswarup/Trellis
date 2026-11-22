// output.rs --------------------------------------------------------------------------------------
//! Bounded diagnostics log, asynchronous job runner tracking, and result links.

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::silo::{Arr, Stash};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
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
    pub id:         JobId,
    pub title:      String,
    pub uri:        Option<String>,
    pub status:     JobStatus,
    pub started_at: Instant,
    pub ended_at:   Option<Instant>,
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
    _Entries:      Stash<OutputEntry>,
    _Jobs:         Stash<JobRecord>,
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
            _Entries:      Stash::New(),
            _Jobs:         Stash::New(),
            _NextJobId:    1,
            _NextEntryId:  1,
            _ErrorCount:   0,
            _WarningCount: 0,
            _MaxEntries:   Self::DEFAULT_MAX_ENTRIES,
        };
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
        self._Entries.Push(entry);

        // Bounded capacity retention
        while self._Entries.Size() > self._MaxEntries {
            self._Entries.Pop();
        }
    }

    pub fn StartJob(&mut self, title: impl Into<String>, uri: Option<String>) -> JobId
    {
        let id = JobId(self._NextJobId);
        self._NextJobId += 1;
        let title_str = title.into();
        self.Log(DiagnosticSeverity::Info, "Jobs", format!("Started job {}: {}", id, title_str));
        self._Jobs.Push(JobRecord {
            id,
            title: title_str,
            uri,
            status: JobStatus::Pending,
            started_at: Instant::now(),
            ended_at: None,
        });
        return id;
    }

    pub fn UpdateJobProgress(&mut self, id: JobId, progress: f32, stage: impl Into<String>)
    {
        let stage_str = stage.into();
        for i in 0..self._Jobs.Size() {
            let job = &mut self._Jobs[i];
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
        let mut job_title = String::new();
        let uri_clone = result_uri.clone();
        for i in 0..self._Jobs.Size() {
            let job = &mut self._Jobs[i];
            if job.id == id && !job.status.IsTerminal() {
                job_title = job.title.clone();
                job.status = JobStatus::Completed {
                    summary:    summary_str.clone(),
                    result_uri: uri_clone.clone(),
                };
                job.ended_at = Some(Instant::now());
            }
        }
        let msg = format!("Completed job {}: {} ({})", id, job_title, summary_str);
        if let Some(uri) = result_uri {
            self.LogWithLink(DiagnosticSeverity::Info, "Jobs", msg, Some(uri));
        } else {
            self.Log(DiagnosticSeverity::Info, "Jobs", msg);
        }
    }

    pub fn FailJob(&mut self, id: JobId, error: impl Into<String>)
    {
        let error_str = error.into();
        let mut job_title = String::new();
        for i in 0..self._Jobs.Size() {
            let job = &mut self._Jobs[i];
            if job.id == id && !job.status.IsTerminal() {
                job_title = job.title.clone();
                job.status = JobStatus::Failed { error: error_str.clone() };
                job.ended_at = Some(Instant::now());
            }
        }
        self.Log(DiagnosticSeverity::Error, "Jobs", format!("Failed job {}: {} - {}", id, job_title, error_str));
    }

    pub fn CancelJob(&mut self, id: JobId)
    {
        let mut job_title = String::new();
        for i in 0..self._Jobs.Size() {
            let job = &mut self._Jobs[i];
            if job.id == id && !job.status.IsTerminal() {
                job_title = job.title.clone();
                job.status = JobStatus::Cancelled;
                job.ended_at = Some(Instant::now());
            }
        }
        self.Log(DiagnosticSeverity::Warning, "Jobs", format!("Cancelled job {}: {}", id, job_title));
    }

    pub fn GetJob(&self, id: JobId) -> Option<&JobRecord>
    {
        for i in 0..self._Jobs.Size() {
            if self._Jobs[i].id == id {
                return Some(&self._Jobs[i]);
            }
        }
        return None;
    }

    pub fn ActiveJob(&self) -> Option<&JobRecord>
    {
        for i in 0..self._Jobs.Size() {
            if !self._Jobs[i].status.IsTerminal() {
                return Some(&self._Jobs[i]);
            }
        }
        return None;
    }

    pub fn Entries(&self) -> Arr<'_, OutputEntry> { self._Entries.Arr() }
    pub fn Jobs(&self) -> Arr<'_, JobRecord> { self._Jobs.Arr() }
    pub fn ErrorCount(&self) -> u32 { self._ErrorCount }
    pub fn WarningCount(&self) -> u32 { self._WarningCount }

    pub fn Clear(&mut self)
    {
        self._Entries = Stash::New();
        self._ErrorCount = 0;
        self._WarningCount = 0;
    }
}

//-------------------------------------------------------------------------------------------------

/// Renders the visual Output tab widget showing diagnostics and clickable result links.
pub fn view_output<'a, Message: 'static + Clone>(
    store: &OutputStore,
    palette: ThemePalette,
    on_link: impl Fn(String) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let mut log_col = column![].spacing(2).width(Length::Fill);

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
