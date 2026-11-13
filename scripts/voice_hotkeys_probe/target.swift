import AppKit
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let window = NSWindow(contentRect: NSRect(x: 250, y: 220, width: 480, height: 180), styleMask: [.titled, .closable], backing: .buffered, defer: false)
window.title = "ShiJu hotkey isolated input target"
window.contentView = NSTextField(labelWithString: "Temporary test window. No user files or credentials.")
window.makeKeyAndOrderFront(nil)
app.activate(ignoringOtherApps: true)
app.run()
