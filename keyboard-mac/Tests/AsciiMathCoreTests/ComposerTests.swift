import AsciiMathCore
import Carbon.HIToolbox
import KeyboardKit
import Testing

/// What is in the document and what is still held after typing text one character at a time.
private struct Typed: Equatable {
    var document = ""
    var marked = ""
}

private func type(
    _ text: String, into composer: Composer, options: ComposerOptions = ComposerOptions()
) -> Typed {
    var typed = Typed()
    for character in text {
        let outcome = composer.press(
            characters: String(character), keyCode: UInt16(kVK_ANSI_A), isShortcut: false,
            options: options)
        typed.document += outcome.committed
        typed.marked = outcome.marked
        if outcome.passThrough {
            typed.document.append(character)
        }
    }
    return typed
}

@Test func composesThroughTheRustLibrary() {
    let composer = Composer()
    #expect(type("a $$1/2", into: composer) == Typed(document: "a ", marked: "$$½"))
    #expect(composer.isHolding)
    #expect(type(" + x^$$", into: composer) == Typed(document: "½ + x"))
    #expect(!composer.isHolding)
    #expect(type("$$x^", into: composer) == Typed(marked: "$$x⸋"))
    #expect(composer.endInput() == "$$x^")
    #expect(composer.endInput() == "")
}

@Test func editingKeysGoByKeyCode() {
    let composer = Composer()
    let options = ComposerOptions()
    func press(_ characters: String, _ keyCode: Int, isShortcut: Bool = false) -> Outcome {
        composer.press(
            characters: characters, keyCode: UInt16(keyCode), isShortcut: isShortcut,
            options: options)
    }
    _ = type("$$xy", into: composer)
    #expect(press("\u{7f}", kVK_Delete) == Outcome(marked: "$$x", passThrough: false))
    #expect(press("\r", kVK_Return) == Outcome(committed: "x", passThrough: false))
    _ = type("$$x", into: composer)
    #expect(press("\u{1b}", kVK_Escape) == Outcome(committed: "$$x", passThrough: false))
    _ = type("$$x", into: composer)
    #expect(press("\u{F702}", kVK_LeftArrow) == Outcome(committed: "$$x", passThrough: true))
    _ = type("$$x", into: composer)
    #expect(
        press("v", kVK_ANSI_V, isShortcut: true) == Outcome(committed: "$$x", passThrough: true))
}

@Test func passesTheOptionsOn() {
    let composer = Composer()
    #expect(type("$$y/x$$", into: composer).document == "ʸ⁄ₓ")
    #expect(
        type("$$y/x$$", into: composer, options: ComposerOptions(scriptFractions: false)).document
            == "y/x")
    #expect(
        type("$$:hand:$$", into: composer, options: ComposerOptions(skinTone: 5)).document == "✋🏿")
}

@Test func delimitersMatchTheRustLibrary() {
    let composer = Composer()
    for delimiter in DelimiterName.allCases {
        let options = ComposerOptions(delimiter: delimiter.number)
        #expect(
            type("\(delimiter.open)x\(delimiter.close)", into: composer, options: options)
                == Typed(document: "x"), "\(delimiter)")
    }
}

@Test func offersAndPutsInNames() {
    let composer = Composer()
    let options = ComposerOptions()
    #expect(composer.completions(options: options).isEmpty)
    _ = type("$$x :smi", into: composer)
    #expect(
        composer.completions(options: options).first == Completion(name: ":smile:", symbol: "😄"))
    #expect(
        composer.complete(name: ":smile:", options: options)
            == Outcome(marked: "$$x 😄", passThrough: false))
    #expect(composer.completions(options: options).isEmpty)
    _ = type(" :han", into: composer)
    #expect(composer.completions(options: ComposerOptions(skinTone: 5)).first?.symbol == "✋🏿")
    _ = type(" + alp", into: composer)
    #expect(composer.completions(options: options).first == Completion(name: "alpha", symbol: "α"))
}
