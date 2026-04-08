// @session-dev @session-api
//! Session registry -- tracks registered PTY sessions.

use std::collections::HashMap;

use chrono::Utc;
use session_contracts::{AuthorityPosture, SessionLifecycle, SessionRegistration, SessionState};

/// Lightweight in-memory record used by the registry to track a launched
/// session before (and after) it is registered.
#[derive(Debug, Clone)]
pub struct RegistryLaunchRecord {
    pub session_id: String,
    pub project_id: String,
    pub machine_id: String,
    pub session_name: Option<String>,
    pub session_ref: Option<String>,
    pub lifecycle: SessionLifecycle,
    pub authority: AuthorityPosture,
    pub interactive_capable: bool,
}

#[derive(Default)]
pub struct SessionRegistry {
    launches: HashMap<String, RegistryLaunchRecord>,
    registrations: HashMap<String, SessionRegistration>,
}

impl SessionRegistry {
    /// Seed a launch record into the registry.
    pub fn seed_launch(&mut self, record: RegistryLaunchRecord) {
        self.launches.insert(record.session_id.clone(), record);
    }

    pub fn register(
        &mut self,
        session_id: &str,
        project_id: &str,
        identity: &str,
        role: &str,
        machine_id: &str,
    ) -> SessionRegistration {
        let registration = SessionRegistration {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            identity: identity.to_string(),
            role: role.to_string(),
            machine_id: machine_id.to_string(),
            registered_at: Utc::now(),
        };
        self.registrations.insert(session_id.to_string(), registration.clone());
        if let Some(launch) = self.launches.get_mut(session_id) {
            launch.lifecycle = SessionLifecycle::Registered;
        }
        registration
    }

    pub fn state(&self, session_id: &str) -> Option<SessionState> {
        let launch = self.launches.get(session_id)?;
        let registration = self.registrations.get(session_id);
        Some(SessionState {
            session_id: launch.session_id.clone(),
            machine_id: launch.machine_id.clone(),
            project_id: launch.project_id.clone(),
            identity: registration.map(|value| value.identity.clone()),
            role: registration.map(|value| value.role.clone()),
            session_name: launch.session_name.clone(),
            session_ref: launch.session_ref.clone(),
            source: Some("launch".into()),
            status: Some("active".into()),
            notify_target: None,
            last_heartbeat_at: Utc::now(),
            interactive_capable: launch.interactive_capable,
            lifecycle: launch.lifecycle,
            authority: launch.authority,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_upgrades_launched_session() {
        let mut registry = SessionRegistry::default();
        registry.seed_launch(RegistryLaunchRecord {
            session_id: "s1".into(),
            project_id: "sample".into(),
            machine_id: "m1".into(),
            session_name: Some("alpha".into()),
            session_ref: None,
            lifecycle: SessionLifecycle::Launched,
            authority: AuthorityPosture::Source,
            interactive_capable: true,
        });

        registry.register("s1", "sample", "worker-1", "implement", "m1");
        assert!(registry.state("s1").unwrap().is_registered());
    }

    #[test]
    fn state_returns_none_for_unknown_session() {
        let registry = SessionRegistry::default();
        assert!(registry.state("nonexistent").is_none());
    }
}
