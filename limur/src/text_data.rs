use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    WidgetId,
    editable_text::EditableTextDelta,
    text::{TextId, TextsResources},
};

pub struct TextData {
    pub(crate) buffer: cosmic_text::Buffer,

    // This potentially can bloat as we never clear these data,
    // but realistically is should be fine as usually TextData not used by many widgets.
    pub(crate) replace_buffer: FxHashSet<WidgetId>,
    pub(crate) dirty: FxHashSet<WidgetId>,
    pub(crate) text_id: FxHashMap<WidgetId, TextId>,
}

impl Default for TextData {
    fn default() -> Self {
        Self::new()
    }
}

impl TextData {
    pub fn new() -> Self {
        let buffer = cosmic_text::Buffer::new_empty(
            // Just a placeholder value, could be actually anything except 0
            // as 0 line height will lead to panic.
            cosmic_text::Metrics::new(12., 12.),
        );

        Self {
            buffer,
            dirty: FxHashSet::default(),
            replace_buffer: FxHashSet::default(),
            text_id: FxHashMap::default(),
        }
    }

    pub fn from(text: &str) -> Self {
        let mut data = Self::new();
        data.set_text(text);

        data
    }

    pub(crate) fn text_id(&self, id: WidgetId) -> Option<TextId> {
        self.text_id.get(&id).cloned()
    }

    pub(crate) fn set_text_id(&mut self, id: WidgetId, text_id: TextId) {
        self.text_id.insert(id, text_id);
    }

    pub fn set_text(&mut self, data: &str) {
        self.buffer.lines.clear();

        for (range, ending) in cosmic_text::LineIter::new(data) {
            self.buffer.lines.push(cosmic_text::BufferLine::new(
                &data[range],
                ending,
                cosmic_text::AttrsList::new(&cosmic_text::Attrs::new()),
                cosmic_text::Shaping::Advanced,
            ));
        }

        if self.buffer.lines.is_empty() {
            self.buffer.lines.push(cosmic_text::BufferLine::new(
                "",
                cosmic_text::LineEnding::default(),
                cosmic_text::AttrsList::new(&cosmic_text::Attrs::new()),
                cosmic_text::Shaping::Advanced,
            ));
        }

        for k in self.text_id.keys() {
            self.replace_buffer.insert(*k);
        }
    }

    pub async fn set_text_async(&mut self, data: &str) {
        self.buffer.lines.clear();

        let mut line_count = 0;
        for (range, ending) in cosmic_text::LineIter::new(data) {
            self.buffer.lines.push(cosmic_text::BufferLine::new(
                &data[range],
                ending,
                cosmic_text::AttrsList::new(&cosmic_text::Attrs::new()),
                cosmic_text::Shaping::Advanced,
            ));

            // Yield periodically
            line_count += 1;
            if line_count % 1000 == 0 {
                std::future::ready(()).await;
            }
        }

        if self.buffer.lines.is_empty() {
            self.buffer.lines.push(cosmic_text::BufferLine::new(
                "",
                cosmic_text::LineEnding::default(),
                cosmic_text::AttrsList::new(&cosmic_text::Attrs::new()),
                cosmic_text::Shaping::Advanced,
            ));
        }

        for k in self.text_id.keys() {
            self.replace_buffer.insert(*k);
        }
    }

    pub fn get_text(&self) -> String {
        let mut full_text = String::new();

        for line in self.buffer.lines.iter() {
            full_text.push_str(line.text());
            full_text.push('\n');
        }

        if full_text.ends_with('\n') {
            full_text.pop();
        }

        full_text
    }

    pub async fn get_text_async(&self) -> String {
        let mut full_text = String::new();

        for (i, line) in self.buffer.lines.iter().enumerate() {
            full_text.push_str(line.text());
            full_text.push('\n');

            if i % 1000 == 0 {
                std::future::ready(()).await;
            }
        }

        if full_text.ends_with('\n') {
            full_text.pop();
        }

        full_text
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.lines.is_empty()
            || (self.buffer.lines.len() == 1 && self.buffer.lines[0].text() == "")
    }

    #[inline]
    pub fn clear(&mut self) {
        self.set_text("");
    }

    pub(crate) fn apply_delta(
        &mut self,
        text_resources: &mut TextsResources,
        id: WidgetId,
        delta: &EditableTextDelta,
    ) {
        match delta {
            EditableTextDelta::Undo(delta) => delta.undo_to_buffer(&mut self.buffer),
            EditableTextDelta::Apply(delta) => delta.apply_to_buffer(&mut self.buffer),
        }

        for (key, text_id) in self.text_id.iter() {
            if *key == id {
                continue;
            }

            self.dirty.insert(*key);
            text_resources
                .get_mut(*text_id)
                .with_buffer_mut(|buffer| match delta {
                    EditableTextDelta::Undo(delta) => delta.undo_to_buffer(buffer),
                    EditableTextDelta::Apply(delta) => delta.apply_to_buffer(buffer),
                });
        }
    }

    pub(crate) fn mark_as_not_dirty(&mut self, id: &WidgetId) {
        self.dirty.remove(id);
    }
}
