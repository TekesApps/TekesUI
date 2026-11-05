// Independent standard NSTextField oracle. Diagnostic only; editing calls super.
import AppKit

let arguments = CommandLine.arguments
precondition(arguments.count == 3, "usage: AppKitBaseline SOURCE CASE")
let source = arguments[1], fixedCase = arguments[2]
precondition(["cua-typeText", "pid-hardware-key", "local-key-characters", "paste", "physical-keyboard"].contains(source))
let samples = ["ascii": "Ascii-A7", "cjk": "原生草稿保留", "mixed": "原生A草稿保留", "pinyin-first": "你好"]
let expected: String = {
    guard let value = samples[fixedCase] else { fatalError("unknown fixed case") }
    return value
}()
precondition(source != "local-key-characters" || fixedCase != "pinyin-first")
var sequence = 0
var nextKey = 0
var nativeKeyEnds = 0
var keyStack: [Int] = []
var callbackCounts: [Int: Int] = [:]
var submitted = 0
var recordedResult = false

func range(_ r: NSRange) -> Any { r.location == NSNotFound ? NSNull() : [r.location, r.location + r.length] }
func plain(_ value: Any) -> String { (value as? NSAttributedString)?.string ?? (value as? String) ?? String(describing: value) }
func record(_ kind: String, _ detail: [String: Any] = [:], _ editor: NSTextView? = nil) {
    sequence += 1
    let e = editor ?? fixture.editor
    let row: [String: Any] = ["control": "appkit", "seq": sequence, "kind": kind,
        "pid": ProcessInfo.processInfo.processIdentifier, "declared_source": source, "case": fixedCase,
        "recorded_uptime_seconds": ProcessInfo.processInfo.systemUptime,
        "detail": detail, "value": e.string, "selection": range(e.selectedRange()),
        "marked": e.hasMarkedText() ? range(e.markedRange()) : NSNull(), "active": NSApp.isActive && fixture.window.isKeyWindow,
        "focused": fixture.window.firstResponder === e, "actions": submitted, "physical_ime": "not_run"]
    let data = try! JSONSerialization.data(withJSONObject: row, options: [.sortedKeys])
    FileHandle.standardError.write(Data(("UNICODE41 " + String(decoding: data, as: UTF8.self) + "\n").utf8))
}
func callback(_ kind: String, _ detail: [String: Any], _ editor: NSTextView) {
    if let key = keyStack.last { callbackCounts[key, default: 0] += 1 }
    var d = detail; d["context_id"] = keyStack.last.map { $0 as Any } ?? NSNull()
    record(kind, d, editor)
}
final class ObservedFieldEditor: NSTextView {
    override func keyDown(with event: NSEvent) {
        nextKey += 1; let key = nextKey; keyStack.append(key); callbackCounts[key] = 0
        record("key_context", ["phase": "start", "context_id": key, "native_code": event.keyCode,
            "characters": event.characters ?? "", "characters_ignoring_modifiers": event.charactersIgnoringModifiers ?? "",
            "event_timestamp_seconds": event.timestamp, "composing": hasMarkedText(), "input_context": inputContext.map { String(describing: Unmanaged.passUnretained($0).toOpaque()) } ?? "missing"], self)
        super.keyDown(with: event)
        record("key_context", ["phase": "end", "context_id": key,
            "callback_count": callbackCounts.removeValue(forKey: key) ?? 0,
            "input_context_handled": NSNull(), "disposition": "standard-superclass-return-not-exposed"], self)
        precondition(keyStack.removeLast() == key)
        nativeKeyEnds += 1
        if source == "pid-hardware-key" && nativeKeyEnds == 6 {
            // Post-final-key cleanup only; no readiness delay, priming or retry.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { fixture.finish() }
        }
    }
    override func insertText(_ value: Any, replacementRange: NSRange) {
        callback("commit", ["text": plain(value), "replacement": range(replacementRange)], self)
        super.insertText(value, replacementRange: replacementRange)
        record("editor", [:], self)
    }
    override func setMarkedText(_ value: Any, selectedRange: NSRange, replacementRange: NSRange) {
        callback("preedit", ["text": plain(value), "selection": range(selectedRange), "replacement": range(replacementRange)], self)
        super.setMarkedText(value, selectedRange: selectedRange, replacementRange: replacementRange)
        record("editor", [:], self)
    }
    override func unmarkText() {
        callback("unmark", [:], self); super.unmarkText(); record("editor", [:], self)
    }
    override func doCommand(by selector: Selector) {
        callback("command", ["selector": NSStringFromSelector(selector)], self)
        super.doCommand(by: selector)
    }
}
final class Delegate: NSObject, NSApplicationDelegate, NSWindowDelegate, NSTextFieldDelegate {
    let editor = ObservedFieldEditor()
    let window = NSWindow(contentRect: NSRect(x: 200, y: 300, width: 660, height: 180), styleMask: [.titled, .closable], backing: .buffered, defer: false)
    var ready = false
    let field = NSTextField(frame: NSRect(x: 30, y: 70, width: 600, height: 50))
    func windowWillReturnFieldEditor(_ sender: NSWindow, to client: Any?) -> Any? { editor }
    func applicationDidFinishLaunching(_ notification: Notification) {
        editor.isFieldEditor = true
        field.isEditable = true; field.isSelectable = true; field.stringValue = ""
        field.placeholderString = "Unicode41 input"; field.delegate = self
        field.target = self; field.action = #selector(submit)
        window.delegate = self; window.title = "Unicode41 AppKit · \(source) · \(fixedCase)"
        window.contentView!.addSubview(field)
        let menu = NSMenu(); let item = NSMenuItem(); let diagnostic = NSMenu(title: "Diagnostic")
        let finishItem = NSMenuItem(title: "Record result and close", action: #selector(finish), keyEquivalent: "")
        finishItem.target = self; diagnostic.addItem(finishItem); item.submenu = diagnostic; menu.addItem(item); NSApp.mainMenu = menu
        window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        precondition(window.makeFirstResponder(field))
        publishReady()
    }
    func applicationDidBecomeActive(_ notification: Notification) { publishReady() }
    func windowDidBecomeKey(_ notification: Notification) { publishReady() }
    func publishReady() {
        guard !ready, NSApp.isActive, window.isKeyWindow, window.firstResponder === editor else { return }
        ready = true
        // Activation is asynchronous. Observe native readiness; never prime a key or guess a delay.
        record("ready", ["expected": expected, "source_is_operator_declared": true])
        if source == "local-key-characters" {
            for (index, character) in expected.enumerated() {
                DispatchQueue.main.asyncAfter(deadline: .now() + Double(index) * 0.1) {
                    for type in [NSEvent.EventType.keyDown, .keyUp] {
                        let event = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: [], timestamp: 0,
                            windowNumber: self.window.windowNumber, context: nil, characters: String(character),
                            charactersIgnoringModifiers: String(character), isARepeat: false, keyCode: 0)!
                        NSApp.sendEvent(event)
                    }
                }
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { self.finish() }
        }
    }
    @objc func submit() { submitted += 1; record("submit") }
    @objc func finish() {
        guard !recordedResult else { return }; recordedResult = true
        record("result", ["expected": expected, "matches": editor.string == expected,
            "no_marked": !editor.hasMarkedText(), "no_submit": submitted == 0, "ready": ready])
        NSApp.terminate(nil)
    }
    func windowShouldClose(_ sender: NSWindow) -> Bool { finish(); return true }
}
let app = NSApplication.shared
let fixture = Delegate()
app.setActivationPolicy(.regular); app.delegate = fixture; app.run()
