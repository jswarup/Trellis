#include <zephyr/kernel.h>
#include <zephyr/sys/printk.h>
#include <zephyr/sys/sys_io.h>
#include <zephyr/sys/device_mmio.h>
#include <string.h>

#define CREW_PHYS_BASE      0x50000000U
#define CREW_SIZE           0x1000U

#define REG_NODE_ID         0x00U
#define REG_STATUS          0x04U
#define REG_TX_DATA         0x08U
#define REG_RX_DATA         0x0CU
#define REG_RX_COUNT        0x10U

#define STATUS_TX_READY     (1U << 0)
#define STATUS_RX_READY     (1U << 1)
#define STATUS_PEER_UP      (1U << 2)

static mm_reg_t g_crew_base = 0;

static inline uint32_t crew_read(uint32_t offset) {
    return sys_read32(g_crew_base + offset);
}

static inline void crew_write(uint32_t offset, uint32_t val) {
    sys_write32(val, g_crew_base + offset);
}

static void crew_send_string(const char *msg) {
    while (*msg != '\0') {
        while (!(crew_read(REG_STATUS) & STATUS_TX_READY)) {
            k_busy_wait(100);
        }
        crew_write(REG_TX_DATA, (uint32_t)(uint8_t)(*msg));
        msg++;
    }
    // Send newline delimiter
    while (!(crew_read(REG_STATUS) & STATUS_TX_READY)) {
        k_busy_wait(100);
    }
    crew_write(REG_TX_DATA, (uint32_t)'\n');
}

static int crew_recv_string(char *buf, size_t max_len, uint32_t timeout_ms) {
    size_t idx = 0;
    uint32_t elapsed = 0;

    while (idx + 1 < max_len) {
        uint32_t status = crew_read(REG_STATUS);
        if (status & STATUS_RX_READY) {
            char c = (char)(crew_read(REG_RX_DATA) & 0xFF);
            if (c == '\n' || c == '\0') {
                break;
            }
            buf[idx++] = c;
        } else {
            k_busy_wait(1000); // 1ms
            elapsed++;
            if (elapsed >= timeout_ms) {
                buf[idx] = '\0';
                return -1; // Timed out
            }
        }
    }
    buf[idx] = '\0';
    return 0;
}

int main(void) {
    device_map(&g_crew_base, CREW_PHYS_BASE, CREW_SIZE, K_MEM_CACHE_NONE);

    uint32_t node_id = crew_read(REG_NODE_ID);
    char rx_buffer[128];

    printk("\n========================================\n");
    printk("[VM%u] Hello World from Zephyr VM%u!\n", node_id, node_id);
    printk("[VM%u] Crew CoSim mapped: phys 0x%08X -> virt 0x%lx\n",
           node_id, CREW_PHYS_BASE, (unsigned long)g_crew_base);
    printk("========================================\n");

    if (node_id == 0) {
        printk("[VM0] Role: SENDER. Waiting 20ms for VM1 to be ready...\n");
        k_msleep(20);

        printk("[VM0] Sending message: \"Hello World from Zephyr VM0!\"\n");
        crew_send_string("Hello World from Zephyr VM0!");

        printk("[VM0] Waiting for reply from VM1...\n");
        if (crew_recv_string(rx_buffer, sizeof(rx_buffer), 5000) == 0) {
            printk("[VM0] Received reply: \"%s\"\n", rx_buffer);
            printk("[VM0] SUCCESS: Dual-VM hello-world exchange complete!\n");
        } else {
            printk("[VM0] ERROR: Timed out waiting for reply from VM1!\n");
        }
    } else {
        printk("[VM1] Role: RECEIVER & ECHO. Waiting for message from VM0...\n");
        if (crew_recv_string(rx_buffer, sizeof(rx_buffer), 5000) == 0) {
            printk("[VM1] Received message: \"%s\"\n", rx_buffer);
            printk("[VM1] Sending reply: \"Hello World back from Zephyr VM1!\"\n");
            crew_send_string("Hello World back from Zephyr VM1!");
            printk("[VM1] SUCCESS: Reply sent to VM0!\n");
        } else {
            printk("[VM1] ERROR: Timed out waiting for message from VM0!\n");
        }
    }

    printk("[VM%u] Execution finished. Entering sleep loop.\n", node_id);
    while (1) {
        k_msleep(1000);
    }
    return 0;
}
