// src/fascia/python_console.rs -----------------------------------------------------------------

//! # Fascia In-Process Python Console Tab
//!
//! Provides the UI view and state management for an interactive, persistent
//! in-process Python REPL inside Fascia.

use crate::fascia::theme::{FasciaStyle, FasciaTheme, ThemePalette, default_code_font};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length};

#[cfg(feature = "python")]
use std::sync::Arc;

#[cfg(feature = "python")]
pub use crate::python::console::{ConsoleExecutionResult, PythonConsoleEngine};

#[cfg(not(feature = "python"))]
#[derive(Debug, Clone)]
pub struct ConsoleExecutionResult
{
    pub output: String,
    pub is_error: bool,
    pub needs_more_input: bool,
}

//-------------------------------------------------------------------------------------------------

/// A single interaction entry in the console history.
#[derive(Debug, Clone)]
pub struct ConsoleHistoryItem
{
    pub execution_count: u32,
    pub input: String,
    pub output: String,
    pub is_error: bool,
}

//-------------------------------------------------------------------------------------------------

/// Actions emitted by the Python console UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonConsoleAction
{
    InputChanged(String),
    Submit,
    Cancel,
    Clear,
}

//-------------------------------------------------------------------------------------------------

/// State for an open Python console tab.
pub struct PythonConsoleState
{
    pub history: Vec<ConsoleHistoryItem>,
    pub input_line: String,
    pub multiline_buffer: String,
    pub execution_count: u32,
    pub is_running: bool,
    pub is_multiline: bool,
    pub command_history: Vec<String>,
    pub status_message: String,
    #[cfg(feature = "python")]
    pub engine: Option<Arc<PythonConsoleEngine>>,
}

impl Default for PythonConsoleState
{
    fn default() -> Self
    {
        #[cfg(feature = "python")]
        let (engine, status) = match PythonConsoleEngine::new()
        {
            Ok(eng) => (Some(Arc::new(eng)), "Python 3.10 runtime ready. 'trellis' module pre-imported.".to_string()),
            Err(err) => (None, format!("Failed to initialize Python runtime: {}", err)),
        };

        #[cfg(not(feature = "python"))]
        let status = "Python support not enabled in this build. Rebuild with '--features python'.".to_string();

        let initial_history = vec![ConsoleHistoryItem {
            execution_count: 0,
            input: "# Trellis In-Process Python Console".to_string(),
            output: format!(
                "{}\nTry: session = trellis.Session(); print(session.version())",
                status
            ),
            is_error: false,
        }];

        return Self {
            history: initial_history,
            input_line: String::new(),
            multiline_buffer: String::new(),
            execution_count: 1,
            is_running: false,
            is_multiline: false,
            command_history: Vec::new(),
            status_message: status,
            #[cfg(feature = "python")]
            engine,
        };
    }
}

impl PythonConsoleState
{
    pub fn new() -> Self
    {
        return Self::default();
    }

    /// Handles changes in the input line text input.
    pub fn input_changed(&mut self, text: String)
    {
        self.input_line = text;
    }

    /// Prepares submission of the current input line.
    /// Returns the line string to execute if non-empty, and updates multi-line state.
    pub fn submit(&mut self) -> Option<String>
    {
        let trimmed = self.input_line.trim_end().to_string();
        if trimmed.is_empty() && !self.is_multiline
        {
            return None;
        }

        let submitted_line = self.input_line.clone();
        self.input_line.clear();

        if self.is_multiline
        {
            self.multiline_buffer.push('\n');
            self.multiline_buffer.push_str(&submitted_line);
        }
        else
        {
            self.multiline_buffer = submitted_line.clone();
            self.command_history.push(submitted_line.clone());
        }

        self.is_running = true;
        return Some(submitted_line);
    }

    /// Records execution completion from the background task.
    pub fn on_executed(&mut self, result: ConsoleExecutionResult)
    {
        self.is_running = false;

        if result.needs_more_input
        {
            self.is_multiline = true;
            return;
        }

        self.is_multiline = false;
        let full_input = std::mem::take(&mut self.multiline_buffer);

        self.history.push(ConsoleHistoryItem {
            execution_count: self.execution_count,
            input: full_input,
            output: result.output,
            is_error: result.is_error,
        });

        self.execution_count += 1;
    }

    /// Interrupts active execution.
    pub fn cancel(&mut self)
    {
        #[cfg(feature = "python")]
        if let Some(engine) = &self.engine
        {
            engine.interrupt();
        }
        self.is_running = false;
        self.is_multiline = false;
        self.multiline_buffer.clear();
    }

    /// Clears the console history.
    pub fn clear(&mut self)
    {
        self.history.clear();
    }
}

//-------------------------------------------------------------------------------------------------

/// Constructs the UI element for the Python Console tab.
pub fn view_python_console<'a, Message: 'static + Clone>(
    state: &'a PythonConsoleState,
    theme: FasciaTheme,
    palette: ThemePalette,
    map_action: impl Fn(PythonConsoleAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let warning_color = Color::from_rgb8(220, 160, 40);
    let success_color = Color::from_rgb8(106, 153, 85);
    let error_color = Color::from_rgb8(244, 71, 71);

    // 1. Header toolbar
    let status_badge = if state.is_running
    {
        text("● Running...").size(12).style(move |_| text::Style {
            color: Some(warning_color),
        })
    }
    else
    {
        text("● Ready").size(12).style(move |_| text::Style {
            color: Some(success_color),
        })
    };

    let title_row = row![
        text("🐍 Trellis Python Console").size(14).style(move |_| text::Style {
            color: Some(palette.text_primary),
        }),
        Space::new().width(Length::Fixed(12.0)),
        status_badge,
        Space::new().width(Length::Fill),
        button(text("Interrupt").size(12))
            .padding([4, 10])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press_maybe(if state.is_running { Some(map_action(PythonConsoleAction::Cancel)) } else { None }),
        Space::new().width(Length::Fixed(6.0)),
        button(text("Clear").size(12))
            .padding([4, 10])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(PythonConsoleAction::Clear)),
    ]
    .align_y(Alignment::Center)
    .padding([8, 12]);

    let header_bar = container(title_row)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(palette.tab_bar_bg)),
            border: iced::border::rounded(0).color(palette.border_subtle).width(1.0),
            ..Default::default()
        });

    // 2. Scrollable Output History
    let mut history_column = column![].spacing(10).padding(12);

    for item in &state.history
    {
        let prompt_text = if item.execution_count > 0
        {
            format!("In [{}]:", item.execution_count)
        }
        else
        {
            "Info:".to_string()
        };

        let in_label = text(prompt_text)
            .size(12)
            .font(default_code_font(theme))
            .style(move |_| text::Style {
                color: Some(palette.accent),
            });

        let input_text = text(&item.input)
            .size(13)
            .font(default_code_font(theme))
            .style(move |_| text::Style {
                color: Some(palette.text_primary),
            });

        let mut item_col = column![
            row![in_label, Space::new().width(Length::Fixed(8.0)), input_text].align_y(Alignment::Start)
        ];

        let trimmed_output = item.output.trim_end();
        if !trimmed_output.is_empty()
        {
            let out_color = if item.is_error { error_color } else { palette.text_secondary };
            let out_text = text(trimmed_output)
                .size(12)
                .font(default_code_font(theme))
                .style(move |_| text::Style {
                    color: Some(out_color),
                });

            let out_container = container(out_text)
                .padding([6, 10])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(palette.content_bg)),
                    border: iced::border::rounded(4).color(palette.border_subtle).width(1.0),
                    ..Default::default()
                });

            item_col = item_col.push(Space::new().height(Length::Fixed(4.0))).push(out_container);
        }

        history_column = history_column.push(item_col);
    }

    let history_scroll = scrollable(history_column)
        .width(Length::Fill)
        .height(Length::Fill);

    // 3. Interactive Input Prompt
    let prompt_label = if state.is_multiline
    {
        "...: ".to_string()
    }
    else
    {
        format!("In [{}]:", state.execution_count)
    };

    let prompt_badge = text(prompt_label)
        .size(13)
        .font(default_code_font(theme))
        .style(move |_| text::Style {
            color: Some(palette.accent),
        });

    let input_field = text_input(
        if state.is_multiline { "Continue Python code block..." } else { "Enter Python statement or expression..." },
        &state.input_line,
    )
    .font(default_code_font(theme))
    .size(13)
    .padding([6, 8])
    .on_input(move |s| map_action(PythonConsoleAction::InputChanged(s)))
    .on_submit(map_action(PythonConsoleAction::Submit));

    let run_btn = button(text("Run").size(12))
        .padding([6, 12])
        .style(move |_, status| FasciaStyle::tab_button(palette, true, status))
        .on_press(map_action(PythonConsoleAction::Submit));

    let input_row = row![
        prompt_badge,
        Space::new().width(Length::Fixed(8.0)),
        input_field,
        Space::new().width(Length::Fixed(8.0)),
        run_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 12]);

    let input_container = container(input_row)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(palette.tab_bar_bg)),
            border: iced::border::rounded(0).color(palette.border_subtle).width(1.0),
            ..Default::default()
        });

    return column![
        header_bar,
        history_scroll,
        input_container,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
}

