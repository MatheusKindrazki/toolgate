use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Enforced,
    Observed,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Deny,
    Ask,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub agent: String,
    pub project_dir: Option<String>,
    pub event_type: String,
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Value,
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    #[serde(default = "enforced")]
    pub capability: CapabilityState,
}
fn enforced() -> CapabilityState {
    CapabilityState::Enforced
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Envelope {
    pub version: u8,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub params: Value,
}
impl Envelope {
    pub fn response(id: Option<String>, ok: bool, data: Value) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            id,
            kind: if ok { "response" } else { "error" }.into(),
            params: data,
        }
    }
}

pub fn encode_frame(message: &Envelope) -> Result<Vec<u8>, serde_json::Error> {
    let body = serde_json::to_vec(message)?;
    let mut frame = Vec::with_capacity(body.len() + 4);
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}
pub fn decode_frame(frame: &[u8]) -> Result<Envelope, FrameError> {
    if frame.len() < 4 {
        return Err(FrameError::Truncated);
    }
    let length = u32::from_be_bytes(frame[..4].try_into().expect("four bytes")) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(FrameError::Oversized(length));
    }
    if frame.len() != length + 4 {
        return Err(FrameError::Truncated);
    }
    serde_json::from_slice(&frame[4..]).map_err(FrameError::Json)
}
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("truncated frame")]
    Truncated,
    #[error("frame size {0} exceeds limit")]
    Oversized(usize),
    #[error("invalid JSON: {0}")]
    Json(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_malformed_and_oversized_frames() {
        assert!(matches!(decode_frame(&[0; 3]), Err(FrameError::Truncated)));
        let f = (MAX_FRAME_BYTES as u32 + 1).to_be_bytes();
        assert!(matches!(decode_frame(&f), Err(FrameError::Oversized(_))));
    }
    #[test]
    fn round_trips_frame() {
        let e = Envelope {
            version: 1,
            id: Some("a".into()),
            kind: "health".into(),
            params: Value::Null,
        };
        assert_eq!(
            decode_frame(&encode_frame(&e).unwrap()).unwrap().kind,
            "health"
        );
    }
}
