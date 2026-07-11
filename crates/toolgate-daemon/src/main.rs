use anyhow::Result;
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
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
    prepare_socket_parent(path)?;
    if path.exists() {
        #[cfg(unix)]
        if !fs::symlink_metadata(path)?.file_type().is_socket() {
            anyhow::bail!(
                "refusing to replace a non-socket file at {}",
                path.display()
            );
        }
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

fn prepare_socket_parent(socket: &Path) -> Result<()> {
    let Some(parent) = socket.parent() else {
        return Ok(());
    };
    if parent.exists() {
        return Ok(());
    }
    let container = parent
        .parent()
        .ok_or_else(|| anyhow::anyhow!("socket path has no parent container"))?;
    fs::create_dir_all(container)?;
    match fs::create_dir(parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    #[cfg(unix)]
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn serve_does_not_chmod_an_existing_caller_owned_socket_parent() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("caller-owned");
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        let socket = parent.join("daemon.sock");
        let socket_string = socket.to_string_lossy().into_owned();
        let task = tokio::spawn(async move { serve(&socket_string).await });

        for _ in 0..50 {
            if socket.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert!(socket.exists());
        assert_eq!(
            fs::metadata(parent).unwrap().permissions().mode() & 0o777,
            0o755
        );
        task.abort();
        let _ = task.await;
    }

    #[cfg(unix)]
    #[test]
    fn creates_and_secures_a_missing_socket_parent() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("toolgate/run");

        prepare_socket_parent(&parent.join("daemon.sock")).unwrap();

        assert_eq!(
            fs::metadata(parent).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_to_replace_a_regular_file_at_the_socket_path() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("daemon.sock");
        fs::write(&socket, "not a socket").unwrap();

        let error = serve(socket.to_str().unwrap()).await.unwrap_err();

        assert!(
            error
                .to_string()
                .contains("refusing to replace a non-socket")
        );
    }
}
