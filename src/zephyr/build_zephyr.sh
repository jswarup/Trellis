#!/bin/bash
set -e
source /home/jyoti/zephyrproject/.venv/bin/activate
export ZEPHYR_BASE=/home/jyoti/zephyrproject/zephyr
export ZEPHYR_SDK_INSTALL_DIR=/home/jyoti/zephyr-sdk-1.0.1
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "[Zephyr-VM] Building Zephyr firmware for acrn (x86_64)..."
west build -p always -b acrn -s . -d build

echo "[Zephyr-VM] Copying ELF to bin..."
mkdir -p "$SCRIPT_DIR/bin"
cp build/zephyr/zephyr.elf "$SCRIPT_DIR/bin/tweety_zephyr.elf"
echo "[Zephyr-VM] Zephyr build complete: $SCRIPT_DIR/bin/tweety_zephyr.elf"
ls -lh "$SCRIPT_DIR/bin/tweety_zephyr.elf"
