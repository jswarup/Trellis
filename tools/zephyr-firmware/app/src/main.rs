// tools/zephyr-firmware/app/src/main.rs
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

//-------------------------------------------------------------------------------------------------
// Memory-mapped Peripheral Addresses

const UART1_BASE: usize = 0xF0300020;
// NS16550 with wideRegisters: true (4-byte register stride)
const UART1_THR: *mut u32 = (UART1_BASE + 0x00) as *mut u32;
const UART1_LSR: *const u32 = (UART1_BASE + 0x14) as *const u32;
const LSR_THRE: u32 = 0x20; // Transmitter Holding Register Empty

// Crew MMIO Peripheral Window
const CREW_BASE: usize = 0x50000000;
const REG_NODE_ID: *const u32 = (CREW_BASE + 0x00) as *const u32;
const REG_STATUS: *const u32 = (CREW_BASE + 0x04) as *const u32;
const REG_TX_DATA: *mut u32 = (CREW_BASE + 0x08) as *mut u32;
const REG_RX_DATA: *const u32 = (CREW_BASE + 0x0C) as *const u32;
const REG_RX_COUNT: *const u32 = (CREW_BASE + 0x10) as *const u32;

const STATUS_TX_READY: u32 = 1;
const STATUS_RX_READY: u32 = 2;
const STATUS_PEER_UP: u32 = 4;

//-------------------------------------------------------------------------------------------------
// Low-level UART Driver

pub struct Uart;

impl Uart {
    pub fn putc(c: u8) {
        unsafe {
            // Wait until transmit holding register is empty
            while (core::ptr::read_volatile(UART1_LSR) & LSR_THRE) == 0 {}
            core::ptr::write_volatile(UART1_THR, c as u32);
        }
    }

    pub fn puts(s: &str) {
        for b in s.bytes() {
            if b == b'\n' {
                Self::putc(b'\r');
            }
            Self::putc(b);
        }
    }

    pub fn print_u32(mut val: u32) {
        if val == 0 {
            Self::putc(b'0');
            return;
        }
        let mut buf = [0u8; 10];
        let mut i = 0;
        while val > 0 {
            buf[i] = b'0' + (val % 10) as u8;
            val /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            Self::putc(buf[i]);
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Entry Point Assembly Stub

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.init")]
pub unsafe extern "C" fn _start() -> ! {
    asm!(
        // Disable interrupts
        "csrw mie, zero",
        // Setup stack pointer
        "la sp, _stack_top",
        // Clear .bss
        "la t0, __bss_start",
        "la t1, __bss_end",
        "1:",
        "bge t0, t1, 2f",
        "sw zero, 0(t0)",
        "addi t0, t0, 4",
        "j 1b",
        "2:",
        // Call main
        "call rust_main",
        // Loop forever if main returns
        "3:",
        "wfi",
        "j 3b",
        options(noreturn)
    );
}

//-------------------------------------------------------------------------------------------------
// Guest Main Application

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() {
    Uart::puts("\n=======================================================\n");
    Uart::puts("[Zephyr/AE350 Guest] Booting Andes N25 core in Renode\n");
    Uart::puts("=======================================================\n");

    // 1. Read Node ID
    let node_id = unsafe { core::ptr::read_volatile(REG_NODE_ID) };
    Uart::puts("[Zephyr/AE350 Guest] Crew Node ID: ");
    Uart::print_u32(node_id);
    Uart::puts("\n");

    // 2. Poll Status
    let status = unsafe { core::ptr::read_volatile(REG_STATUS) };
    Uart::puts("[Zephyr/AE350 Guest] Initial Crew Status: ");
    Uart::print_u32(status);
    Uart::puts("\n");

    if (status & STATUS_PEER_UP) != 0 {
        Uart::puts("[Zephyr/AE350 Guest] Peer node is detected ONLINE\n");
    }

    // 3. Send message across Crew MMIO
    let message = "Hello from custom Renode Zephyr Guest!\n";
    Uart::puts("[Zephyr/AE350 Guest] Transmitting message to peer...\n");

    for b in message.bytes() {
        // Wait for TX ready
        while (unsafe { core::ptr::read_volatile(REG_STATUS) } & STATUS_TX_READY) == 0 {}
        unsafe {
            core::ptr::write_volatile(REG_TX_DATA, b as u32);
        }
    }
    Uart::puts("[Zephyr/AE350 Guest] Message transmission complete.\n");

    // 4. Poll for incoming response bytes (up to 1000 attempts)
    Uart::puts("[Zephyr/AE350 Guest] Checking for peer response...\n");
    let mut attempts = 0;
    while attempts < 1000 {
        let cur_status = unsafe { core::ptr::read_volatile(REG_STATUS) };
        if (cur_status & STATUS_RX_READY) != 0 {
            let rx_count = unsafe { core::ptr::read_volatile(REG_RX_COUNT) };
            Uart::puts("[Zephyr/AE350 Guest] Received response bytes: ");
            Uart::print_u32(rx_count);
            Uart::puts(" bytes\n[Peer Reply] ");

            while (unsafe { core::ptr::read_volatile(REG_STATUS) } & STATUS_RX_READY) != 0 {
                let rx_byte = (unsafe { core::ptr::read_volatile(REG_RX_DATA) } & 0xFF) as u8;
                Uart::putc(rx_byte);
            }
            Uart::puts("\n");
            break;
        }
        attempts += 1;
    }

    Uart::puts("[Zephyr/AE350 Guest] Application completed successfully.\n");
}

//-------------------------------------------------------------------------------------------------
// Panic Handler

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    Uart::puts("\n[Zephyr/AE350 Guest] PANIC occurred!\n");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
