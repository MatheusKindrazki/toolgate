import Darwin
import SwiftUI

struct DaemonHealth: Codable { let status: String; let version: String }
struct ToolgateEvent: Codable, Identifiable { let id: Int; let timestamp: String; let agent: String; let event_type: String; let tool_name: String?; let decision: String }

@main struct ToolgateApp: App {
    @State private var health = "Connecting…"
    @State private var events: [ToolgateEvent] = []
    var body: some Scene {
        MenuBarExtra("Toolgate", systemImage: health == "ok" ? "checkmark.shield" : "exclamationmark.shield") {
            VStack(alignment: .leading) {
                Text("Toolgate").font(.headline)
                Text("Daemon: \(health)")
                ForEach(events.prefix(5)) { event in Text("\(event.agent): \(event.decision)").font(.caption) }
                Divider(); Text("Coverage is adapter-mediated, not OS-wide.").font(.caption)
                Button("Quit") { NSApplication.shared.terminate(nil) }
            }.padding().task { await refresh() }
        }
        WindowGroup { ContentView(health: health, events: events).task { await refresh() } }
    }
    private func refresh() async { health = await DaemonClient.health(); events = await DaemonClient.events() }
}
struct ContentView: View {
    let health: String; let events: [ToolgateEvent]
    var body: some View { TabView { List(events) { event in VStack(alignment: .leading) { Text("\(event.agent) · \(event.decision)"); Text(event.tool_name ?? event.event_type).font(.caption) } }.overlay { if events.isEmpty { ContentUnavailableView("No events", systemImage: "list.bullet") } }.tabItem { Label("Feed", systemImage: "list.bullet") } }.frame(minWidth: 400, minHeight: 240).navigationTitle("Daemon: \(health)") }
}
enum DaemonClient {
    static func health() async -> String { await Task.detached { (try? request("health").status) ?? "unavailable" }.value }
    static func events() async -> [ToolgateEvent] { await Task.detached { (try? request("list_events").events) ?? [] }.value }
    private struct Response: Decodable { let type: String; let params: Data
        enum CodingKeys: String, CodingKey { case type; case params }
        init(from decoder: Decoder) throws { let container = try decoder.container(keyedBy: CodingKeys.self); type = try container.decode(String.self, forKey: .type); params = try JSONEncoder().encode(try container.decode(JSONValue.self, forKey: .params)) }
        var status: String { (try? JSONDecoder().decode(DaemonHealth.self, from: params).status) ?? "unavailable" }
        var events: [ToolgateEvent] { (try? JSONDecoder().decode([ToolgateEvent].self, from: params)) ?? [] }
    }
    private enum JSONValue: Codable { case object([String: JSONValue]), array([JSONValue]), string(String), number(Double), bool(Bool), null
        init(from decoder: Decoder) throws { let c = try decoder.singleValueContainer(); if c.decodeNil(){self = .null}else if let v = try? c.decode(Bool.self){self = .bool(v)}else if let v = try? c.decode(Double.self){self = .number(v)}else if let v = try? c.decode(String.self){self = .string(v)}else if let v = try? c.decode([JSONValue].self){self = .array(v)}else{self = .object(try c.decode([String: JSONValue].self))} }
        func encode(to encoder: Encoder) throws { var c=encoder.singleValueContainer(); switch self { case .object(let v):try c.encode(v);case .array(let v):try c.encode(v);case .string(let v):try c.encode(v);case .number(let v):try c.encode(v);case .bool(let v):try c.encode(v);case .null:try c.encodeNil() } }
    }
    private static func request(_ type: String) throws -> Response {
        let path = ProcessInfo.processInfo.environment["TOOLGATE_SOCKET"] ?? (NSHomeDirectory() + "/Library/Application Support/Toolgate/run/daemon.sock")
        let fd = socket(AF_UNIX, SOCK_STREAM, 0); guard fd >= 0 else { throw POSIXError(.ENOTCONN) }; defer { close(fd) }
        var address = sockaddr_un(); address.sun_family = sa_family_t(AF_UNIX); let utf8 = Array(path.utf8) + [0]
        guard utf8.count <= MemoryLayout.size(ofValue: address.sun_path) else { throw POSIXError(.ENAMETOOLONG) }
        withUnsafeMutableBytes(of: &address.sun_path) { $0.copyBytes(from: utf8) }
        let result = withUnsafePointer(to: &address) { pointer in pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size)) } }
        guard result == 0 else { throw POSIXError(.ENOTCONN) }
        let body = try JSONSerialization.data(withJSONObject: ["version":1,"type":type,"params":[:]])
        var size = UInt32(body.count).bigEndian; try writeAll(fd, Data(bytes: &size, count: 4)); try writeAll(fd, body)
        let header = try readExactly(fd, 4); let length = header.withUnsafeBytes { $0.loadUnaligned(as: UInt32.self).bigEndian }; guard length <= 1_048_576 else { throw POSIXError(.EMSGSIZE) }
        return try JSONDecoder().decode(Response.self, from: readExactly(fd, Int(length)))
    }
    private static func writeAll(_ fd: Int32, _ data: Data) throws { let wrote = data.withUnsafeBytes { Darwin.write(fd, $0.baseAddress, data.count) }; if wrote != data.count { throw POSIXError(.EIO) } }
    private static func readExactly(_ fd: Int32, _ count: Int) throws -> Data { var bytes=[UInt8](repeating:0,count:count); var offset=0; while offset<count { let n = bytes.withUnsafeMutableBytes { Darwin.read(fd, $0.baseAddress!.advanced(by: offset), count-offset) }; if n<=0 { throw POSIXError(.ECONNRESET) }; offset += n }; return Data(bytes) }
}
