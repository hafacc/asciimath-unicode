import AsciiMathCore
import Carbon.HIToolbox
import Cocoa
import InputMethodKit
import KeyboardKit

@objc(AsciiMathInputController)
final class InputController: IMKInputController {
    /// The range that makes the client replace the held text, or the selection if none is held.
    private static let heldText = NSRange(location: NSNotFound, length: NSNotFound)

    private let composer = Composer()

    /// The names offered for the one being typed, best first, and empty while the list is closed.
    private var completions: [Completion] = []

    /// Which of `completions` tab puts in.
    private var highlighted = 0

    /// The stored settings, as the composer takes them.
    private static var options: ComposerOptions {
        let settings = KeyboardKit.Settings(defaults: .standard)
        return ComposerOptions(
            delimiter: settings.delimiter.number,
            stripBrackets: settings.pruneParens,
            vulgarFractions: settings.vulgarFractions,
            scriptFractions: settings.scriptFractions,
            skinTone: settings.skinTone.number
        )
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        if let event, event.type == .keyDown, let client = sender as? any IMKTextInput {
            let options = Self.options
            let isOpen = !completions.isEmpty
            let step = isPlain(event, kVK_DownArrow) ? 1 : isPlain(event, kVK_UpArrow) ? -1 : 0
            if isOpen, step != 0 {
                highlighted = (highlighted + step + completions.count) % completions.count
                highlight()
                return true
            } else if isOpen, isPlain(event, kVK_Escape) {
                closeList()
                return true
            } else if !isOpen, isPlain(event, kVK_Tab), openList(options: options) {
                return true
            } else {
                let wasHolding = composer.isHolding
                let picked = isOpen && isPlain(event, kVK_Tab) ? completions[highlighted] : nil
                let outcome =
                    picked.map { composer.complete(name: $0.name, options: options) }
                    ?? composer.press(
                        characters: event.characters ?? "",
                        keyCode: event.keyCode,
                        isShortcut: !event.modifierFlags.isDisjoint(with: [.command, .control]),
                        options: options
                    )
                apply(outcome, wasHolding: wasHolding, to: client)
                if isOpen, picked == nil {
                    _ = openList(options: options)
                } else {
                    closeList()
                }
                return !outcome.passThrough
            }
        } else {
            return false
        }
    }

    /// The names offered, as the list shows them.
    override func candidates(_ sender: Any!) -> [Any]! {
        completions.map(Self.label)
    }

    override func candidateSelectionChanged(_ candidateString: NSAttributedString!) {
        if let index = index(of: candidateString) {
            highlighted = index
        }
    }

    /// Puts in the name clicked in the list.
    override func candidateSelected(_ candidateString: NSAttributedString!) {
        if let index = index(of: candidateString), let client = client() {
            let options = Self.options
            apply(
                composer.complete(name: completions[index].name, options: options),
                wasHolding: true, to: client)
            closeList()
        }
    }

    /// Hands over the held text as typed when the client says typing can't continue there.
    override func commitComposition(_ sender: Any!) {
        let committed = composer.endInput()
        if !committed.isEmpty, let client = sender as? any IMKTextInput {
            client.insertText(committed, replacementRange: Self.heldText)
        }
        closeList()
    }

    override func deactivateServer(_ sender: Any!) {
        commitComposition(sender)
        super.deactivateServer(sender)
    }

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        menu.addItem(
            withTitle: Copy.settingsMenuItem, action: #selector(showSettings(_:)), keyEquivalent: ""
        )
        menu.addItem(
            withTitle: Copy.updateMenuItem, action: #selector(checkForUpdates(_:)),
            keyEquivalent: ""
        )
        return menu
    }

    @MainActor @objc private func showSettings(_ sender: Any?) {
        SettingsWindow.show()
    }

    @MainActor @objc private func checkForUpdates(_ sender: Any?) {
        UpdateCheck.run()
    }

    private static func label(of completion: Completion) -> String {
        if completion.symbol == completion.name {
            completion.name
        } else {
            "\(completion.symbol)  \(completion.name)"
        }
    }

    private func index(of label: NSAttributedString?) -> Int? {
        completions.firstIndex { Self.label(of: $0) == label?.string }
    }

    /// Whether the event is the key with `keyCode`, with no modifier held.
    private func isPlain(_ event: NSEvent, _ keyCode: Int) -> Bool {
        event.keyCode == UInt16(keyCode)
            && event.modifierFlags.isDisjoint(with: [.command, .control, .option, .shift])
    }

    /// Opens the list with the names that finish the one being typed, and says whether any do.
    private func openList(options: ComposerOptions) -> Bool {
        completions = composer.completions(options: options)
        highlighted = 0
        let hasCompletions = !completions.isEmpty
        // the system calls an input controller only on the main thread
        MainActor.assumeIsolated {
            if hasCompletions {
                CompletionList.show()
            } else {
                CompletionList.hide()
            }
        }
        return hasCompletions
    }

    private func closeList() {
        completions = []
        MainActor.assumeIsolated { CompletionList.hide() }
    }

    private func highlight() {
        let line = highlighted
        MainActor.assumeIsolated { CompletionList.highlight(line: line) }
    }

    private func apply(_ outcome: Outcome, wasHolding: Bool, to client: any IMKTextInput) {
        if !outcome.committed.isEmpty {
            client.insertText(outcome.committed, replacementRange: Self.heldText)
        }
        if !outcome.marked.isEmpty || (wasHolding && outcome.committed.isEmpty) {
            show(marked: outcome.marked, in: client)
        }
    }

    private func show(marked: String, in client: any IMKTextInput) {
        let underlined = NSAttributedString(
            string: marked,
            attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
                .markedClauseSegment: 0,
            ])
        client.setMarkedText(
            underlined,
            // the caret only ever sits at the end
            selectionRange: NSRange(location: underlined.length, length: 0),
            replacementRange: Self.heldText
        )
    }
}

/// The list that offers names, shared by every input controller, which the system asks for them.
@MainActor private enum CompletionList {
    private static var panel: IMKCandidates?

    /// Shows the list with the names the active input controller offers, the first highlighted.
    static func show() {
        let panel =
            panel
            ?? IMKCandidates(server: server, panelType: kIMKSingleColumnScrollingCandidatePanel)
        Self.panel = panel
        panel?.update()
        panel?.show(kIMKLocateCandidatesBelowHint)
        highlight(line: 0)
    }

    static func hide() {
        panel?.hide()
    }

    static func highlight(line: Int) {
        if let panel {
            panel.selectCandidate(withIdentifier: panel.candidateIdentifier(atLineNumber: line))
        }
    }
}
