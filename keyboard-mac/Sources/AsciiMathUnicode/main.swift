import Cocoa
import InputMethodKit

/// The connection apps type through, which the list of offered names shows through too.
let server: IMKServer = {
    if let connectionName = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String,
        let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)
    {
        server
    } else {
        fatalError("can't start the input method server; is this running from its app bundle?")
    }
}()

// the system looks the controller up by its name in Info.plist, which the linker can't see
_ = InputController.self

withExtendedLifetime(server) {
    NSApplication.shared.run()
}
