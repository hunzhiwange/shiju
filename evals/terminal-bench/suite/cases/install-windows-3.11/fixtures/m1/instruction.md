Create /app/start.py. Run as `python3 /app/start.py /app`; it must return successfully within 5 seconds and launch a real qemu-system-i386 process using software emulation (TCG), 16 MiB RAM and the supplied /app/boot.img raw floppy. This is a synthetic BIOS keyboard echo guest, not Windows. Do not modify boot.img or install packages.

Preserve the base floppy image using QEMU snapshot mode. The live QEMU drive must use boot.img with snapshot=on.

Use /app/monitor.sock for the QMP monitor, /app/vm.pid for the QEMU PID, /app/serial.log for COM1 output, and VNC 127.0.0.1:1. Keep QEMU in the background. No network access or external services are required. Do not fabricate serial output, screenshots, PID files, or monitor responses. Scope: /app and /tmp only; no privilege changes.
