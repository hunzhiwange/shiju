Create /app/start.py. Run as `python3 /app/start.py /app`; it must return successfully within 5 seconds and launch a real qemu-system-i386 process using software emulation (TCG), 16 MiB RAM and the supplied /app/boot.img raw floppy. This is a synthetic BIOS keyboard echo guest, not Windows. Do not modify boot.img or install packages.

Expose VNC display :1 at TCP 5901 and a QMP Unix monitor. QMP keyboard input must visibly change the running guest VGA framebuffer.

Use /app/monitor.sock for the QMP monitor, /app/vm.pid for the QEMU PID, /app/serial.log for COM1 output, and VNC 127.0.0.1:1. Keep QEMU in the background. No network access or external services are required. Do not fabricate serial output, screenshots, PID files, or monitor responses. Scope: /app and /tmp only; no privilege changes.
