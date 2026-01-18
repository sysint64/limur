use cosmic_text::Edit;
use slotmap::{SlotMap, new_key_type};
use string_interner;

use crate::{Vec2, View};

new_key_type! {
    pub struct FontId;
    pub struct TextId;
}

pub type StringId = string_interner::DefaultSymbol;

pub type StringInterner = string_interner::StringInterner<string_interner::DefaultBackend>;

pub struct FontResources {
    pub font_system: cosmic_text::FontSystem,
    fonts: SlotMap<FontId, &'static str>,
}

impl Default for FontResources {
    fn default() -> Self {
        Self::new()
    }
}

impl FontResources {
    pub fn new() -> Self {
        let font_system = cosmic_text::FontSystem::new();

        Self {
            font_system,
            fonts: SlotMap::default(),
        }
    }

    pub fn load_font(&mut self, name: &'static str, data: &[u8]) -> FontId {
        self.font_system.db_mut().load_font_data(data.to_vec());

        self.fonts.insert(name)
    }
}

pub enum Text<'buffer> {
    Buffer {
        buffer: cosmic_text::Buffer,
        attrs: cosmic_text::Attrs<'buffer>,
        font_size: f32,
        line_height: f32,
    },
    Editor {
        editor: cosmic_text::Editor<'buffer>,
        attrs: cosmic_text::Attrs<'buffer>,
        font_size: f32,
        line_height: f32,
    },
}

pub struct TextsResources<'a> {
    items: SlotMap<TextId, Text<'a>>,
}

impl<'a> Default for TextsResources<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TextsResources<'a> {
    pub fn new() -> Self {
        Self {
            items: SlotMap::default(),
        }
    }

    pub fn editor(&self, id: TextId) -> &cosmic_text::Editor<'a> {
        match self.items.get(id).unwrap() {
            Text::Buffer { .. } => panic!("Provided text id is not editor"),
            Text::Editor { editor, .. } => editor,
        }
    }

    pub fn editor_mut(&mut self, id: TextId) -> &mut cosmic_text::Editor<'a> {
        match self.items.get_mut(id).unwrap() {
            Text::Buffer { .. } => panic!("Provided text id is not editor"),
            Text::Editor { editor, .. } => editor,
        }
    }

    pub fn shape_as_needed(
        &mut self,
        id: TextId,
        font_system: &mut cosmic_text::FontSystem,
        prune: bool,
    ) {
        match self.items.get_mut(id).unwrap() {
            Text::Buffer { buffer, .. } => buffer.shape_until_scroll(font_system, prune),
            Text::Editor { editor, .. } => editor.shape_as_needed(font_system, prune),
        }
    }

    pub fn get(&self, id: TextId) -> &Text<'a> {
        self.items.get(id).unwrap()
    }

    pub fn get_mut(&mut self, id: TextId) -> &mut Text<'a> {
        self.items.get_mut(id).unwrap()
    }

    pub fn get_mut_option(&mut self, id: TextId) -> Option<&mut Text<'a>> {
        self.items.get_mut(id)
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn add_text<F>(
        &mut self,
        view: &View,
        font_resources: &mut FontResources,
        font_size: f32,
        line_height: f32,
        callback: F,
    ) -> TextId
    where
        F: FnOnce(&mut FontResources, &mut Text<'a>),
    {
        let mut text = Text::new(view, font_resources, font_size, line_height);
        callback(font_resources, &mut text);

        self.items.insert(text)
    }

    pub fn add_editor<F>(
        &mut self,
        view: &View,
        font_resources: &mut FontResources,
        font_size: f32,
        line_height: f32,
        callback: F,
    ) -> TextId
    where
        F: FnOnce(&mut FontResources, &mut Text<'a>),
    {
        let mut text = Text::editor(view, font_resources, font_size, line_height);
        callback(font_resources, &mut text);

        self.items.insert(text)
    }

    pub fn update_text<F, T>(&mut self, id: TextId, callback: F) -> T
    where
        F: FnOnce(&mut Text<'a>) -> T,
    {
        let text = self.items.get_mut(id).unwrap();

        callback(text)
    }

    pub fn update_view(&mut self, view: &View, font_resources: &mut FontResources) {
        for text in self.items.values_mut() {
            text.update_view(view, font_resources);
        }
    }

    pub fn remove(&mut self, id: TextId) {
        self.items.remove(id);
    }
}

pub enum TextStyle {
    Normal,
    Italic,
}

pub enum TextWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl<'buffer> Text<'buffer> {
    pub fn new(
        view: &View,
        font_resources: &mut FontResources,
        font_size: f32,
        line_height: f32,
    ) -> Self {
        let buffer = cosmic_text::Buffer::new(
            &mut font_resources.font_system,
            cosmic_text::Metrics::new(
                font_size * view.scale_factor,
                line_height * view.scale_factor,
            ),
        );

        let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::SansSerif);

        Self::Buffer {
            buffer,
            attrs,
            font_size,
            line_height,
        }
    }

    pub fn editor(
        view: &View,
        font_resources: &mut FontResources,
        font_size: f32,
        line_height: f32,
    ) -> Self {
        let buffer = cosmic_text::Buffer::new(
            &mut font_resources.font_system,
            cosmic_text::Metrics::new(
                font_size * view.scale_factor,
                line_height * view.scale_factor,
            ),
        );

        let attrs = cosmic_text::Attrs::new().family(cosmic_text::Family::SansSerif);
        let editor = cosmic_text::Editor::new(buffer);

        Self::Editor {
            editor,
            attrs,
            font_size,
            line_height,
        }
    }

    pub fn set_metrics(
        &mut self,
        view: &View,
        font_resources: &mut FontResources,
        font_size: f32,
        line_height: f32,
    ) {
        self.with_buffer_mut(|buffer| {
            buffer.set_metrics(
                &mut font_resources.font_system,
                cosmic_text::Metrics::new(
                    font_size * view.scale_factor,
                    line_height * view.scale_factor,
                ),
            );
        });
    }

    pub fn update_view(&mut self, view: &View, font_resources: &mut FontResources) {
        let font_size = self.font_size();
        let line_height = self.line_height();

        self.with_buffer_mut(|buffer| {
            buffer.set_metrics(
                &mut font_resources.font_system,
                cosmic_text::Metrics::new(
                    font_size * view.scale_factor,
                    line_height * view.scale_factor,
                ),
            );
        });
    }

    pub fn set_style(&mut self, style: TextStyle) {
        self.with_attrs_mut(|attrs| {
            *attrs = attrs.clone().style(match style {
                TextStyle::Normal => cosmic_text::Style::Normal,
                TextStyle::Italic => cosmic_text::Style::Italic,
            });
        });
    }

    pub fn set_weight(&mut self, weight: TextWeight) {
        self.with_attrs_mut(|attrs| {
            *attrs = attrs.clone().weight(match weight {
                TextWeight::Thin => cosmic_text::Weight::THIN,
                TextWeight::ExtraLight => cosmic_text::Weight::EXTRA_LIGHT,
                TextWeight::Light => cosmic_text::Weight::LIGHT,
                TextWeight::Normal => cosmic_text::Weight::NORMAL,
                TextWeight::Medium => cosmic_text::Weight::MEDIUM,
                TextWeight::Semibold => cosmic_text::Weight::SEMIBOLD,
                TextWeight::Bold => cosmic_text::Weight::BOLD,
                TextWeight::ExtraBold => cosmic_text::Weight::EXTRA_BOLD,
                TextWeight::Black => cosmic_text::Weight::BLACK,
            });
        });
    }

    pub fn calculate_size(&mut self) -> Vec2 {
        let mut max_width = 0.;
        let mut height = 0.;

        self.with_buffer(|buffer| {
            for layout in buffer.layout_runs() {
                max_width = f32::max(max_width, layout.line_w);
                height = layout.line_y;
            }
        });

        Vec2::new(max_width, height)
    }

    pub fn set_text(&mut self, font_resources: &mut FontResources, text: &str) {
        self.with_buffer_and_attrs_mut(|buffer, attrs| {
            buffer.set_text(
                &mut font_resources.font_system,
                text,
                attrs,
                cosmic_text::Shaping::Advanced,
            );
        });
    }

    pub fn with_buffer_and_attrs_mut<F>(&mut self, callback: F)
    where
        F: FnOnce(&mut cosmic_text::Buffer, &mut cosmic_text::Attrs),
    {
        match self {
            Text::Buffer { buffer, attrs, .. } => callback(buffer, attrs),
            Text::Editor { editor, attrs, .. } => callback(
                match editor.buffer_ref_mut() {
                    cosmic_text::BufferRef::Owned(buffer) => buffer,
                    _ => panic!("Invalid ref"),
                },
                attrs,
            ),
        }
    }

    pub fn buffer(&self) -> &cosmic_text::Buffer {
        match self {
            Text::Buffer { buffer, .. } => buffer,
            Text::Editor { editor, .. } => match editor.buffer_ref() {
                cosmic_text::BufferRef::Owned(buffer) => buffer,
                _ => panic!("Invalid ref"),
            },
        }
    }

    pub fn with_attrs_mut<F>(&mut self, callback: F)
    where
        F: FnOnce(&mut cosmic_text::Attrs),
    {
        match self {
            Text::Buffer { attrs, .. } => callback(attrs),
            Text::Editor { attrs, .. } => callback(attrs),
        }
    }

    pub fn with_buffer<F>(&self, callback: F)
    where
        F: FnOnce(&cosmic_text::Buffer),
    {
        match self {
            Text::Buffer { buffer, .. } => callback(buffer),
            Text::Editor { editor, .. } => editor.with_buffer(callback),
        }
    }

    pub fn with_buffer_mut<F>(&mut self, callback: F)
    where
        F: FnOnce(&mut cosmic_text::Buffer),
    {
        match self {
            Text::Buffer { buffer, .. } => callback(buffer),
            Text::Editor { editor, .. } => editor.with_buffer_mut(callback),
        }
    }

    fn font_size(&self) -> f32 {
        match self {
            Text::Buffer { font_size, .. } => *font_size,
            Text::Editor { font_size, .. } => *font_size,
        }
    }

    fn line_height(&self) -> f32 {
        match self {
            Text::Buffer { line_height, .. } => *line_height,
            Text::Editor { line_height, .. } => *line_height,
        }
    }
}
