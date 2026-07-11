import XCTest
@testable import ToolgateApp

final class ToolgateAppTests: XCTestCase {
    func testHealthModelDecodes() throws {
        let value = try JSONDecoder().decode(
            DaemonHealth.self,
            from: Data(#"{"status":"ok","version":"0.1.0"}"#.utf8)
        )
        XCTAssertEqual(value.status, "ok")
    }
}
