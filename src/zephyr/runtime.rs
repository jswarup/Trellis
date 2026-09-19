// src/zephyr/runtime.rs
use	crate::crew::hub::CrewHub;
use	crate::crew::node::CrewNode;
use	crate::zephyr::driver::ZephyrCrewDriver;
use	std::any::Any;
use	std::sync::Arc;
use	std::sync::atomic::{ AtomicBool, Ordering };

//-------------------------------------------------------------------------------------------------
// Zephyr Types
#[derive( Debug, Clone, PartialEq)]
pub enum ZephyrError {
    NotStarted,
    AlreadyStarted,
    InvalidConfig,
    ExecutionFault( String),
}
#[derive( Debug, Clone, PartialEq)]
pub struct StepBudget
{
    pub instruction_limit: u64,
    pub time_ns: u64,
}
#[derive( Debug, Clone, PartialEq)]
pub enum ZephyrState {
    Created,
    Running,
    Halted,
    Faulted,
}
#[derive( Debug, Clone, PartialEq)]
pub struct StepResult
{
    pub instructions_executed: u64,
    pub time_elapsed_ns: u64,
    pub state: ZephyrState,
}
#[derive( Debug, Clone, PartialEq)]
pub struct ZephyrDiagnostics
{
    pub current_state: ZephyrState,
    pub uptime_ns: u64,
    pub exit_reason: Option< String>,
}

//-------------------------------------------------------------------------------------------------
// Runtime Trait
pub trait IZephyrRuntime: Any + Send + Sync {
    fn	start( &mut self) -> Result< (), ZephyrError>;
    fn	step( &mut self, budget: StepBudget) -> Result< StepResult, ZephyrError>;
    fn	reset( &mut self) -> Result< (), ZephyrError>;
    fn	stop( &mut self) -> Result< (), ZephyrError>;
    fn	state( &self) -> ZephyrState;
    fn	diagnostics( &self) -> ZephyrDiagnostics;
    // Helper for downcasting
    fn	as_any( &self) -> &dyn Any;
}
pub use	IZephyrRuntime as ZephyrRuntime;

//-------------------------------------------------------------------------------------------------
// LibRuntime - Direct simulation execution engine
pub struct LibRuntime
{
    _driver: ZephyrCrewDriver,
    _state: ZephyrState,
    _heartbeat_ticks: u64,
}
impl LibRuntime
{
    pub fn	new( hub: Arc< CrewHub>, node: Arc< CrewNode>, base_addr: u64) -> Self
    {
        node.set_online( true);
        Self {
            _driver: ZephyrCrewDriver::with_base_addr( hub, node, base_addr),
            _state: ZephyrState::Created,
            _heartbeat_ticks: 0,
        }
    }
    pub fn	driver( &self) -> &ZephyrCrewDriver
    {
        &self._driver
    }
}
impl IZephyrRuntime for LibRuntime {
    fn	start( &mut self) -> Result< (), ZephyrError>
    {
        if self._state == ZephyrState::Running {
            return Err( ZephyrError::AlreadyStarted);
        }
        self._state = ZephyrState::Running;
        Ok( ())
    }
    fn	step( &mut self, budget: StepBudget) -> Result< StepResult, ZephyrError>
    {
        if self._state != ZephyrState::Running {
            return Err( ZephyrError::NotStarted);
        }
        // In LibRuntime, one step correlates to one heartbeat tick
        self._heartbeat_ticks += 1;
        Ok( StepResult {
            instructions_executed: budget.instruction_limit,
            time_elapsed_ns: budget.time_ns,
            state: ZephyrState::Running,
        })
    }
    fn	reset( &mut self) -> Result< (), ZephyrError>
    {
        self._heartbeat_ticks = 0;
        self._state = ZephyrState::Created;
        Ok( ())
    }
    fn	stop( &mut self) -> Result< (), ZephyrError>
    {
        self._state = ZephyrState::Halted;
        Ok( ())
    }
    fn	state( &self) -> ZephyrState
    {
        self._state.clone()
    }
    fn	diagnostics( &self) -> ZephyrDiagnostics
    {
        ZephyrDiagnostics {
            current_state: self._state.clone(),
            uptime_ns: self._heartbeat_ticks * 1000,
            exit_reason: None,
        }
    }
    fn	as_any( &self) -> &dyn Any
    {
        self
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
// RenodeRuntime — execution engine running compiled ELF inside Renode
pub struct RenodeRuntime
{
    _config: crate::zephyr::config::ZephyrVmConfig,
    _hub: Arc< CrewHub>,
    _node: Arc< CrewNode>,
    _state: ZephyrState,
    _renode_process: Option< std::process::Child>,
    _bridge_handle: Option< std::thread::JoinHandle< ()>>,
    _bridge_active: Arc< AtomicBool>,
    _monitor_stream: Option< std::net::TcpStream>,
    _uptime_ns: u64,
    _executed_instructions: u64,
}
impl RenodeRuntime
{
    pub fn	new( 
        config: crate::zephyr::config::ZephyrVmConfig,
        hub: Arc< CrewHub>,
        node: Arc< CrewNode>,
    ) -> Self
    {
        node.set_online( true);
        Self {
            _config: config,
            _hub: hub,
            _node: node,
            _state: ZephyrState::Created,
            _renode_process: None,
            _bridge_handle: None,
            _bridge_active: Arc::new( AtomicBool::new( false)),
            _monitor_stream: None,
            _uptime_ns: 0,
            _executed_instructions: 0,
        }
    }
    pub fn	process( &mut self) -> Option< &mut std::process::Child>
    {
        self._renode_process.as_mut()
    }
}
fn	read_monitor_prompt( 
    stream: &mut std::net::TcpStream,
    timeout: std::time::Duration,
) -> Result< String, String>
{
    use	std::io::Read;
    let  	_ = stream.set_read_timeout( Some( timeout));
    let  	mut buf = [0u8; 1024];
    let  	mut output = String::new();
    loop {
        match stream.read( &mut buf) {
            Ok( 0) => break,
            Ok( n) => {
                let  	s = String::from_utf8_lossy( &buf[..n]);
                output.push_str( &s);
                if output.ends_with( ") ") || output.ends_with( "> ") {
                    break;
                }
            }
            Err( e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {
                return Err( "Timeout reading from Renode monitor".to_string());
            }
            Err( e) => return Err( e.to_string()),
        }
    }
    Ok( output)
}
fn	send_monitor_command( 
    stream: &mut std::net::TcpStream,
    cmd: &str,
    timeout: std::time::Duration,
) -> Result< String, String>
{
    use	std::io::Write;
    stream
        .write_all( cmd.as_bytes())
        .map_err( |e| e.to_string())?;
    stream.flush().map_err( |e| e.to_string())?;
    read_monitor_prompt( stream, timeout)
}
impl IZephyrRuntime for RenodeRuntime {
    fn	start( &mut self) -> Result< (), ZephyrError>
    {
        use	std::io::{ Read, Write };
        if self._state == ZephyrState::Running {
            return Err( ZephyrError::AlreadyStarted);
        }
        let  	renode_exe = self
            ._config
            .machine
            .renode_executable
            .clone()
            .unwrap_or_else( || std::path::PathBuf::from( r"C:\Tools\Renode\renode.exe"));
        if !renode_exe.exists() {
            return Err( ZephyrError::ExecutionFault( format!( 
                "Renode executable not found at {}",
                renode_exe.display()
            )));
        }
        let  	resc_script = self
            ._config
            .machine
            .renode_script
            .clone()
            .unwrap_or_else( || {
                std::path::PathBuf::from( r"tools\renode\scripts\run-ae350-n25-zephyr.resc")
            });
        if !resc_script.exists() {
            return Err( ZephyrError::ExecutionFault( format!( 
                "Renode script not found at {}",
                resc_script.display()
            )));
        }
        let  	elf_path = self
            ._config
            .machine
            .firmware_elf
            .clone()
            .unwrap_or_else( || std::path::PathBuf::from( r"out\zephyr\ae350-n25\zephyr.elf"));
        if !elf_path.exists() {
            return Err( ZephyrError::ExecutionFault( format!( 
                "Guest firmware ELF not found at {}",
                elf_path.display()
            )));
        }
        // 1. Bind fresh ephemeral TCP listener for CoSim peripheral socket
        let  	cosim_listener = std::net::TcpListener::bind( "127.0.0.1:0").map_err( |e| {
            ZephyrError::ExecutionFault( format!( "Failed to bind CoSim TCP listener: {}", e))
        })?;
        cosim_listener.set_nonblocking( true).map_err( |e| {
            ZephyrError::ExecutionFault( format!( "Failed to set nonblocking on listener: {}", e))
        })?;
        let  	cosim_port = cosim_listener
            .local_addr()
            .map_err( |e| {
                ZephyrError::ExecutionFault( format!( "Failed to get listener port: {}", e))
            })?
            .port();
        // 2. Find an available ephemeral port for Renode monitor (-P)
        let  	monitor_port = {
            let  	temp = std::net::TcpListener::bind( "127.0.0.1:0").map_err( |e| {
                ZephyrError::ExecutionFault( format!( "Failed to bind monitor temp listener: {}", e))
            })?;
            temp.local_addr()
                .map_err( |e| {
                    ZephyrError::ExecutionFault( format!( "Failed to get monitor port: {}", e))
                })?
                .port()
        };
        // 3. Spawn Renode child process
        let  	mut cmd = std::process::Command::new( &renode_exe);
        cmd.arg( "-P")
            .arg( monitor_port.to_string())
            .arg( "--plain")
            .arg( "--disable-gui")
            .arg( "-e")
            .arg( format!( 
                "$zephyr_elf=@{}; i @{}",
                elf_path.display(),
                resc_script.display()
            ))
            .env( "CREW_SOCKET_PORT", cosim_port.to_string())
            .env( 
                "CREW_BASE_ADDR",
                format!( "0x{:X}", self._config.machine.crew_base_addr),
            )
            .env( "CREW_NODE_ID", self._config.node_id.to_string());
        let  	mut child = cmd
            .spawn()
            .map_err( |e| ZephyrError::ExecutionFault( format!( "Failed to spawn Renode: {}", e)))?;
        // 4. Wait for the Renode monitor. The PyDev script connects only on its first MMIO access.
        let  	timeout_ms = self._config.machine.handshake_timeout_ms.max( 1000);
        let  	poll_interval = std::time::Duration::from_millis( 50);
        let  	max_polls = ( timeout_ms / 50).max( 1);
        let  	mut monitor_stream = None;
        for _ in 0..max_polls {
            if let  	Ok( Some( status)) = child.try_wait() {
                let  	_ = child.kill();
                return Err( ZephyrError::ExecutionFault( format!( 
                    "Renode process exited prematurely during handshake with status: {}",
                    status
                )));
            }
            if monitor_stream.is_none()
                && let  	Ok( mut m_stream) = std::net::TcpStream::connect( ( "127.0.0.1", monitor_port))
            {
                let  	_ = m_stream.set_nonblocking( false);
                use	std::io::Write;
                let  	handshake_cmd = format!( 
                    "sysbus ReadDoubleWord 0x{:X}\r\n",
                    self._config.machine.crew_base_addr
                );
                let  	_ = m_stream.write_all( handshake_cmd.as_bytes());
                let  	_ = m_stream.flush();
                monitor_stream = Some( m_stream);
            }
            if monitor_stream.is_some() {
                break;
            }
            std::thread::sleep( poll_interval);
        }
        let  	mut monitor_stream = match monitor_stream {
            Some( m) => m,
            None => {
                let  	_ = child.kill();
                let  	_ = child.wait();
                return Err( ZephyrError::ExecutionFault( format!( 
                    "Renode monitor connection timed out after {} ms",
                    timeout_ms
                )));
            }
        };
        // Consume initial monitor prompt from Renode
        let  	_ = read_monitor_prompt( &mut monitor_stream, std::time::Duration::from_millis( 2000));
        // 5. Accept the PyDev connection when guest execution first accesses Crew MMIO.
        self._monitor_stream = Some( monitor_stream);
        self._renode_process = Some( child);
        let  	hub = self._hub.clone();
        let  	node = self._node.clone();
        let  	bridge_active = self._bridge_active.clone();
        bridge_active.store( true, Ordering::Release);
        let  	handle = std::thread::spawn( move || {
            while bridge_active.load( Ordering::Acquire) {
                match cosim_listener.accept() {
                    Ok( ( mut stream, _)) => {
                        let  	_ =
                            stream.set_read_timeout( Some( std::time::Duration::from_millis( 100)));
                        let  	mut buf = [0u8; 24];
                        while bridge_active.load( Ordering::Acquire) {
                            match stream.read_exact( &mut buf) {
                                Ok( ()) => {
                                    match crate::crew::protocol::ProtocolMessage::from_le_bytes( 
                                        &buf,
                                    )
                                    {
                                        Ok( req) => {
                                            let  	resp = hub.handle_request( &node, &req);
                                            if stream.write_all( &resp.to_le_bytes()).is_err() {
                                                break;
                                            }
                                        }
                                        Err( _) => break,
                                    }
                                }
                                Err( error)
                                    if error.kind() == std::io::ErrorKind::WouldBlock
                                        || error.kind() == std::io::ErrorKind::TimedOut => {}
                                Err( _) => break,
                            }
                        }
                    }
                    Err( error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep( std::time::Duration::from_millis( 10));
                    }
                    Err( _) => break,
                }
            }
        });
        self._bridge_handle = Some( handle);
        self._state = ZephyrState::Running;
        Ok( ())
    }
    fn	step( &mut self, budget: StepBudget) -> Result< StepResult, ZephyrError>
    {
        if self._state != ZephyrState::Running {
            return Err( ZephyrError::NotStarted);
        }
        // Check if Renode child process is still alive
        if let  	Some( child) = self._renode_process.as_mut()
            && let  	Ok( Some( _)) = child.try_wait()
        {
            self._state = ZephyrState::Halted;
            return Ok( StepResult {
                instructions_executed: 0,
                time_elapsed_ns: 0,
                state: ZephyrState::Halted,
            });
        }
        // Issue bounded step command to Renode monitor
        if let  	Some( monitor) = self._monitor_stream.as_mut() {
            let  	step_cmd = format!( "cpu0 Step {}\r\n", budget.instruction_limit);
            if send_monitor_command( monitor, &step_cmd, std::time::Duration::from_millis( 5000))
                .is_err()
            {
                self._state = ZephyrState::Faulted;
                return Err( ZephyrError::ExecutionFault( 
                    "Failed to issue step command to Renode monitor".to_string(),
                ));
            }
        }
        self._executed_instructions += budget.instruction_limit;
        self._uptime_ns += budget.time_ns;
        Ok( StepResult {
            instructions_executed: budget.instruction_limit,
            time_elapsed_ns: budget.time_ns,
            state: ZephyrState::Running,
        })
    }
    fn	reset( &mut self) -> Result< (), ZephyrError>
    {
        self.stop()?;
        self._node.reset_stats();
        self._node.clear_rx();
        self._node.set_online( true);
        self._uptime_ns = 0;
        self._executed_instructions = 0;
        self.start()?;
        Ok( ())
    }
    fn	stop( &mut self) -> Result< (), ZephyrError>
    {
        self._bridge_active.store( false, Ordering::Release);
        if let  	Some( monitor) = self._monitor_stream.take() {
            let  	_ = monitor.shutdown( std::net::Shutdown::Both);
        }
        if let  	Some( handle) = self._bridge_handle.take() {
            let  	_ = handle.join();
        }
        if let  	Some( mut child) = self._renode_process.take() {
            let  	_ = child.kill();
            let  	_ = child.wait();
        }
        self._state = ZephyrState::Halted;
        Ok( ())
    }
    fn	state( &self) -> ZephyrState
    {
        self._state.clone()
    }
    fn	diagnostics( &self) -> ZephyrDiagnostics
    {
        ZephyrDiagnostics {
            current_state: self._state.clone(),
            uptime_ns: self._uptime_ns,
            exit_reason: None,
        }
    }
    fn	as_any( &self) -> &dyn Any
    {
        self
    }
}
impl Drop for RenodeRuntime {
    fn	drop( &mut self)
    {
        let  	_ = self.stop();
    }
}
