use std::{
    hash::{Hash, Hasher},
    u64,
};

use rustc_hash::FxHasher;

#[derive(Default, Clone, Copy, Debug, Eq)]
pub struct WidgetId {
    base: u64, // hash of file/line/column
    seed: Option<u64>,
}

impl Hash for WidgetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.seed.hash(state);
    }
}

impl PartialEq for WidgetId {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.seed == other.seed
    }
}

impl WidgetId {
    #[track_caller]
    pub fn auto() -> Self {
        let location = std::panic::Location::caller();

        let mut hasher = FxHasher::default();
        std::ptr::hash(location.file(), &mut hasher);
        location.line().hash(&mut hasher);
        location.column().hash(&mut hasher);

        Self {
            base: hasher.finish(),
            seed: None,
        }
    }

    #[track_caller]
    pub fn auto_with_seed(seed: impl Hash) -> Self {
        let mut hasher = FxHasher::default();
        seed.hash(&mut hasher);

        Self::auto().with_seed(Some(hasher.finish()))
    }

    pub fn with_seed(mut self, seed: Option<u64>) -> Self {
        if self.seed.is_none() {
            self.seed = seed;
        }
        self
    }
}

pub struct LayoutWidget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetType {
    type_id: std::any::TypeId,
    name: &'static str,
}

impl WidgetType {
    pub fn of<T: 'static>() -> Self {
        Self {
            type_id: std::any::TypeId::of::<T>(),
            name: std::any::type_name::<T>(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetRef {
    pub widget_type: WidgetType,
    pub id: WidgetId,
}

pub struct DebugBoundary;

impl WidgetRef {
    pub(crate) fn new(widget_type: WidgetType, id: WidgetId) -> Self {
        Self { widget_type, id }
    }
}
