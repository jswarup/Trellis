#include <zephyr/kernel.h>
#include <zephyr/device.h>
#include <zephyr/sys/sys_io.h>
#include <zephyr/sys/device_mmio.h>
#include <errno.h>

#include "drivers/crew.h"

#define CREW_RX_QUEUE_DEPTH  256
#define RX_WORKER_STACK_SIZE 1024
#define RX_WORKER_PRIO       K_PRIO_COOP(2)

struct crew_config {
    uintptr_t phys_base;
    size_t    size;
};

struct crew_data {
    mm_reg_t        virt_base;
    uint32_t        node_id;
    struct k_msgq   rx_msgq;
    char            rx_msgq_buffer[CREW_RX_QUEUE_DEPTH * sizeof(uint8_t)];
    struct k_thread rx_thread;
    K_KERNEL_STACK_MEMBER(rx_stack, RX_WORKER_STACK_SIZE);
};

static inline uint32_t reg_read(struct crew_data *data, uint32_t offset) {
    return sys_read32(data->virt_base + offset);
}

static inline void reg_write(struct crew_data *data, uint32_t offset, uint32_t val) {
    sys_write32(val, data->virt_base + offset);
}

// -------------------------------------------------------------------------------------------------
// Background cooperative worker thread that drains MMIO RX FIFO into k_msgq
// -------------------------------------------------------------------------------------------------
static void crew_rx_worker(void *p1, void *p2, void *p3) {
    ARG_UNUSED(p2);
    ARG_UNUSED(p3);

    const struct device *dev = (const struct device *)p1;
    struct crew_data *data = (struct crew_data *)dev->data;

    while (1) {
        uint32_t status = reg_read(data, CREW_REG_STATUS);

        if (status & CREW_STATUS_RX_READY) {
            uint8_t byte = (uint8_t)(reg_read(data, CREW_REG_RX_DATA) & 0xFF);
            k_msgq_put(&data->rx_msgq, &byte, K_NO_WAIT);
            /* Cooperatively yield to let waiting consumers process the received byte */
            k_yield();
        } else {
            /* Yield and sleep 1 tick when no data is pending to let other cooperative tasks run */
            k_yield();
            k_sleep(K_TICKS(1));
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Driver API implementation
// -------------------------------------------------------------------------------------------------
static int crew_impl_send(const struct device *dev, const uint8_t *data_buf, size_t len, k_timeout_t timeout) {
    struct crew_data *data = (struct crew_data *)dev->data;
    int64_t start_time = k_uptime_get();
    int64_t timeout_ms = K_TIMEOUT_EQ(timeout, K_FOREVER) ? -1 : k_ticks_to_ms_near64(timeout.ticks);

    for (size_t i = 0; i < len; ++i) {
        while (!(reg_read(data, CREW_REG_STATUS) & CREW_STATUS_TX_READY)) {
            if (timeout_ms >= 0 && (k_uptime_get() - start_time) >= timeout_ms) {
                return -ETIMEDOUT;
            }
            k_yield();
        }
        reg_write(data, CREW_REG_TX_DATA, (uint32_t)data_buf[i]);
    }
    return 0;
}

static int crew_impl_recv(const struct device *dev, uint8_t *buf, size_t max_len, size_t *out_len, k_timeout_t timeout) {
    struct crew_data *data = (struct crew_data *)dev->data;
    size_t count = 0;
    int64_t start_time = k_uptime_get();
    int64_t timeout_ms = K_TIMEOUT_EQ(timeout, K_FOREVER) ? -1 : k_ticks_to_ms_near64(timeout.ticks);

    while (count < max_len) {
        uint8_t byte = 0;
        k_timeout_t remaining = K_FOREVER;

        if (timeout_ms >= 0) {
            int64_t elapsed = k_uptime_get() - start_time;
            if (elapsed >= timeout_ms) {
                break;
            }
            remaining = K_MSEC(timeout_ms - elapsed);
        }

        int ret = k_msgq_get(&data->rx_msgq, &byte, remaining);
        if (ret != 0) {
            break;
        }

        buf[count++] = byte;
        if (byte == '\n' || byte == '\0') {
            break;
        }
    }

    if (out_len) {
        *out_len = count;
    }
    return (count > 0) ? 0 : -ETIMEDOUT;
}

static uint32_t crew_impl_get_node_id(const struct device *dev) {
    struct crew_data *data = (struct crew_data *)dev->data;
    return data->node_id;
}

static uint32_t crew_impl_get_status(const struct device *dev) {
    struct crew_data *data = (struct crew_data *)dev->data;
    return reg_read(data, CREW_REG_STATUS);
}

static const struct crew_driver_api crew_driver_api_inst = {
    .send = crew_impl_send,
    .recv = crew_impl_recv,
    .get_node_id = crew_impl_get_node_id,
    .get_status = crew_impl_get_status,
};

// -------------------------------------------------------------------------------------------------
// Driver registration and POST_KERNEL initialization
// -------------------------------------------------------------------------------------------------
static int crew_driver_init(const struct device *dev) {
    const struct crew_config *cfg = (const struct crew_config *)dev->config;
    struct crew_data *data = (struct crew_data *)dev->data;

    device_map(&data->virt_base, cfg->phys_base, cfg->size, K_MEM_CACHE_NONE);

    k_msgq_init(&data->rx_msgq, data->rx_msgq_buffer, sizeof(uint8_t), CREW_RX_QUEUE_DEPTH);

    data->node_id = reg_read(data, CREW_REG_NODE_ID);

    k_thread_create(
        &data->rx_thread,
        data->rx_stack,
        K_KERNEL_STACK_SIZEOF(data->rx_stack),
        crew_rx_worker,
        (void *)dev, NULL, NULL,
        RX_WORKER_PRIO, 0, K_NO_WAIT
    );
    k_thread_name_set(&data->rx_thread, "crew_rx");

    return 0;
}

static const struct crew_config crew_cfg_0 = {
    .phys_base = 0x50000000U,
    .size      = 0x1000U,
};

static struct crew_data crew_data_0;

DEVICE_DEFINE(
    crew_cosim,
    "CREW_COSIM",
    crew_driver_init,
    NULL,
    &crew_data_0,
    &crew_cfg_0,
    POST_KERNEL,
    CONFIG_KERNEL_INIT_PRIORITY_DEVICE,
    &crew_driver_api_inst
);

