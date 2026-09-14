#pragma once

#include <zephyr/device.h>
#include <zephyr/kernel.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* MMIO register offsets */
#define CREW_REG_NODE_ID    0x00U
#define CREW_REG_STATUS     0x04U
#define CREW_REG_TX_DATA    0x08U
#define CREW_REG_RX_DATA    0x0CU
#define CREW_REG_RX_COUNT   0x10U

/* Status register bit flags */
#define CREW_STATUS_TX_READY (1U << 0)
#define CREW_STATUS_RX_READY (1U << 1)
#define CREW_STATUS_PEER_UP  (1U << 2)

/**
 * @brief Driver API for Crew co-simulation virtual peripheral.
 */
__subsystem struct crew_driver_api {
    int (*send)(const struct device *dev, const uint8_t *data, size_t len, k_timeout_t timeout);
    int (*recv)(const struct device *dev, uint8_t *buf, size_t max_len, size_t *out_len, k_timeout_t timeout);
    uint32_t (*get_node_id)(const struct device *dev);
    uint32_t (*get_status)(const struct device *dev);
};

/**
 * @brief Transmit bytes to peer virtual machine with cooperative flow control.
 *
 * @param dev Pointer to device structure.
 * @param data Buffer of bytes to send.
 * @param len Number of bytes to send.
 * @param timeout Maximum duration to wait for transmission readiness.
 * @return 0 on success, or negative error code on timeout/failure.
 */
static inline int crew_send(const struct device *dev, const uint8_t *data, size_t len, k_timeout_t timeout) {
    const struct crew_driver_api *api = (const struct crew_driver_api *)dev->api;
    return api->send(dev, data, len, timeout);
}

/**
 * @brief Receive bytes from peer virtual machine using buffered message queue.
 *
 * Drains incoming bytes from the driver's internal queue until max_len is reached,
 * a newline delimiter ('\n' or '\0') is encountered, or timeout expires.
 *
 * @param dev Pointer to device structure.
 * @param buf Destination buffer.
 * @param max_len Maximum bytes to read into buf.
 * @param out_len Output pointer receiving the actual number of bytes read.
 * @param timeout Maximum duration to wait for incoming bytes.
 * @return 0 on success, or negative error code on timeout.
 */
static inline int crew_recv(const struct device *dev, uint8_t *buf, size_t max_len, size_t *out_len, k_timeout_t timeout) {
    const struct crew_driver_api *api = (const struct crew_driver_api *)dev->api;
    return api->recv(dev, buf, max_len, out_len, timeout);
}

/**
 * @brief Retrieve the local node ID (0 or 1).
 */
static inline uint32_t crew_get_node_id(const struct device *dev) {
    const struct crew_driver_api *api = (const struct crew_driver_api *)dev->api;
    return api->get_node_id(dev);
}

/**
 * @brief Retrieve current status register value.
 */
static inline uint32_t crew_get_status(const struct device *dev) {
    const struct crew_driver_api *api = (const struct crew_driver_api *)dev->api;
    return api->get_status(dev);
}

#ifdef __cplusplus
}
#endif

