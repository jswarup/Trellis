// src/python/console.rs -------------------------------------------------------------------------

//! # Python In-Process Console Runtime
//!
//! Provides the persistent interactive interpreter session, standard stream
//! redirection, multi-line compilation semantics, and asynchronous execution cancellation.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------

/// Result of evaluating a command line in the interactive Python console.
#[derive(Debug, Clone)]
pub struct ConsoleExecutionResult
{
    pub output: String,
    pub is_error: bool,
    pub needs_more_input: bool,
}

//-------------------------------------------------------------------------------------------------

/// Persistent interactive Python console engine.
pub struct PythonConsoleEngine
{
    console_obj: PyObject,
    pub active_thread_id: Arc<AtomicU64>,
    pub cancelled: Arc<AtomicBool>,
}

impl PythonConsoleEngine
{
    /// Creates and initializes a new persistent console session.
    pub fn new() -> PyResult<Self>
    {
        crate::python::ensure_initialized();
        return Python::with_gil( |py| {
            let code_mod = py.import( "code")?;
            let locals = PyDict::new( py);
            locals.set_item( "__name__", "__main__")?;
            locals.set_item( "__doc__", py.None())?;

            // Automatically expose `trellis` in the console namespace
            if let Ok( trellis_mod) = py.import( "trellis")
            {
                let _ = locals.set_item( "trellis", trellis_mod);
            }

            let console_cls = code_mod.getattr( "InteractiveConsole")?;
            let console = console_cls.call1( (locals,))?;

            return Ok( Self {
                console_obj: console.unbind(),
                active_thread_id: Arc::new( AtomicU64::new( 0)),
                cancelled: Arc::new( AtomicBool::new( false)),
            });
        });
    }

    /// Evaluates a line of input in the persistent interactive console.
    pub fn execute_line( &self, line: &str) -> ConsoleExecutionResult
    {
        crate::python::ensure_initialized();
        self.cancelled.store( false, Ordering::SeqCst);

        return Python::with_gil( |py| {
            // Record current thread ident for asynchronous cancellation
            if let Ok( ident) = py
                .import( "threading")
                .and_then( |th| th.getattr( "get_ident"))
                .and_then( |gi| gi.call0())
                .and_then( |id_obj| id_obj.extract::<u64>())
            {
                self.active_thread_id.store( ident, Ordering::SeqCst);
            }

            let sys = match py.import( "sys")
            {
                Ok( s) => s,
                Err( e) => return ConsoleExecutionResult {
                    output: format!( "Failed to import sys: {}", e),
                    is_error: true,
                    needs_more_input: false,
                },
            };

            let io = match py.import( "io")
            {
                Ok( i) => i,
                Err( e) => return ConsoleExecutionResult {
                    output: format!( "Failed to import io: {}", e),
                    is_error: true,
                    needs_more_input: false,
                },
            };

            // Intercept standard output and standard error
            let string_io = match io.getattr( "StringIO").and_then( |cls| cls.call0())
            {
                Ok( s) => s,
                Err( e) => return ConsoleExecutionResult {
                    output: format!( "Failed to create StringIO: {}", e),
                    is_error: true,
                    needs_more_input: false,
                },
            };

            let old_stdout = sys.getattr( "stdout").ok();
            let old_stderr = sys.getattr( "stderr").ok();

            let _ = sys.setattr( "stdout", &string_io);
            let _ = sys.setattr( "stderr", &string_io);

            let console = self.console_obj.bind( py);
            let push_res = console.call_method1( "push", (line,));

            // Restore standard streams
            if let Some( stdout) = old_stdout
            {
                let _ = sys.setattr( "stdout", stdout);
            }
            if let Some( stderr) = old_stderr
            {
                let _ = sys.setattr( "stderr", stderr);
            }

            self.active_thread_id.store( 0, Ordering::SeqCst);

            let mut output = String::new();
            if let Ok( s) = string_io.call_method0( "getvalue").and_then( |gv| gv.extract::<String>())
            {
                output = s;
            }

            match push_res
            {
                Ok( more_obj) => {
                    let needs_more = more_obj.extract::<bool>().unwrap_or( false);
                    let is_error = output.contains( "Traceback (most recent call last):")
                        || output.contains( "SyntaxError:")
                        || output.contains( "Error:");
                    return ConsoleExecutionResult {
                        output,
                        is_error,
                        needs_more_input: needs_more,
                    };
                }
                Err( err) => {
                    let err_str = format!( "Execution error: {}", err);
                    return ConsoleExecutionResult {
                        output: if output.is_empty() {
                            err_str
                        } else {
                            format!( "{}\n{}", output, err_str)
                        },
                        is_error: true,
                        needs_more_input: false,
                    };
                }
            }
        });
    }

    /// Interrupts active execution by raising KeyboardInterrupt asynchronously in the worker thread.
    pub fn interrupt( &self)
    {
        self.cancelled.store( true, Ordering::SeqCst);
        let thread_id = self.active_thread_id.load( Ordering::SeqCst);
        if thread_id != 0
        {
            unsafe {
                pyo3::ffi::PyThreadState_SetAsyncExc(
                    thread_id as std::os::raw::c_long,
                    pyo3::ffi::PyExc_KeyboardInterrupt,
                );
            }
        }
        return;
    }
}
