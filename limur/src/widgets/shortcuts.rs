use crate::ShortcutScopeId;

use super::BuildContext;

pub struct ShortcutsBuilder {
    active: bool,
    scope_id: ShortcutScopeId,
}

impl ShortcutsBuilder {
    pub fn active(mut self, value: bool) -> Self {
        self.active = value;

        self
    }

    pub fn build<F>(self, ctx: &mut BuildContext, callback: F)
    where
        F: FnOnce(&mut BuildContext),
    {
        if self.active {
            ctx.shortcuts_manager.push_scope(self.scope_id);
        }

        callback(ctx);

        if self.active {
            ctx.shortcuts_manager
                .pop_scope(ctx.input, ctx.shortcuts_registry);
        }
    }
}

pub fn shortcut_scope<T: Into<ShortcutScopeId>>(scope_id: T) -> ShortcutsBuilder {
    ShortcutsBuilder {
        active: true,
        scope_id: scope_id.into(),
    }
}
