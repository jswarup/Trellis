# AE350/N25 Zephyr Firmware

The Renode AE350/N25 reference model is not an upstream Zephyr board. This directory is reserved for the Andes vendor board support package and the Segue Crew Zephyr driver.

Expected local layout:

```text
tools/
  renode/
    bin/Renode.exe
    platforms/ae350-n25-crew.repl
    scripts/run-ae350-n25-zephyr.resc
  zephyr-firmware/
    zephyr/                 local Zephyr checkout or vendor SDK
    app/                    Segue firmware application
out/
  zephyr/ae350-n25/zephyr.elf
```

The target must provide the AE350/N25 CPU configuration, startup code, linker script, NS16550 UART mapping at `0xf0300020`, PLIC mapping at `0xe4000000`, and a Crew device mapping at `0x50000000`.

Build the vendor-supported board into `out/zephyr/ae350-n25/zephyr.elf`, then run:

```powershell
tools/renode/bin/Renode.exe --console tools/renode/scripts/run-ae350-n25-zephyr.resc
```

The platform file reserves the Crew MMIO range. Connecting that range to `CrewHub` requires the Renode socket co-simulation extension; add its extension assembly and endpoint settings once the extension version is selected.
