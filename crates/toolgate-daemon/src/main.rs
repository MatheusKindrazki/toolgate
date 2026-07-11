use anyhow::Result;
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{env, fs, path::Path};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use toolgate_core::{
    policy::evaluate,
    protocol::{Envelope, Event, MAX_FRAME_BYTES},
    redaction::redact,
    store::Store,
};
#[tokio::main]
async fn main() -> Result<()> {
    let socket = env::args().nth(1).unwrap_or_else(|| {
        format!(
            "{}/Library/Application Support/Toolgate/run/daemon.sock",
            env::var("HOME").unwrap_or_else(|_| ".".into())
        )
    });
    serve(&socket).await
}
pub async fn serve(socket: &str) -> Result<()> {
    let path = Path::new(socket);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    let database = path
        .with_extension("sqlite3")
        .to_string_lossy()
        .into_owned();
    loop {
        let (stream, _) = listener.accept().await?;
        let database = database.clone();
        tokio::spawn(async move {
            let _ = handle(stream, &database).await;
        });
    }
}
async fn handle(mut stream: UnixStream, database: &str) -> Result<()> {
    let request = read(&mut stream).await?;
    let response = match request.kind.as_str() {
        "health" => Envelope::response(
            request.id,
            true,
            json!({"status":"ok","version":"0.1.0","capabilities":{"claude_code":"enforced","codex":"unsupported","hermes":"unsupported"}}),
        ),
        "evaluate" => match serde_json::from_value::<Event>(
            request.params.get("event").cloned().unwrap_or(Value::Null),
        ) {
            Ok(event) => {
                let d = evaluate(&event, &[]);
                let redacted = redact(&event.tool_input);
                match Store::open(database)
                    .and_then(|store| store.persist(&event, d.action, d.policy_id, &redacted))
                {
                    Ok(event_id) => Envelope::response(
                        request.id,
                        true,
                        json!({"event_id":event_id,"action":d.action,"state":d.state,"redacted_input":redacted}),
                    ),
                    Err(error) => Envelope::response(
                        request.id,
                        false,
                        json!({"error":format!("persistence failed: {error}")}),
                    ),
                }
            }
            Err(_) => Envelope::response(request.id, false, json!({"error":"invalid event"})),
        },
        "list_events" => match Store::open(database).and_then(|store| store.recent_events(20)) {
            Ok(events) => Envelope::response(request.id, true, json!(events)),
            Err(error) => Envelope::response(
                request.id,
                false,
                json!({"error":format!("read failed: {error}")}),
            ),
        },
        _ => Envelope::response(request.id, false, json!({"error":"unsupported request"})),
    };
    write(&mut stream, &response).await
}
pub async fn read(stream: &mut UnixStream) -> Result<Envelope> {
    let mut size = [0; 4];
    stream.read_exact(&mut size).await?;
    let n = u32::from_be_bytes(size) as usize;
    if n > MAX_FRAME_BYTES {
        anyhow::bail!("oversized frame")
    }
    let mut body = vec![0; n];
    stream.read_exact(&mut body).await?;
    Ok(serde_json::from_slice(&body)?)
}
pub async fn write(stream: &mut UnixStream, message: &Envelope) -> Result<()> {
    let body = serde_json::to_vec(message)?;
    stream.write_all(&(body.len() as u32).to_be_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;
    Ok(())
}
