extern crate self as clew;

pub mod animation;
pub mod assets;
mod foundation;
pub mod identifiable;
pub mod interaction;
pub mod io;
pub mod keyboard;
mod layout;
pub mod lifecycle;
pub mod render;
pub mod shortcuts;
pub mod state;
pub mod text;
pub mod text_data;
pub mod text_history;
mod widget_id;
pub mod widgets;

pub use animation::*;
pub use foundation::*;
pub use interaction::WidgetInteractionState;
pub use render::{Renderer, layout_and_render};
pub use shortcuts::*;
pub use text_data::*;
pub use widget_id::*;
pub use widgets::*;

pub mod prelude {
    pub use crate::animation::Animation;
    pub use crate::foundation::Value;
    pub use crate::identifiable::Identifiable;
    pub use crate::state::WidgetState;
    pub use crate::widgets::builder::{Resolve, WidgetBuilder};
    pub use crate::widgets::stateful::StatefulWidgetBuilder;
}
