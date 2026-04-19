use limur::{ShortcutsRegistry, shortcuts::ShortcutsManager, widgets::builder::BuildContext};

pub trait Window<App, Event = ()> {
    fn on_event(&mut self, _app: &mut App, _event: &Event) {}

    fn on_init(&mut self, _shortcuts_registry: &mut ShortcutsRegistry) {}

    fn on_shortcut(&mut self, _shortcuts_manager: &ShortcutsManager) {}

    fn build(&mut self, app: &mut App, ctx: &mut BuildContext);
}
