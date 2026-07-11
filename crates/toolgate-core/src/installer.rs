use serde_json::{Value, json};
use std::{fs, path::Path};
const MARKER: &str = "toolgate-v0.1";
pub fn install_claude(settings: &Path, command: &str) -> Result<(), String> {
    let mut root = read(settings)?;
    let hooks = root
        .as_object_mut()
        .ok_or("settings must be a JSON object")?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let obj = hooks.as_object_mut().ok_or("hooks must be an object")?;
    for (event, matcher, phase) in [
        ("PreToolUse", "Bash|Write|Edit|Read", "pre"),
        ("PostToolUse", "*", "post"),
    ] {
        let groups = obj
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or("hook list must be an array")?;
        if !groups
            .iter()
            .any(|g| g.pointer("/hooks/0/toolgate_owner").and_then(Value::as_str) == Some(MARKER))
        {
            groups.push(json!({"matcher":matcher,"hooks":[{"type":"command","command":command,"args":["claude-code",phase],"timeout":5,"toolgate_owner":MARKER}]}));
        }
    }
    write(settings, &root)
}
pub fn uninstall_claude(settings: &Path) -> Result<(), String> {
    let mut root = read(settings)?;
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        for event in ["PreToolUse", "PostToolUse"] {
            if let Some(groups) = hooks.get_mut(event).and_then(Value::as_array_mut) {
                groups.retain(|g| {
                    g.pointer("/hooks/0/toolgate_owner").and_then(Value::as_str) != Some(MARKER)
                });
            }
        }
    }
    write(settings, &root)
}
fn read(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_and_uninstall_preserve_user_config() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        fs::write(&p,r#"{"theme":"dark","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"mine"}]}]}}"#).unwrap();
        install_claude(&p, "/bin/toolgate-hook").unwrap();
        uninstall_claude(&p).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(p).unwrap()).unwrap();
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }
}
