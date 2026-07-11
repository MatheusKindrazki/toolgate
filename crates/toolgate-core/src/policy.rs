use crate::protocol::{Action, CapabilityState, Event};
use globset::Glob;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Policy {
    pub id: Option<i64>,
    pub scope: String,
    pub agent: Option<String>,
    pub project_dir: Option<String>,
    pub tool_name: Option<String>,
    pub pattern: Option<String>,
    pub action: Action,
    pub priority: i32,
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decision {
    pub action: Action,
    pub state: CapabilityState,
    pub policy_id: Option<i64>,
}
pub fn evaluate(event: &Event, policies: &[Policy]) -> Decision {
    if event.capability != CapabilityState::Enforced {
        return Decision {
            action: Action::Allow,
            state: event.capability,
            policy_id: None,
        };
    }
    let mut matched: Vec<&Policy> = policies
        .iter()
        .filter(|p| p.enabled && matches(p, event))
        .collect();
    matched.sort_by_key(|p| (-p.priority, action_rank(p.action)));
    matched
        .first()
        .map(|p| Decision {
            action: p.action,
            state: CapabilityState::Enforced,
            policy_id: p.id,
        })
        .unwrap_or_else(|| default_for(event))
}
fn action_rank(action: Action) -> i8 {
    match action {
        Action::Deny => 0,
        Action::Ask => 1,
        Action::Allow => 2,
    }
}
fn matches(p: &Policy, e: &Event) -> bool {
    p.agent.as_ref().is_none_or(|x| x == &e.agent)
        && p.project_dir
            .as_ref()
            .is_none_or(|x| e.project_dir.as_ref() == Some(x))
        && p.tool_name
            .as_ref()
            .is_none_or(|x| e.tool_name.as_ref() == Some(x))
        && p.pattern.as_ref().is_none_or(|x| {
            input_path(&e.tool_input).is_some_and(|path| {
                Glob::new(x)
                    .ok()
                    .map(|g| g.compile_matcher().is_match(path))
                    .unwrap_or(false)
            })
        })
}
fn input_path(value: &serde_json::Value) -> Option<&str> {
    value
        .get("file_path")
        .or_else(|| value.get("path"))
        .and_then(|v| v.as_str())
}
fn default_for(e: &Event) -> Decision {
    let input = e.tool_input.to_string();
    let sensitive = input_path(&e.tool_input).is_some_and(is_sensitive_path)
        || input.contains(".env")
        || input.contains(".ssh")
        || input.contains(".aws");
    let destructive = e.tool_name.as_deref() == Some("Bash")
        && ["git push", "rm -rf", "npm install", "pip install"]
            .iter()
            .any(|s| input.contains(s));
    let read = matches!(e.tool_name.as_deref(), Some("Read")) && !sensitive;
    Decision {
        action: if sensitive {
            Action::Deny
        } else if destructive {
            Action::Ask
        } else if read {
            Action::Allow
        } else {
            Action::Deny
        },
        state: CapabilityState::Enforced,
        policy_id: None,
    }
}
pub fn is_sensitive_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.ends_with("/.env")
        || normalized == ".env"
        || normalized.contains("/.ssh/")
        || normalized.contains("/.aws/")
        || normalized.starts_with("~/.ssh")
        || normalized.starts_with("~/.aws")
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn event(path: &str) -> Event {
        Event {
            agent: "claude-code".into(),
            project_dir: Some("/work".into()),
            event_type: "tool_call".into(),
            tool_name: Some("Write".into()),
            tool_input: json!({"file_path":path}),
            pid: None,
            session_id: None,
            capability: CapabilityState::Enforced,
        }
    }
    #[test]
    fn sensitive_paths_are_denied() {
        assert_eq!(
            evaluate(&event("/Users/a/.ssh/id_ed25519"), &[]).action,
            Action::Deny
        );
        assert_eq!(evaluate(&event("/work/.env"), &[]).action, Action::Deny);
    }
    #[test]
    fn deny_wins_equal_priority_conflict() {
        let deny = Policy {
            id: Some(1),
            scope: "tool".into(),
            agent: None,
            project_dir: None,
            tool_name: Some("Write".into()),
            pattern: None,
            action: Action::Deny,
            priority: 30,
            enabled: true,
        };
        let mut allow = deny.clone();
        allow.id = Some(2);
        allow.action = Action::Allow;
        assert_eq!(
            evaluate(&event("/work/a"), &[allow, deny]).action,
            Action::Deny
        );
    }
    #[test]
    fn observed_cannot_claim_enforcement() {
        let mut e = event("/work/a");
        e.capability = CapabilityState::Observed;
        assert_eq!(evaluate(&e, &[]).state, CapabilityState::Observed);
    }
}
