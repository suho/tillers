use crate::models::keyboard_mapping::{
    ActionParameters, ActionType, KeyboardMapping, KeyboardMappingError, KeyboardMappingSet,
    ModifierKey, ShortcutCombination,
};
use crate::{Result, TilleRSError};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

/// Event emitted when a shortcut is triggered
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardEvent {
    pub mapping_id: Uuid,
    pub action: ActionType,
    pub parameters: ActionParameters,
    pub global: bool,
}

/// Metrics for keyboard handler operations
#[derive(Debug, Default, Clone)]
pub struct KeyboardHandlerMetrics {
    pub registered_mappings: usize,
    pub triggered_events: u64,
    pub conflicts_prevented: u64,
    pub modifier_migrations: usize,
}

/// Service responsible for registering and dispatching global keyboard shortcuts
pub struct KeyboardHandler {
    mappings: Arc<RwLock<KeyboardMappingSet>>,
    metrics: Arc<RwLock<KeyboardHandlerMetrics>>,
    reserved_shortcuts: HashSet<ShortcutCombination>,
}

impl KeyboardHandler {
    /// Create a new keyboard handler. Optional reserved shortcuts can be provided to avoid conflicts
    /// with macOS system shortcuts.
    pub fn new(reserved_shortcuts: HashSet<ShortcutCombination>) -> Self {
        Self {
            mappings: Arc::new(RwLock::new(KeyboardMappingSet::new())),
            metrics: Arc::new(RwLock::new(KeyboardHandlerMetrics::default())),
            reserved_shortcuts,
        }
    }

    /// Register a keyboard mapping, enforcing Option (⌥) as the default modifier and preventing conflicts.
    pub async fn register_mapping(&self, mut mapping: KeyboardMapping) -> Result<Uuid> {
        let migrated = self.enforce_option_modifier(&mut mapping);

        if self
            .reserved_shortcuts
            .contains(&mapping.shortcut_combination)
        {
            warn!(
                shortcut = %mapping.shortcut_combination,
                "Shortcut conflicts with macOS reserved combination"
            );
            self.metrics.write().await.conflicts_prevented += 1;
            return Err(TilleRSError::ValidationError(format!(
                "Shortcut {} is reserved by the system",
                mapping.shortcut_combination
            ))
            .into());
        }

        mapping
            .validate()
            .map_err(|err| convert_mapping_error(&err))?;

        let mut mappings = self.mappings.write().await;
        mappings
            .add_mapping(mapping.clone())
            .map_err(|err| convert_mapping_error(&err))?;

        let mut metrics = self.metrics.write().await;
        metrics.registered_mappings = mappings.mappings.len();
        if migrated {
            metrics.modifier_migrations += 1;
        }
        debug!(
            id = %mapping.id,
            shortcut = %mapping.shortcut_combination,
            "Registered keyboard mapping"
        );

        Ok(mapping.id)
    }

    /// Remove a mapping by identifier
    pub async fn unregister_mapping(&self, id: Uuid) -> bool {
        let mut mappings = self.mappings.write().await;
        let removed = mappings.remove_mapping(id);
        if removed {
            self.metrics.write().await.registered_mappings = mappings.mappings.len();
        }
        removed
    }

    /// Trigger a shortcut manually. Returns the action to perform when a mapping matches.
    pub async fn handle_shortcut(
        &self,
        combination: &ShortcutCombination,
    ) -> Result<Option<KeyboardEvent>> {
        let mappings = self.mappings.read().await;
        let mapping = mappings.find_by_shortcut(combination).cloned();
        drop(mappings);

        if let Some(mapping) = mapping {
            self.metrics.write().await.triggered_events += 1;
            Ok(Some(KeyboardEvent {
                mapping_id: mapping.id,
                action: mapping.action_type,
                parameters: mapping.parameters,
                global: mapping.global_scope,
            }))
        } else {
            Ok(None)
        }
    }

    /// Snapshot all mappings for UI or persistence layers
    pub async fn mappings(&self) -> Vec<KeyboardMapping> {
        self.mappings.read().await.mappings.clone()
    }

    /// Retrieve current metrics
    pub async fn metrics(&self) -> KeyboardHandlerMetrics {
        self.metrics.read().await.clone()
    }

    fn enforce_option_modifier(&self, mapping: &mut KeyboardMapping) -> bool {
        let mut combination = mapping.shortcut_combination.clone();
        let mut modified = false;

        if combination.migrate_command_to_option() {
            modified = true;
        }

        if !combination.has_option_modifier() {
            combination.modifiers.push(ModifierKey::Option);
            combination.modifiers.sort_by_key(|m| format!("{:?}", m));
            combination.modifiers.dedup();
            modified = true;
        }

        if modified {
            mapping.shortcut_combination =
                ShortcutCombination::new(combination.modifiers.clone(), combination.key.clone());
        }
        modified
    }
}

fn convert_mapping_error(error: &KeyboardMappingError) -> anyhow::Error {
    TilleRSError::ValidationError(error.to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::keyboard_mapping::{ActionParameters, ActionType, Key};

    fn default_handler() -> KeyboardHandler {
        KeyboardHandler::new(HashSet::new())
    }

    fn make_mapping(combination: ShortcutCombination) -> KeyboardMapping {
        KeyboardMapping {
            id: Uuid::new_v4(),
            name: "Switch".to_string(),
            shortcut_combination: combination,
            action_type: ActionType::SwitchWorkspace,
            parameters: ActionParameters::WorkspaceId(Uuid::new_v4()),
            enabled: true,
            global_scope: true,
            description: None,
        }
    }

    #[tokio::test]
    async fn register_mapping_enforces_option_modifier() {
        let handler = default_handler();
        let combination = ShortcutCombination::new(vec![ModifierKey::Command], Key::Number(1));
        let mapping = make_mapping(combination);

        handler.register_mapping(mapping).await.unwrap();
        let stored = handler.mappings().await;
        assert_eq!(stored.len(), 1);
        assert!(stored[0].shortcut_combination.has_option_modifier());
        assert!(!stored[0]
            .shortcut_combination
            .modifiers
            .contains(&ModifierKey::Command));
    }

    #[tokio::test]
    async fn register_fails_for_reserved_shortcut() {
        let combination = ShortcutCombination::new(vec![ModifierKey::Option], Key::Number(1));
        let mut reserved = HashSet::new();
        reserved.insert(combination.clone());

        let handler = KeyboardHandler::new(reserved);
        let mapping = make_mapping(combination);
        let error = handler.register_mapping(mapping).await.unwrap_err();
        assert!(error.to_string().contains("reserved"));
    }

    #[tokio::test]
    async fn handle_shortcut_returns_event() {
        let handler = default_handler();
        let combination = ShortcutCombination::new(vec![ModifierKey::Option], Key::Number(2));
        let mapping = make_mapping(combination.clone());
        handler.register_mapping(mapping).await.unwrap();

        let event = handler
            .handle_shortcut(&combination)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.action, ActionType::SwitchWorkspace);
        assert!(event.global);
    }
}
