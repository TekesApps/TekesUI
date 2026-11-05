// #41 manual outcome baseline. Standard NSApplication/NSTextField/default editor.
// No input event, editing callback, text-change, or input-context observation.
import AppKit
import CoreGraphics

precondition(CommandLine.arguments.count == 4, "usage: SOURCE CASE EXTERNAL_DISPLAY_ID")
let source = CommandLine.arguments[1]
let fixedCase = CommandLine.arguments[2]
precondition(["physical-keyboard", "cua-typeText", "paste"].contains(source))
let samples = ["ascii": "Ascii-A7", "cjk": "原生草稿保留",
               "mixed": "原生A草稿保留", "pinyin-first": "你好"]
let expected: String = {
    guard let value = samples[fixedCase] else { fatalError("invalid case") }
    return value
}()
let displayID: UInt32 = {
    guard let value = UInt32(CommandLine.arguments[3]) else { fatalError("invalid display ID") }
    return value
}()

final class StockBaseline: NSObject, NSApplicationDelegate, NSWindowDelegate {
    let window = NSWindow(contentRect: NSRect(x: 200, y: 300, width: 660, height: 180),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
    let field = NSTextField(frame: NSRect(x: 30, y: 70, width: 600, height: 50))
    var actions = 0
    var finished = false
    func emit(_ row: [String: Any]) {
        let data = try! JSONSerialization.data(withJSONObject: row, options: [.sortedKeys])
        FileHandle.standardError.write(Data(("UNICODE41_STOCK_MANUAL " + String(decoding: data, as: UTF8.self) + "\n").utf8))
    }
    func screenID(_ screen: NSScreen) -> UInt32? {
        (screen.deviceDescription[NSDeviceDescriptionKey(rawValue: "NSScreenNumber")] as? NSNumber)?.uint32Value
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        precondition(CGDisplayIsBuiltin(displayID) == 0 && CGDisplayIsInMirrorSet(displayID) == 0)
        let matches = NSScreen.screens.filter { screenID($0) == displayID }
        precondition(matches.count == 1, "requested external screen unavailable")
        let screen = matches[0], visible = screen.visibleFrame, size = window.frame.size
        let target = NSRect(x: visible.minX + 24, y: visible.maxY - size.height - 24,
                            width: size.width, height: size.height)
        precondition(visible.contains(target), "window does not fit external visible frame")
        window.setFrameOrigin(target.origin)
        field.isEditable = true; field.isSelectable = true; field.stringValue = ""
        field.placeholderString = "Unicode41 untouched stock baseline"
        field.target = self; field.action = #selector(submit)
        window.delegate = self
        window.title = "Unicode41 Untouched Stock · Manual outcome"
        window.contentView!.addSubview(field)
        let menu = NSMenu(), item = NSMenuItem(), diagnostic = NSMenu(title: "Diagnostic")
        let close = NSMenuItem(title: "Record result and close", action: #selector(finish), keyEquivalent: "")
        close.target = self; diagnostic.addItem(close); item.submenu = diagnostic; menu.addItem(item)
        let edit = NSMenu(title: "Edit")
        let paste = NSMenuItem(title: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        paste.target = nil
        paste.keyEquivalentModifierMask = .command
        edit.addItem(paste)
        let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
        editItem.submenu = edit
        menu.addItem(editItem)
        NSApp.mainMenu = menu
        window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        precondition(window.makeFirstResponder(field))
        // No currentEditor/inputContext/read of editing state before input.
        emit(["kind": "window-presented", "pid": ProcessInfo.processInfo.processIdentifier,
              "source": source, "case": fixedCase, "source_is_operator_declared": true,
              "requested_display_id": displayID, "screen_name": screen.localizedName,
              "window_frame": [window.frame.minX, window.frame.minY, window.frame.width, window.frame.height],
              "editing_state_observed": false, "callback_provenance": "unobserved"])
    }
    @objc func submit() { actions += 1 }
    @objc func finish() {
        guard !finished else { return }; finished = true
        // Only an explicit operator finish/close reads final state. No key hooks.
        let editor = field.currentEditor() as? NSTextView
        var row: [String: Any] = ["kind": "result", "pid": ProcessInfo.processInfo.processIdentifier,
            "source": source, "case": fixedCase, "expected": expected,
            "source_is_operator_declared": true, "actions": actions,
            "editor_present": editor != nil, "strict_callback_acceptance": NSNull(),
            "callback_provenance": "unobserved"]
        if let editor = editor {
            let selection = editor.selectedRange()
            row["value"] = editor.string
            row["selection"] = [selection.location, selection.location + selection.length]
            row["editor_class"] = NSStringFromClass(type(of: editor))
            if editor.hasMarkedText() {
                let marked = editor.markedRange()
                row["marked"] = [marked.location, marked.location + marked.length]
            } else { row["marked"] = NSNull() }
            row["outcome_matches_expected"] = editor.string == expected && !editor.hasMarkedText()
                && selection.location == expected.utf16.count && selection.length == 0 && actions == 0
        } else {
            row["outcome_matches_expected"] = NSNull() // Missing state is incomplete, never a fallback pass.
        }
        emit(row)
        NSApp.terminate(nil)
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool { finish(); return true }
}
let app = NSApplication.shared
let baseline = StockBaseline()
app.setActivationPolicy(.regular); app.delegate = baseline; app.run()
