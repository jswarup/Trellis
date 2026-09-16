// src/zephyr/runtime.rs

use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::zephyr::driver::ZephyrCrewDriver;
use std::any::Any;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Zephyr Types

#[derive(Debug, Clone, PartialEq)]
pub enum ZephyrError {
    NotStarted,
    AlreadyStarted,
    InvalidConfig,
    ExecutionFault(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepBudget {
    pub instruction_limit: u64,
    pub time_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZephyrState {
    Created,
    Running,
    Halted,
    Faulted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    pub instructions_executed: u64,
    pub time_elapsed_ns: u64,
    pub state: ZephyrState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZephyrDiagnostics {
    pub current_state: ZephyrState,
    pub uptime_ns: u64,
    pub exit_reason: Option<String>,
}

//-------------------------------------------------------------------------------------------------
// Runtime Trait

pub trait ZephyrRuntime: Any + Send + Sync {
    fn start(&mut self) -> Result<(), ZephyrError>;
    fn step(&mut self, budget: StepBudget) -> Result<StepResult, ZephyrError>;
    fn reset(&mut self) -> Result<(), ZephyrError>;
    fn stop(&mut self) -> Result<(), ZephyrError>;
    fn state(&self) -> ZephyrState;
    fn diagnostics(&self) -> ZephyrDiagnostics;

    // Helper for downcasting
    fn as_any(&self) -> &dyn Any;
}

//-------------------------------------------------------------------------------------------------
// LibRuntime - Direct simulation execution engine

pub struct LibRuntime {
    _driver: ZephyrCrewDriver,
    _state: ZephyrState,
    _heartbeat_ticks: u64,
}

impl LibRuntime {
    pub fn new(hub: Arc<CrewHub>, node: Arc<CrewNode>, base_addr: u64) -> Self {
        node.set_online(true);
        Self {
            _driver: ZephyrCrewDriver::with_base_addr(hub, node, base_addr),
            _state: ZephyrState::Created,
            _heartbeat_ticks: 0,
        }
    }

    pub fn driver(&self) -> &ZephyrCrewDriver {
        &self._driver
    }
}

impl ZephyrRuntime for LibRuntime {
    fn start(&mut self) -> Result<(), ZephyrError> {
        if self._state == ZephyrState::Running {
            return Err(ZephyrError::AlreadyStarted);
        }
        self._state = ZephyrState::Running;
        Ok(())
    }

    fn step(&mut self, budget: StepBudget) -> Result<StepResult, ZephyrError> {
        if self._state != ZephyrState::Running {
            return Err(ZephyrError::NotStarted);
        }

        // In LibRuntime, one step correlates to one heartbeat tick
        self._heartbeat_ticks += 1;

        Ok(StepResult {
            instructions_executed: budget.instruction_limit,
            time_elapsed_ns: budget.time_ns,
            state: ZephyrState::Running,
        })
    }

    fn reset(&mut self) -> Result<(), ZephyrError> {
        self._heartbeat_ticks = 0;
        self._state = ZephyrState::Created;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ZephyrError> {
        self._state = ZephyrState::Halted;
        Ok(())
    }

    fn state(&self) -> ZephyrState {
        self._state.clone()
    }

    fn diagnostics(&self) -> ZephyrDiagnostics {
        ZephyrDiagnostics {
            current_state: self._state.clone(),
            uptime_ns: self._heartbeat_ticks * 1000,
            exit_reason: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

//-------------------------------------------------------------------------------------------------
// RenodeRuntime — execution engine running compiled ELF inside Renode

pub struct RenodeRuntime {
    _hub: Arc<CrewHub>,
    _node: Arc<CrewNode>,
    _elf_path: std::path::PathBuf,
    _state: ZephyrState,
    _renode_process: Option<std::process::Child>,
    _bridge_handle: Option<std::thread::JoinHandle<()>>,
    _socket: Option<std::net::TcpStream>,
    _listener: Option<std::net::TcpListener>,
    _port: u16,
    _uptime_ns: u64,
}

impl RenodeRuntime {
    pub fn new(hub: Arc<CrewHub>, node: Arc<CrewNode>, elf_path: std::path::PathBuf) -> Self {
        node.set_online(true);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").ok();
        let port = listener
            .as_ref()
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port())
            .unwrap_or(0);

        Self {
            _hub: hub,
            _node: node,
            _elf_path: elf_path,
            _state: ZephyrState::Created,
            _renode_process: None,
            _bridge_handle: None,
            _socket: None,
            _listener: listener,
            _port: port,
            _uptime_ns: 0,
        }
    }

    pub fn port(&self) -> u16 {
        self._port
    }

    pub fn process(&mut self) -> Option<&mut std::process::Child> {
        self._renode_process.as_mut()
    }
}

impl ZephyrRuntime for RenodeRuntime {
    fn start(&mut self) -> Result<(), ZephyrError> {
        use std::io::{Read, Write};

        if self._state == ZephyrState::Running {
            return Err(ZephyrError::AlreadyStarted);
        }

        let renode_exe = std::path::PathBuf::from(r"C:\Tools\Renode\renode.exe");
        if !renode_exe.exists() {
            return Err(ZephyrError::ExecutionFault(format!(
                "Renode executable not found at {}",
                renode_exe.display()
            )));
        }

        let resc_script =
            std::path::PathBuf::from(r"tools\renode\scripts\run-ae350-n25-zephyr.resc");
        let mut cmd = std::process::Command::new(&renode_exe);
        cmd.arg("--plain")
            .arg("--console")
            .arg("-e")
            .arg(format!(
                "$zephyr_elf=@{}; i @{}",
                self._elf_path.display(),
                resc_script.display()
            ))
            .env("CREW_SOCKET_PORT", self._port.to_string());

        let child = cmd
            .spawn()
            .map_err(|e| ZephyrError::ExecutionFault(format!("Failed to spawn Renode: {}", e)))?;
        self._renode_process = Some(child);

        // Accept connection from crew_pydev with timeout
        if let Some(ref listener) = self._listener {
            let mut stream_opt = None;
            for _ in 0..50 {
                listener.set_nonblocking(true).ok();
                if let Ok((stream, _)) = listener.accept() {
                    let _ = stream.set_nonblocking(false);
                    stream_opt = Some(stream);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            if let Some(stream) = stream_opt {
                let mut read_stream = stream.try_clone().map_err(|e| {
                    ZephyrError::ExecutionFault(format!("Socket clone failed: {}", e))
                })?;
                let mut write_stream = stream.try_clone().map_err(|e| {
                    ZephyrError::ExecutionFault(format!("Socket clone failed: {}", e))
                })?;
                self._socket = Some(stream);

                let hub = self._hub.clone();
                let node = self._node.clone();
                let handle = std::thread::spawn(move || {
                    let mut buf = [0u8; 24];
                    while read_stream.read_exact(&mut buf).is_ok() {
                        let req: crate::crew::protocol::ProtocolMessage =
                            unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const _) };
                        let resp = hub.handle_request(&node, &req);
                        let resp_bytes: [u8; 24] = unsafe { std::mem::transmute(resp) };
                        if write_stream.write_all(&resp_bytes).is_err() {
                            break;
                        }
                    }
                });
                self._bridge_handle = Some(handle);
            }
        }

        self._state = ZephyrState::Running;
        Ok(())
    }

    fn step(&mut self, budget: StepBudget) -> Result<StepResult, ZephyrError> {
        if self._state != ZephyrState::Running {
            return Err(ZephyrError::NotStarted);
        }

        // Give the background bridge and emulation thread a slice of real time
        std::thread::sleep(std::time::Duration::from_millis(10));
        self._uptime_ns += budget.time_ns;

        Ok(StepResult {
            instructions_executed: budget.instruction_limit,
            time_elapsed_ns: budget.time_ns,
            state: ZephyrState::Running,
        })
    }

    fn reset(&mut self) -> Result<(), ZephyrError> {
        self.stop()?;
        self.start()?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ZephyrError> {
        if let Some(socket) = self._socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Both);
        }
        if let Some(handle) = self._bridge_handle.take() {
            let _ = handle.join();
        }
        if let Some(mut child) = self._renode_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self._state = ZephyrState::Halted;
        Ok(())
    }

    fn state(&self) -> ZephyrState {
        self._state.clone()
    }

    fn diagnostics(&self) -> ZephyrDiagnostics {
        ZephyrDiagnostics {
            current_state: self._state.clone(),
            uptime_ns: self._uptime_ns,
            exit_reason: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for RenodeRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
