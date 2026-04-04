use std::{
    collections::hash_map::Entry,
    time::{Duration, Instant},
};

use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::{SmallVec, smallvec};

use crate::{
    io::UserInput,
    keyboard::{KeyCode, KeyModifiers},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct KeyBinding {
    modifiers: KeyModifiers,
    key: KeyCode,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ShortcutConfig {
    sequence: Vec<KeyBinding>,
    repeat: bool,
}

fn remove_modifiers(sequence: &[KeyBinding], modifiers: KeyModifiers) -> Vec<KeyBinding> {
    if modifiers.is_empty() {
        sequence.to_vec()
    } else {
        sequence
            .iter()
            .map(|binding| KeyBinding {
                modifiers: binding.modifiers & !modifiers,
                key: binding.key,
            })
            .collect()
    }
}

impl KeyBinding {
    pub fn new(key: KeyCode) -> Self {
        Self {
            modifiers: KeyModifiers::empty(),
            key,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.modifiers |= KeyModifiers::CONTROL;

        self
    }

    pub fn with_shift(mut self) -> Self {
        self.modifiers |= KeyModifiers::SHIFT;

        self
    }

    pub fn with_alt(mut self) -> Self {
        self.modifiers |= KeyModifiers::ALT;

        self
    }

    pub fn with_super(mut self) -> Self {
        self.modifiers |= KeyModifiers::SUPER;

        self
    }
}

pub const SHORTCUTS_ROOT_SCOPE_ID: ShortcutScopeId = ShortcutScopeId("root");

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ShortcutScopeId(pub &'static str);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ShortcutModifierId(pub &'static str);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct ShortcutId(pub &'static str);

#[derive(Default)]
pub struct ShortcutsRegistry {
    scopes: FxHashMap<ShortcutScopeId, ShortcutScope>,
}

#[derive(Default, Clone)]
pub struct ShortcutScope {
    shortcuts: FxHashMap<ShortcutId, ShortcutConfig>,
    modifiers: FxHashMap<ShortcutModifierId, KeyModifiers>,
}

impl ShortcutsRegistry {
    pub fn scope<T: Into<ShortcutScopeId>>(&mut self, key: T) -> &mut ShortcutScope {
        let key = key.into();

        self.scopes
            .entry(key)
            .or_insert_with(ShortcutScope::default)
    }

    pub fn merge_with(&mut self, shortcuts_registry: &ShortcutsRegistry) {
        for (id, scope) in &shortcuts_registry.scopes {
            self.scopes.insert(*id, scope.clone());
        }
    }
}

impl ShortcutScope {
    pub fn add<T: Into<ShortcutId>>(&mut self, id: T, shortcut: KeyBinding) -> &mut ShortcutScope {
        let key = id.into();

        let config = ShortcutConfig {
            sequence: vec![shortcut],
            repeat: false,
        };

        match self.shortcuts.entry(key) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = config;
            }
            Entry::Vacant(_) => {
                self.shortcuts.insert(key, config);
            }
        }

        self
    }

    pub fn add_repeat<T: Into<ShortcutId>>(
        &mut self,
        id: T,
        shortcut: KeyBinding,
    ) -> &mut ShortcutScope {
        let key = id.into();

        let config = ShortcutConfig {
            sequence: vec![shortcut],
            repeat: true,
        };

        match self.shortcuts.entry(key) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = config;
            }
            Entry::Vacant(_) => {
                self.shortcuts.insert(key, config);
            }
        }

        self
    }

    pub fn add_sequence<T: Into<ShortcutId>>(
        &mut self,
        id: T,
        sequence: &[KeyBinding],
    ) -> &mut ShortcutScope {
        let key = id.into();

        let config = ShortcutConfig {
            sequence: Vec::from(sequence),
            repeat: true,
        };

        match self.shortcuts.entry(key) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = config;
            }
            Entry::Vacant(_) => {
                self.shortcuts.insert(key, config);
            }
        }

        self
    }

    pub fn add_modifier<T: Into<ShortcutModifierId>>(
        &mut self,
        id: T,
        modifier: KeyModifiers,
    ) -> &mut ShortcutScope {
        let key = id.into();

        match self.modifiers.entry(key) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = modifier;
            }
            Entry::Vacant(_) => {
                self.modifiers.insert(key, modifier);
            }
        }

        self
    }
}

pub struct ShortcutsManager {
    last_sequence: Vec<KeyBinding>,
    last_found_candidate: Option<Instant>,
    chord_timeout: Duration,
    candidates: u32,

    pub(crate) current_path: SmallVec<[ShortcutScopeId; 4]>,
    pub(crate) active_path: SmallVec<[ShortcutScopeId; 4]>,
    pub(crate) branches: SmallVec<[SmallVec<[ShortcutScopeId; 4]>; 4]>,
    pub(crate) depth_before_pop: usize,
    pub(crate) current_active_shortcuts: FxHashMap<SmallVec<[ShortcutScopeId; 4]>, ShortcutId>,
    pub(crate) next_active_shortcuts: FxHashMap<SmallVec<[ShortcutScopeId; 4]>, ShortcutId>,
    pub(crate) current_active_modifiers:
        FxHashMap<SmallVec<[ShortcutScopeId; 4]>, FxHashSet<ShortcutModifierId>>,
    pub(crate) next_active_modifiers:
        FxHashMap<SmallVec<[ShortcutScopeId; 4]>, FxHashSet<ShortcutModifierId>>,
}

impl Default for ShortcutsManager {
    fn default() -> Self {
        Self {
            chord_timeout: Duration::from_secs(2),
            last_found_candidate: Default::default(),
            current_path: smallvec![SHORTCUTS_ROOT_SCOPE_ID],
            active_path: SmallVec::new(),
            branches: SmallVec::new(),
            depth_before_pop: 1,
            last_sequence: Default::default(),
            current_active_shortcuts: Default::default(),
            next_active_shortcuts: Default::default(),
            current_active_modifiers: Default::default(),
            next_active_modifiers: Default::default(),
            candidates: 0,
        }
    }
}

impl ShortcutsManager {
    pub fn is_shortcut<T: Into<ShortcutId>>(&self, id: T) -> bool {
        if let Some(active_shortcut_id) = self.current_active_shortcuts.get(&self.current_path) {
            *active_shortcut_id == id.into()
        } else {
            false
        }
    }

    pub(crate) fn active_shortcut_id(&self) -> Option<ShortcutId> {
        self.current_active_shortcuts
            .get(&self.current_path)
            .copied()
    }

    pub fn has_modifier<T: Into<ShortcutModifierId>>(&self, id: T) -> bool {
        if let Some(active_modifiers) = self.current_active_modifiers.get(&self.current_path) {
            active_modifiers.contains(&id.into())
        } else {
            false
        }
    }

    #[inline]
    pub(crate) fn push_scope<T: Into<ShortcutScopeId>>(&mut self, scope: T) {
        self.current_path.push(scope.into());
        self.depth_before_pop = self.current_path.len();
    }

    #[inline]
    pub(crate) fn pop_scope(&mut self, user_input: &UserInput, registry: &ShortcutsRegistry) {
        if self.current_path.len() == self.depth_before_pop {
            self.branches.push(self.current_path.clone());
            let shortcut_id = self.resolve_shortcut_for_current_path(user_input, registry);

            if let Some(shortcut_id) = shortcut_id {
                self.next_active_shortcuts
                    .insert(self.active_path.clone(), shortcut_id);

                self.next_active_shortcuts
                    .insert(self.current_path.clone(), shortcut_id);
            }
        }

        self.current_path.pop();
    }

    pub(crate) fn resolve_shortcut_for_current_path(
        &mut self,
        user_input: &UserInput,
        registry: &ShortcutsRegistry,
    ) -> Option<ShortcutId> {
        let mut shortcut_id = None;

        for (modifiers, _) in user_input.key_pressed.iter() {
            let modifiers = modifiers.unwrap_or_default();

            let (candidates, resolved_shortcut_id, active_path) = Self::resolve(
                registry,
                modifiers,
                &self.current_path,
                &mut self.next_active_modifiers,
                &self.last_sequence,
                false,
            );

            shortcut_id = resolved_shortcut_id;

            self.active_path = active_path;
            self.candidates += candidates;
        }

        if shortcut_id.is_none() {
            for (modifiers, key) in user_input.key_pressed_repeat.iter() {
                let modifiers = modifiers.unwrap_or_default();

                if let Some(key) = key {
                    let (_, resolved_shortcut_id, active_path) = Self::resolve(
                        registry,
                        modifiers,
                        &self.current_path,
                        &mut self.next_active_modifiers,
                        &[KeyBinding {
                            modifiers,
                            key: *key,
                        }],
                        true,
                    );

                    shortcut_id = resolved_shortcut_id;

                    self.active_path = active_path;
                }
            }
        }

        shortcut_id
    }

    pub(crate) fn init_cycle(&mut self, user_input: &UserInput) {
        self.current_active_shortcuts = std::mem::take(&mut self.next_active_shortcuts);
        self.current_active_modifiers = std::mem::take(&mut self.next_active_modifiers);

        self.branches.clear();
        self.next_active_shortcuts.clear();
        self.next_active_modifiers.clear();

        for (modifiers, key) in user_input.key_pressed.iter() {
            let modifiers = modifiers.unwrap_or_default();

            self.candidates = 0;

            if let Some(time) = self.last_found_candidate {
                let duration = time.elapsed();

                if duration > self.chord_timeout {
                    self.last_sequence.clear();
                }
            } else {
                self.last_sequence.clear();
            }

            if let Some(key) = key {
                self.last_sequence.push(KeyBinding {
                    modifiers,
                    key: *key,
                });
            }
        }
    }

    pub(crate) fn finalize_cycle(&mut self) {
        let has_not_active_shortcut = self.next_active_shortcuts.is_empty();

        if has_not_active_shortcut && self.candidates == 0 {
            self.last_sequence.clear();
            self.last_found_candidate = None;
        } else if has_not_active_shortcut && self.candidates > 0 {
            self.last_found_candidate = Some(Instant::now());
        } else {
            self.last_sequence.clear();
        }
    }

    pub(crate) fn resolve(
        registry: &ShortcutsRegistry,
        modifiers: KeyModifiers,
        scopes: &SmallVec<[ShortcutScopeId; 4]>,
        shortucts_modifiers: &mut FxHashMap<
            SmallVec<[ShortcutScopeId; 4]>,
            FxHashSet<ShortcutModifierId>,
        >,
        chords: &[KeyBinding],
        repeat: bool,
    ) -> (u32, Option<ShortcutId>, SmallVec<[ShortcutScopeId; 4]>) {
        let mut shortcut_id = None;
        let mut candidates = 0;
        let mut resolved_modifiers = KeyModifiers::empty();
        let mut active_modifiers = FxHashSet::default();

        // Resolve modifier
        for scope in scopes.iter().rev() {
            let scope = registry.scopes.get(scope);

            if let Some(scope) = scope {
                for (id, scope_modifiers) in scope.modifiers.iter() {
                    if *scope_modifiers & modifiers == *scope_modifiers {
                        active_modifiers.insert(*id);
                        resolved_modifiers |= *scope_modifiers;
                    }
                }
            }
        }


        let mut active_path = scopes.clone();

        // Resolve keybinding
        for scope_id in scopes.iter().rev() {
            let scope = registry.scopes.get(scope_id);

            if let Some(scope) = scope {
                let mut found_in_scope = false;

                for (id, key_bindings) in scope.shortcuts.iter() {
                    if repeat && repeat != key_bindings.repeat {
                        continue;
                    }

                    // FIRST: Try exact match with all modifiers
                    if key_bindings.sequence == chords {
                        shortcut_id = Some(*id);
                        found_in_scope = true;
                        break;
                    }

                    // Check for chord candidate (exact modifiers)
                    if key_bindings.sequence.starts_with(chords) {
                        candidates += 1;
                    }
                }

                // SECOND: If no exact match found, try with modifiers removed
                if !found_in_scope {
                    let chords_stripped = remove_modifiers(chords, resolved_modifiers);

                    // Only check if we actually removed something
                    if chords_stripped != chords {
                        for (id, key_bindings) in scope.shortcuts.iter() {
                            if repeat && repeat != key_bindings.repeat {
                                continue;
                            }

                            if key_bindings.sequence == chords_stripped {
                                shortcut_id = Some(*id);
                                break;
                            }

                            if key_bindings.sequence.starts_with(&chords_stripped) {
                                candidates += 1;
                            }
                        }
                    }
                }

                if shortcut_id.is_none() {
                    active_path.pop();
                }
            }

            if shortcut_id.is_some() {
                break;
            }
        }

        shortucts_modifiers.insert(active_path.clone(), active_modifiers);

        (candidates, shortcut_id, active_path)
    }

    pub(crate) fn reset(&mut self) {
        self.current_active_shortcuts.clear();
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     impl Into<ShortcutId> for &'static str {
//         fn into(self) -> ShortcutId {
//             ShortcutId(self)
//         }
//     }

//     impl Into<ShortcutModifierId> for &'static str {
//         fn into(self) -> ShortcutModifierId {
//             ShortcutModifierId(self)
//         }
//     }

//     impl Into<ShortcutScopeId> for &'static str {
//         fn into(self) -> ShortcutScopeId {
//             ShortcutScopeId(self)
//         }
//     }

//     fn mock_registry() -> ShortcutRegistry {
//         let mut registry = ShortcutRegistry::default();
//         registry
//             .scope("scope1")
//             .add("shorcut1", KeyBinding::new(KeyCode::KeyA).with_ctrl())
//             .add(
//                 "shorcut2",
//                 KeyBinding::new(KeyCode::KeyB).with_ctrl().with_shift(),
//             )
//             .add("shorcut3", KeyBinding::new(KeyCode::KeyC))
//             .add("shorcut4", KeyBinding::new(KeyCode::KeyB))
//             .add_repeat("shortcut_repeat", KeyBinding::new(KeyCode::ArrowLeft))
//             .add_modifier("modifier1", KeyModifiers::alt())
//             .add_modifier("modifier2", KeyModifiers::ctrl());

//         registry
//             .scope("scope2")
//             .add(
//                 "shorcut5",
//                 KeyBinding::new(KeyCode::KeyB).with_ctrl().with_shift(),
//             )
//             .add("shorcut6", KeyBinding::new(KeyCode::KeyD))
//             .add("shorcut7", KeyBinding::new(KeyCode::KeyE))
//             .add_sequence(
//                 "sequence1",
//                 &[
//                     KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                     KeyBinding::new(KeyCode::KeyG).with_ctrl(),
//                 ],
//             )
//             .add_sequence(
//                 "sequence2",
//                 &[
//                     KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                     KeyBinding::new(KeyCode::KeyI).with_ctrl(),
//                 ],
//             )
//             .add_sequence(
//                 "sequence3",
//                 &[
//                     KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                     KeyBinding::new(KeyCode::KeyK).with_ctrl(),
//                 ],
//             )
//             .add_modifier("modifier3", KeyModifiers::alt())
//             .add_modifier("modifier4", KeyModifiers::shift());

//         registry
//     }

//     #[test]
//     fn test_simple_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyB)],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("shorcut4".into()));
//         assert_eq!(modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_repeat_resolve_fail_when_non_repeatable() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyB)],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("shorcut4".into()));
//         assert_eq!(modifiers.is_empty(), true);

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyB)],
//             true,
//         );

//         assert_eq!(shortcut_id, None);
//         assert_eq!(modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_repeat_resolve_success_when_repeatable() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::ArrowLeft)],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("shortcut_repeat".into()));
//         assert_eq!(modifiers.is_empty(), true);

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::ArrowLeft)],
//             true,
//         );

//         assert_eq!(shortcut_id, Some("shortcut_repeat".into()));
//         assert_eq!(modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_modifier_resolve_mix() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, _) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt().with_ctrl(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[],
//             false,
//         );
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier2".into()), true);
//     }

//     #[test]
//     fn test_modifier_resolve_mix_with_shortcut() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt().with_ctrl(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyC).with_ctrl().with_alt()],
//             false,
//         );

//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier2".into()), true);
//         assert_eq!(shortcut_id, Some("shorcut3".into()));
//     }

//     #[test]
//     fn test_modifier_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, _) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[],
//             false,
//         );
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier3".into()), false);

//         modifiers.clear();

//         let (_, _) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt(),
//             &["scope2".into()],
//             &mut modifiers,
//             &[],
//             false,
//         );
//         assert_eq!(modifiers.contains(&"modifier1".into()), false);
//         assert_eq!(modifiers.contains(&"modifier3".into()), true);

//         modifiers.clear();

//         let (_, _) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt(),
//             &["scope1".into(), "scope2".into()],
//             &mut modifiers,
//             &[],
//             false,
//         );
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier3".into()), true);
//     }

//     #[test]
//     fn test_modifiers_intersection_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, _) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::ctrl().with_alt(),
//             &["scope1".into()],
//             &mut modifiers,
//             &[],
//             false,
//         );
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier2".into()), true);

//         modifiers.clear();
//     }

//     #[test]
//     #[ignore]
//     fn test_sequence_candidates_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (candidates, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope2".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyF).with_ctrl()],
//             false,
//         );

//         assert_eq!(candidates, 3);
//         assert_eq!(shortcut_id, None);
//         assert_eq!(modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_sequence_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope2".into()],
//             &mut modifiers,
//             &[
//                 KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                 KeyBinding::new(KeyCode::KeyG).with_ctrl(),
//             ],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("sequence1".into()));
//         assert_eq!(modifiers.is_empty(), true);

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope2".into()],
//             &mut modifiers,
//             &[
//                 KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                 KeyBinding::new(KeyCode::KeyI).with_ctrl(),
//             ],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("sequence2".into()));
//         assert_eq!(modifiers.is_empty(), true);

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::empty(),
//             &["scope2".into()],
//             &mut modifiers,
//             &[
//                 KeyBinding::new(KeyCode::KeyF).with_ctrl(),
//                 KeyBinding::new(KeyCode::KeyK).with_ctrl(),
//             ],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("sequence3".into()));
//         assert_eq!(modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_scopes_resolve() {
//         let registry = mock_registry();
//         let mut modifiers = FxHashSet::default();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt(),
//             &["scope1".into(), "scope2".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyB).with_ctrl().with_shift()],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("shorcut5".into()));
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier3".into()), true);

//         modifiers.clear();

//         let (_, shortcut_id) = ShortcutManager::resolve(
//             &registry,
//             KeyModifiers::alt(),
//             &["scope2".into(), "scope1".into()],
//             &mut modifiers,
//             &[KeyBinding::new(KeyCode::KeyB).with_ctrl().with_shift()],
//             false,
//         );

//         assert_eq!(shortcut_id, Some("shorcut2".into()));
//         assert_eq!(modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(modifiers.contains(&"modifier3".into()), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_mix_with_shortcut() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope1");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::alt().with_ctrl(),
//             Some(KeyCode::KeyC),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.modifiers.contains(&"modifier1".into()), true);
//         assert_eq!(manager.modifiers.contains(&"modifier2".into()), true);
//         assert_eq!(manager.shortcut_id, Some("shorcut3".into()));
//     }

//     #[test]
//     fn test_on_key_binding_activate_simple() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope1");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::empty(),
//             Some(KeyCode::KeyB),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, Some("shorcut4".into()));
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_empty_scopes() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::empty(),
//             Some(KeyCode::KeyB),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_repeat() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope1");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::empty(),
//             Some(KeyCode::KeyB),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, Some("shorcut4".into()));
//         assert_eq!(manager.modifiers.is_empty(), true);

//         manager.reset();
//         manager.push_scope("scope1");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::empty(),
//             Some(KeyCode::KeyB),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, Some("shorcut4".into()));
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_sequence() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyF),
//             false,
//         );

//         assert_eq!(
//             manager.last_sequence,
//             &[KeyBinding::new(KeyCode::KeyF).with_ctrl()]
//         );
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);

//         manager.reset();
//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyG),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, Some("sequence1".into()));
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_sequence_timeout() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyF),
//             false,
//         );

//         assert_eq!(
//             manager.last_sequence,
//             &[KeyBinding::new(KeyCode::KeyF).with_ctrl()]
//         );
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);

//         manager.reset();
//         manager.last_found_candidate = None;
//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyG),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }

//     #[test]
//     fn test_on_key_binding_activate_sequence_repeat_after_timeout() {
//         let registry = mock_registry();
//         let mut manager = ShortcutManager::default();

//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyF),
//             false,
//         );

//         assert_eq!(
//             manager.last_sequence,
//             &[KeyBinding::new(KeyCode::KeyF).with_ctrl()]
//         );
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);

//         manager.reset();
//         manager.last_found_candidate = None;

//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyF),
//             false,
//         );

//         assert_eq!(
//             manager.last_sequence,
//             &[KeyBinding::new(KeyCode::KeyF).with_ctrl()]
//         );
//         assert_eq!(manager.shortcut_id, None);
//         assert_eq!(manager.modifiers.is_empty(), true);

//         manager.reset();
//         manager.push_scope("scope2");
//         manager.on_key_binding_activate(
//             &registry,
//             KeyModifiers::ctrl(),
//             Some(KeyCode::KeyG),
//             false,
//         );

//         assert_eq!(manager.last_sequence, &[]);
//         assert_eq!(manager.shortcut_id, Some("sequence1".into()));
//         assert_eq!(manager.modifiers.is_empty(), true);
//     }
// }
