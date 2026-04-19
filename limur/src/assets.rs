use std::collections::HashMap;

use crate::text::FontResources;

#[derive(Default)]
pub struct Assets<'a> {
    fonts: HashMap<&'static str, &'a [u8]>,
    svg: HashMap<&'static str, usvg::Tree>,
}

impl<'a> Assets<'a> {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            svg: HashMap::new(),
        }
    }

    pub fn load_font(&mut self, name: &'static str, data: &'a [u8]) {
        self.fonts.insert(name, data);
    }

    pub fn load_svg(&mut self, name: &'static str, data: &[u8]) {
        let opt = usvg::Options::default();
        let rtree = usvg::Tree::from_data(data, &opt).expect("Invalid SVG");

        self.svg.insert(name, rtree);
    }

    pub fn get_svg_tree(&self, name: &str) -> Option<&usvg::Tree> {
        self.svg.get(name)
    }

    pub fn create_font_resources(&self) -> FontResources {
        let mut fonts = FontResources::new();

        for (name, data) in self.fonts.iter() {
            log::debug!("Load font: {name}");
            fonts.load_font(name, data);
        }

        fonts
    }
}
