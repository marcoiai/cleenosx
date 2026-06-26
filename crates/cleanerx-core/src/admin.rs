use crate::models::AdminSessionStatus;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
struct AdminSessionState {
    unlocked: bool,
    last_unlocked_at_ms: Option<u64>,
}

static ADMIN_SESSION: OnceLock<Mutex<AdminSessionState>> = OnceLock::new();

pub fn admin_session_status() -> AdminSessionStatus {
    let state = ADMIN_SESSION
        .get_or_init(|| Mutex::new(AdminSessionState::default()))
        .lock()
        .expect("admin session poisoned")
        .clone();

    AdminSessionStatus {
        unlocked: state.unlocked,
        available: true,
        last_unlocked_at_ms: state.last_unlocked_at_ms,
        message: if state.unlocked {
            "Admin Mode is enabled for this app session. CleanerX will prefer administrator cleanup when available.".to_string()
        } else {
            "Admin Mode is off. Unlock once to reuse administrator cleanup through this app session.".to_string()
        },
    }
}

pub fn unlock_admin_session() -> Result<AdminSessionStatus, String> {
    let status = Command::new("osascript")
        .arg("-e")
        .arg(r#"do shell script "/usr/bin/true" with administrator privileges"#)
        .status()
        .map_err(|error| format!("failed to request administrator privileges: {error}"))?;

    if !status.success() {
        return Err(format!(
            "administrator authorization did not complete: {}",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ));
    }

    let mut state = ADMIN_SESSION
        .get_or_init(|| Mutex::new(AdminSessionState::default()))
        .lock()
        .expect("admin session poisoned");
    state.unlocked = true;
    state.last_unlocked_at_ms = Some(now_ms());
    drop(state);

    Ok(admin_session_status())
}

pub fn lock_admin_session() -> AdminSessionStatus {
    let mut state = ADMIN_SESSION
        .get_or_init(|| Mutex::new(AdminSessionState::default()))
        .lock()
        .expect("admin session poisoned");
    state.unlocked = false;
    drop(state);
    admin_session_status()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .unwrap_or_default()
}
