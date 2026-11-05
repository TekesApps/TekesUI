// #41 outcome-only control. AppKit owns the default NSTextField field editor.
// No field-editor override, inputContext read, warmup key or text compensation.
import AppKit
import CoreGraphics

precondition(CommandLine.arguments.count == 4, "usage: SOURCE CASE EXTERNAL_DISPLAY_ID")
let source = CommandLine.arguments[1], fixedCase = CommandLine.arguments[2]
precondition(source == "pid-hardware-key" && fixedCase == "pinyin-first")
let expected = "你好"
let requestedDisplayID: CGDirectDisplayID = {
    guard let value = UInt32(CommandLine.arguments[3]) else { fatalError("invalid display ID") }
    return value
}()

@objc(Unicode41StockApplication)
final class StockApplication: NSApplication {
    weak var probe: StockProbe?
    override func sendEvent(_ event: NSEvent) {
        guard let probe = probe, probe.ready, event.type == .keyDown,
              keyWindow === probe.window else {
            super.sendEvent(event)
            return
        }
        let id = probe.beginDispatch(event) // Event metadata only; no editor access.
        super.sendEvent(event)              // Default AppKit dispatch exactly once.
        probe.endDispatch(id)
    }
}

final class StockProbe: NSObject, NSApplicationDelegate, NSWindowDelegate, NSTextFieldDelegate {
    let window = NSWindow(contentRect: NSRect(x: 200, y: 300, width: 660, height: 180),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
    let field = NSTextField(frame: NSRect(x: 30, y: 70, width: 600, height: 50))
    var ready = false
    var finished = false
    var seq = 0
    var dispatchCount = 0
    var completedDispatches = 0
    var actions = 0

    // Intentionally no windowWillReturnFieldEditor and no custom NSTextView.
    func emit(_ kind: String, _ detail: [String: Any], snapshot: Bool) {
        seq += 1
        var row: [String: Any] = ["control": "appkit-stock", "oracle_mode": "stock-outcome-only",
            "callback_provenance": "unobserved", "span_scope": "NSApplication.sendEvent",
            "seq": seq, "kind": kind, "detail": detail,
            "pid": ProcessInfo.processInfo.processIdentifier, "requested_display_id": requestedDisplayID,
            "declared_source": source, "case": fixedCase,
            "recorded_uptime_seconds": ProcessInfo.processInfo.systemUptime,
            "physical_ime": "not_run", "state_observed": snapshot]
        if snapshot {
            let editor = field.currentEditor() as? NSTextView
            row["editor_present"] = editor != nil
            if let editor = editor {
                let selection = editor.selectedRange()
                row["value"] = editor.string
                row["selection"] = [selection.location, selection.location + selection.length]
                if editor.hasMarkedText() {
                    let marked = editor.markedRange()
                    row["marked"] = [marked.location, marked.location + marked.length]
                } else { row["marked"] = NSNull() }
                row["editor_class"] = NSStringFromClass(type(of: editor))
                row["focused"] = window.firstResponder === editor
            }
            row["active"] = NSApp.isActive && window.isKeyWindow
            row["actions"] = actions
        }
        let data = try! JSONSerialization.data(withJSONObject: row, options: [.sortedKeys])
        FileHandle.standardError.write(Data(("UNICODE41 " + String(decoding: data, as: UTF8.self) + "\n").utf8))
    }
    func beginDispatch(_ event: NSEvent) -> Int {
        dispatchCount += 1
        emit("key_context", ["phase": "start", "context_id": dispatchCount,
            "native_code": event.keyCode, "characters": event.characters ?? "",
            "characters_ignoring_modifiers": event.charactersIgnoringModifiers ?? "",
            "event_timestamp_seconds": event.timestamp], snapshot: false)
        return dispatchCount
    }
    func endDispatch(_ id: Int) {
        emit("key_context", ["phase": "end", "context_id": id,
            "disposition": "application-dispatch-returned-callbacks-unobserved",
            "input_context_handled": NSNull()], snapshot: true)
        completedDispatches += 1
        if completedDispatches == 6 {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { self.finish() }
        }
    }
    func displayID(_ screen: NSScreen) -> CGDirectDisplayID? {
        (screen.deviceDescription[NSDeviceDescriptionKey(rawValue: "NSScreenNumber")] as? NSNumber)?.uint32Value
    }
    func placeOnExternalScreen() {
        precondition(CGDisplayIsBuiltin(requestedDisplayID) == 0 && CGDisplayIsInMirrorSet(requestedDisplayID) == 0,
                     "stock test requires a non-mirrored external display")
        let matches = NSScreen.screens.filter { displayID($0) == requestedDisplayID }
        precondition(matches.count == 1, "requested external NSScreen is unavailable")
        let screen = matches[0], visible = screen.visibleFrame, size = window.frame.size
        let target = NSRect(x: visible.minX + 24, y: visible.maxY - size.height - 24,
                            width: size.width, height: size.height)
        precondition(visible.contains(target), "stock window does not fit external visible frame")
        window.setFrameOrigin(target.origin) // Only this still-hidden diagnostic window.
        precondition(window.screen.flatMap { displayID($0) } == requestedDisplayID && visible.contains(window.frame),
                     "stock window was not placed wholly on the requested external screen")
        emit("placement", ["screen_name": screen.localizedName,
            "visible_frame": [visible.minX, visible.minY, visible.width, visible.height],
            "window_frame": [window.frame.minX, window.frame.minY, window.frame.width, window.frame.height]], snapshot: false)
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        field.isEditable = true; field.isSelectable = true; field.stringValue = ""
        field.placeholderString = "Unicode41 stock input"; field.delegate = self
        field.target = self; field.action = #selector(submit)
        window.delegate = self; window.title = "Unicode41 Stock AppKit · Pinyin outcome"
        window.contentView!.addSubview(field)
        let menu = NSMenu(), item = NSMenuItem(), diagnostic = NSMenu(title: "Diagnostic")
        let close = NSMenuItem(title: "Record outcome and close", action: #selector(finish), keyEquivalent: "")
        close.target = self; diagnostic.addItem(close); item.submenu = diagnostic; menu.addItem(item)
        NSApp.mainMenu = menu
        guard let app = NSApp as? StockApplication else { fatalError("stock application dispatch observer missing") }
        app.probe = self
        placeOnExternalScreen()
        window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        precondition(window.makeFirstResponder(field))
        publishReady()
    }
    func applicationDidBecomeActive(_ notification: Notification) { publishReady() }
    func windowDidBecomeKey(_ notification: Notification) { publishReady() }
    func publishReady() {
        guard !ready, NSApp.isActive, window.isKeyWindow,
              let editor = field.currentEditor() as? NSTextView,
              window.firstResponder === editor else { return }
        precondition(window.screen.flatMap { displayID($0) } == requestedDisplayID,
                     "stock input must remain on the requested external screen at ready")
        ready = true
        emit("ready", ["expected": expected, "source_is_operator_declared": true], snapshot: true)
    }
    @objc func submit() { actions += 1; emit("submit", [:], snapshot: true) }
    @objc func finish() {
        guard !finished else { return }; finished = true
        let editor = field.currentEditor() as? NSTextView
        emit("result", ["expected": expected, "matches": editor?.string == expected,
            "no_marked": editor.map { !$0.hasMarkedText() } ?? false,
            "no_submit": actions == 0, "ready": ready,
            "strict_callback_acceptance": "not_observed"], snapshot: true)
        NSApp.terminate(nil)
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool { finish(); return true }
}
let app = StockApplication.shared
let probe = StockProbe()
app.setActivationPolicy(.regular); app.delegate = probe; app.run()
