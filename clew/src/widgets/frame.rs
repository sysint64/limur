use smallvec::SmallVec;

use crate::{
    Clip, Constraints, EdgeInsets, Size, WidgetId, WidgetRef,
    layout::{ContainerKind, LayoutCommand},
};

use super::{BuildContext, builder::Layout};

pub struct FrameBuilder {
    pub(crate) id: WidgetId,
    pub(crate) size: Size,
    pub(crate) constraints: Constraints,
    pub(crate) zindex: i32,
    pub(crate) padding: EdgeInsets,
    pub(crate) margin: EdgeInsets,
    pub(crate) backgrounds: SmallVec<[WidgetRef; 8]>,
    pub(crate) foregrounds: SmallVec<[WidgetRef; 8]>,
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
    pub(crate) clip: Clip,
    pub(crate) ignore_pointer: bool,
    pub(crate) flags: FrameBuilderFlags,
}

impl FrameBuilder {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: WidgetId::auto(),
            size: Default::default(),
            constraints: Default::default(),
            zindex: Default::default(),
            padding: Default::default(),
            margin: Default::default(),
            backgrounds: Default::default(),
            foregrounds: Default::default(),
            offset_x: Default::default(),
            offset_y: Default::default(),
            clip: Clip::None,
            ignore_pointer: false,
            flags: FrameBuilderFlags::empty(),
        }
    }

    pub fn take_layout(&mut self) -> Layout {
        self.flags.remove(FrameBuilderFlags::SIZE);
        self.flags.remove(FrameBuilderFlags::CONSTRAINTS);

        Layout {
            size: self.size,
            constraints: self.constraints,
        }
    }

    pub fn build<F, T>(&mut self, context: &mut BuildContext, callback: F) -> T
    where
        F: FnOnce(&mut BuildContext) -> T,
    {
        let has_offset = self.flags.contains(FrameBuilderFlags::OFFSET);

        if has_offset {
            context.push_layout_command(LayoutCommand::BeginOffset {
                offset_x: self.offset_x,
                offset_y: self.offset_y,
            });
        }

        let needs_container = self.flags.intersects(
            FrameBuilderFlags::SIZE
                .union(FrameBuilderFlags::CONSTRAINTS)
                .union(FrameBuilderFlags::ZINDEX)
                .union(FrameBuilderFlags::PADDING)
                .union(FrameBuilderFlags::MARGIN)
                .union(FrameBuilderFlags::BACKGROUNDS)
                .union(FrameBuilderFlags::FOREGROUNDS)
                .union(FrameBuilderFlags::CLIP),
        );

        let value;

        let last_ignore_pointer = context.ignore_pointer;
        context.ignore_pointer = self.ignore_pointer && context.ignore_pointer;

        if needs_container {
            let (backgrounds, foregrounds) = context.resolve_decorators(self);

            context.push_layout_command(LayoutCommand::BeginContainer {
                backgrounds,
                foregrounds,
                zindex: self.zindex,
                padding: self.padding,
                margin: self.margin,
                kind: ContainerKind::Passthrough,
                size: self.size,
                constraints: self.constraints,
                clip: self.clip,
            });

            value = context.scope(self.id, callback);

            context.push_layout_command(LayoutCommand::EndContainer);
        } else {
            value = context.scope(self.id, callback);
        }

        if has_offset {
            context.push_layout_command(LayoutCommand::EndOffset);
        }

        context.ignore_pointer = last_ignore_pointer;

        value
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct FrameBuilderFlags: u16 {
        const ID = 1 << 0;
        const SIZE = 1 << 1;
        const CONSTRAINTS = 1 << 2;
        const ZINDEX = 1 << 3;
        const PADDING = 1 << 4;
        const MARGIN = 1 << 5;
        const BACKGROUNDS = 1 << 6;
        const FOREGROUNDS = 1 << 7;
        const OFFSET = 1 << 8;
        const CLIP = 1 << 9;
        const IGNORE_POINTER = 1 << 10;
    }
}

impl Default for FrameBuilder {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}
