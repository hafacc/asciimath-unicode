Ascii Math Unicode keyboard
===========================

A macOS keyboard (an input source) that converts [ascii math](http://asciimath.org/) to unicode as you type.
Ordinary typing passes straight through, so it can stay selected all the time.
After the opening delimiter (`$$` by default) what you type is held, underlined, and shown converted as you go; the closing delimiter or Return replaces it with the result, and Escape leaves it as typed.
While you type a name, like `alp` or the emoji name `:smi`, Tab opens a list of the names that finish it below the text. It narrows as you keep typing; up and down move through it, Tab again puts in the highlighted name, and Escape closes it.

It converts with the same Rust library as the extension, through the small binding in `../keyboard-core/`.

Building
--------

Needs Xcode and a Rust toolchain (`cargo` in `~/.cargo/bin` or on the `PATH`).
Open `AsciiMathUnicode.xcodeproj`; everything is done from there, in the AsciiMathUnicode scheme: Build (⌘B) builds the app, running cargo for the Rust library first. Test (⌘U) runs the Swift tests and the Rust library's tests. Archive makes the copy to install or release.

Every build also checks the code and fails on a finding: rustfmt and clippy on `../keyboard-core/`, `swift format` on `Sources` and `Tests` and compiler warnings.

The app lands in Xcode's build folder (Product › Show Build Folder in Finder).
Archive and any other Release build are for both kinds of Mac, so they need `rustup target add x86_64-apple-darwin` once.

From a terminal, the same things are `xcodebuild -project AsciiMathUnicode.xcodeproj -scheme AsciiMathUnicode build` (or `test`).

Installing
----------

1. Run `rustup target add x86_64-apple-darwin` once, since Archive builds for both kinds of Mac.
2. In Xcode, Product › Archive. When it finishes, the installer `build/AsciiMathUnicode-VERSION.pkg` is shown in Finder.
3. Open the package and follow the installer.
4. Log out and back in.
5. The first time, add it in System Settings › Keyboard › Text Input › Edit… › + (it is listed under English) and pick it from the input menu in the menu bar.

The package puts the app in `/Library/Input Methods`, for every user of the Mac, and quits a running copy so the new one is used the next time you type.
It leaves a copy in your own `~/Library/Input Methods` alone; delete that one yourself, or there will be two.

The package is made by `Installer/package.sh`, which the scheme runs after Archive.
Xcode doesn't report whether such a step failed, so if something went wrong it leaves `build/package.log` and opens it.
From a terminal, `xcodebuild -project AsciiMathUnicode.xcodeproj -scheme AsciiMathUnicode -archivePath build/AsciiMathUnicode.xcarchive archive` makes the same package; check that the file is there, since `xcodebuild` succeeds without it.

macOS turns every keyboard like this off in password fields.

Uninstalling
------------

```
sudo "/Library/Input Methods/AsciiMathUnicode.app/Contents/Resources/uninstall.sh"
```

It quits the keyboard, deletes the app and forgets the installer's record of it; add `--settings` to delete your options too.
Then remove the keyboard in System Settings › Keyboard › Text Input › Edit…, and log out and back in.

By hand, the same is: `killall AsciiMathUnicode`, delete `/Library/Input Methods/AsciiMathUnicode.app`, `sudo pkgutil --forget cc.hafa.inputmethod.AsciiMathUnicode`, and for the options `defaults delete cc.hafa.inputmethod.AsciiMathUnicode`.

Icons
-----

Each build renders the app icon and the input menu icon from the product's icon, `extension/public/am.svg`, and its one-colour form, `extension/am-off.svg`, with tools that come with macOS.
The rendered files land under `Resources/` and aren't checked in; they are only rendered again when an SVG changes.

Settings
--------

Options in the input menu opens the settings window.
They are stored in the keyboard's own preferences (`defaults read cc.hafa.inputmethod.AsciiMathUnicode`), under the same names and with the same defaults as the extension's options.

Releasing
---------

The project signs to run locally and the package is unsigned, which is enough for a Mac you can approve it on.
A copy for anyone else needs a paid Apple developer account, and then:

1. The app signed with a Developer ID Application certificate and the hardened runtime: in the target's Signing & Capabilities, or with the build settings `CODE_SIGN_IDENTITY = Developer ID Application`, `ENABLE_HARDENED_RUNTIME = YES` and the team.
2. The package signed with a Developer ID Installer certificate. `Installer/package.sh` does this by itself once the certificate is in the keychain; the build setting `INSTALLER_SIGN_IDENTITY` picks one if there are several.
3. Notarization of the package: `xcrun notarytool submit build/AsciiMathUnicode-VERSION.pkg --wait` with an App Store Connect key or an app-specific password, then `xcrun stapler staple` on the package.
4. In CI, the two certificates (as `.p12` files with their passwords) and the notarization credentials as secrets.

The version is `CFBundleShortVersionString` in `Info.plist`.
