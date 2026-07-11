use serde_json::{Value, json};
use std::{
    env,
    io::{self, Read},
};
use tokio::{
    net::UnixStream,
    time::{Duration, timeout},
};
use toolgate_core::{
    installer::{install_claude, uninstall_claude},
    protocol::{CapabilityState, Envelope, Event, PROTOCOL_VERSION},
};
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if matches!(
        args.get(1).map(String::as_str),
        Some("install" | "uninstall")
    ) {
        let settings = args
            .get(2)
            .map(String::as_str)
            .unwrap_or("~/.claude/settings.json");
        let settings = expand_home(settings);
        let result = if args[1] == "install" {
            install_claude(
                std::path::Path::new(&settings),
                args.get(3).map(String::as_str).unwrap_or("toolgate-hook"),
            )
        } else {
            uninstall_claude(std::path::Path::new(&settings))
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    let phase = args.get(2).cloned().unwrap_or_else(|| "pre".into());
    let socket = env::var("TOOLGATE_SOCKET").unwrap_or_else(|_| {
        format!(
            "{}/Library/Application Support/Toolgate/run/daemon.sock",
            env::var("HOME").unwrap_or_else(|_| ".".into())
        )
    });
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() {
        deny("cannot read hook input");
        return;
    }
    let input: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            deny("malformed hook input");
            return;
        }
    };
    if phase == "post" {
        return;
    }
    let event = Event {
        agent: "claude-code".into(),
        project_dir: input.get("cwd").and_then(Value::as_str).map(str::to_owned),
        event_type: "tool_call".into(),
        tool_name: input
            .get("tool_name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        tool_input: input.get("tool_input").cloned().unwrap_or(Value::Null),
        pid: None,
        session_id: input
            .get("session_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        capability: CapabilityState::Enforced,
    };
    let request = Envelope {
        version: PROTOCOL_VERSION,
        id: Some("hook".into()),
        kind: "evaluate".into(),
        params: json!({"event":event}),
    };
    let allowed = fail_closed(
        timeout(Duration::from_secs(5), ask(&socket, &request))
            .await
            .ok(),
    );
    if !allowed {
        deny("Toolgate denied or timed out")
    }
}
fn expand_home(path: &str) -> String {
    path.strip_prefix("~/")
        .map(|suffix| {
            format!(
                "{}/{}",
                env::var("HOME").unwrap_or_else(|_| ".".into()),
                suffix
            )
        })
        .unwrap_or_else(|| path.into())
}
fn fail_closed(result: Option<anyhow::Result<bool>>) -> bool {
    result.and_then(Result::ok).unwrap_or(false)
}
async fn ask(socket: &str, request: &Envelope) -> anyhow::Result<bool> {
    let mut s = UnixStream::connect(socket).await?;
    let body = serde_json::to_vec(request)?;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    s.write_all(&(body.len() as u32).to_be_bytes()).await?;
    s.write_all(&body).await?;
    let mut h = [0; 4];
    s.read_exact(&mut h).await?;
    let mut b = vec![0; u32::from_be_bytes(h) as usize];
    s.read_exact(&mut b).await?;
    let response: Envelope = serde_json::from_slice(&b)?;
    Ok(response.params.get("action").and_then(Value::as_str) == Some("allow"))
}
fn deny(reason: &str) {
    eprintln!("{reason}");
    std::process::exit(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn timeout_fails_closed() {
        let timed_out = timeout(
            Duration::from_millis(1),
            std::future::pending::<anyhow::Result<bool>>(),
        )
        .await
        .ok();
        assert!(!fail_closed(timed_out));
    }
}
