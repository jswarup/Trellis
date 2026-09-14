#include <zephyr/kernel.h>
#include <zephyr/sys/printk.h>
#include <zephyr/device.h>
#include <string.h>

#include "drivers/crew.h"

#define HEARTBEAT_STACK_SIZE 1024
#define HEARTBEAT_PRIO       K_PRIO_COOP(5)

K_THREAD_STACK_DEFINE(g_heartbeat_stack, HEARTBEAT_STACK_SIZE);
static struct k_thread g_heartbeat_thread;

// -------------------------------------------------------------------------------------------------
// Concurrent cooperative heartbeat task
// Demonstrates that cooperative multi-tasking runs cleanly alongside the Crew driver
// -------------------------------------------------------------------------------------------------
static void heartbeat_worker(void *p1, void *p2, void *p3) {
    ARG_UNUSED(p2);
    ARG_UNUSED(p3);

    uint32_t nodeId = POINTER_TO_UINT(p1);
    uint32_t counter = 0;

    while (1) {
        k_msleep(500);
        printk("[VM%u-Heartbeat] Cooperative tick #%u\n", nodeId, ++counter);
    }
}

// -------------------------------------------------------------------------------------------------
// Main application entry point
// -------------------------------------------------------------------------------------------------
int main(void) {
    const struct device *const crewDev = device_get_binding("CREW_COSIM");

    if (!crewDev || !device_is_ready(crewDev)) {
        printk("[ERROR] Crew co-simulation device driver not ready!\n");
        return -1;
    }

    uint32_t nodeId = crew_get_node_id(crewDev);
    char rxBuffer[128];
    size_t rxLen = 0;

    printk("\n========================================\n");
    printk("[VM%u] Hello World from Zephyr VM%u!\n", nodeId, nodeId);
    printk("[VM%u] Driver bound: %s\n", nodeId, crewDev->name);
    printk("========================================\n");

    /* Launch concurrent cooperative heartbeat task */
    k_thread_create(
        &g_heartbeat_thread,
        g_heartbeat_stack,
        K_THREAD_STACK_SIZEOF(g_heartbeat_stack),
        heartbeat_worker,
        UINT_TO_POINTER(nodeId), NULL, NULL,
        HEARTBEAT_PRIO, 0, K_NO_WAIT
    );
    k_thread_name_set(&g_heartbeat_thread, "heartbeat");

    if (nodeId == 0) {
        printk("[VM0] Role: SENDER. Waiting 20ms for VM1 to be ready...\n");
        k_msleep(20);

        const char *msg0 = "Hello World from Zephyr VM0!\n";
        printk("[VM0] Sending message via driver: \"Hello World from Zephyr VM0!\"\n");
        int err = crew_send(crewDev, (const uint8_t *)msg0, strlen(msg0), K_FOREVER);
        if (err != 0) {
            printk("[VM0] ERROR: Failed to send message (err=%d)\n", err);
        }

        printk("[VM0] Waiting for reply from VM1 via cooperative driver...\n");
        err = crew_recv(crewDev, (uint8_t *)rxBuffer, sizeof(rxBuffer) - 1, &rxLen, K_MSEC(5000));
        if (err == 0) {
            rxBuffer[rxLen] = '\0';
            // Strip trailing newline if present for log print
            if (rxLen > 0 && rxBuffer[rxLen - 1] == '\n') {
                rxBuffer[rxLen - 1] = '\0';
            }
            printk("[VM0] Received reply: \"%s\"\n", rxBuffer);
            printk("[VM0] SUCCESS: Dual-VM hello-world exchange complete!\n");
        } else {
            printk("[VM0] ERROR: Timed out waiting for reply from VM1! (err=%d)\n", err);
        }
    } else {
        printk("[VM1] Role: RECEIVER & ECHO. Waiting for message from VM0 via cooperative driver...\n");
        int err = crew_recv(crewDev, (uint8_t *)rxBuffer, sizeof(rxBuffer) - 1, &rxLen, K_MSEC(5000));
        if (err == 0) {
            rxBuffer[rxLen] = '\0';
            if (rxLen > 0 && rxBuffer[rxLen - 1] == '\n') {
                rxBuffer[rxLen - 1] = '\0';
            }
            printk("[VM1] Received message: \"%s\"\n", rxBuffer);

            const char *msg1 = "Hello World back from Zephyr VM1!\n";
            printk("[VM1] Sending reply via driver: \"Hello World back from Zephyr VM1!\"\n");
            crew_send(crewDev, (const uint8_t *)msg1, strlen(msg1), K_FOREVER);
            printk("[VM1] SUCCESS: Reply sent to VM0!\n");
        } else {
            printk("[VM1] ERROR: Timed out waiting for message from VM0! (err=%d)\n", err);
        }
    }

    printk("[VM%u] Protocol exchange finished. Continuing cooperative idle loop.\n", nodeId);
    while (1) {
        k_msleep(1000);
    }

    return 0;
}
