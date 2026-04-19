use crate::{Constraints, Size, SizeConstraint, impl_size_methods, layout::LayoutCommand};

use super::builder::BuildContext;

pub struct GapBuilder {
    size: Size,
    constraints: Constraints,
}

impl GapBuilder {
    impl_size_methods!();

    pub fn build(&self, context: &mut BuildContext) {
        context.push_layout_command(LayoutCommand::Spacer {
            size: self.size,
            constraints: self.constraints,
        });
    }
}

pub fn gap() -> GapBuilder {
    GapBuilder {
        size: Size::new(SizeConstraint::Fixed(0.), SizeConstraint::Fixed(0.)),
        constraints: Constraints::default(),
    }
}
