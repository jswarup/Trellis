// src/zephyr/app.rs

use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::zephyr::driver::ZephyrCrewDriver;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// ZephyrVm — simulates a guest Zephyr RTOS VM running the crew driver and cooperative tasks.

pub struct ZephyrVm {
    _driver: ZephyrCrewDriver,
    _heartbeat_ticks: u32,
}

impl ZephyrVm {
    pub fn new(hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self {
        node.set_online(true);
        Self {
            _driver: ZephyrCrewDriver::new(hub, node),
            _heartbeat_ticks: 0,
        }
    }

    #[inline]
    pub fn node_id(&self) -> u32 {
        self._driver.get_node_id()
    }

    #[inline]
    pub fn driver(&self) -> &ZephyrCrewDriver {
        &self._driver
    }

    pub fn tick_heartbeat(&mut self) -> u32 {
        self._heartbeat_ticks += 1;
        self._heartbeat_ticks
    }

    pub fn heartbeat_ticks(&self) -> u32 {
        self._heartbeat_ticks
    }

    pub fn send_message(&self, msg: &str) -> usize {
        self._driver.send(msg.as_bytes())
    }

    pub fn recv_message(&self, max_bytes: usize) -> String {
        let mut buf = vec![0u8; max_bytes];
        let n = self._driver.recv(&mut buf);
        buf.truncate(n);
        String::from_utf8_lossy(&buf).to_string()
    }
}
