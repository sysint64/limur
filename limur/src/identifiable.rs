use std::hash::Hash;

pub trait Identifiable {
    type Id: Hash;

    fn id(&self) -> Self::Id;
}

// Primitive integer types
impl Identifiable for usize {
    type Id = usize;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for u8 {
    type Id = u8;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for u16 {
    type Id = u16;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for u32 {
    type Id = u32;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for u64 {
    type Id = u64;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for u128 {
    type Id = u128;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for isize {
    type Id = isize;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for i8 {
    type Id = i8;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for i16 {
    type Id = i16;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for i32 {
    type Id = i32;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for i64 {
    type Id = i64;

    fn id(&self) -> Self::Id {
        *self
    }
}

impl Identifiable for i128 {
    type Id = i128;

    fn id(&self) -> Self::Id {
        *self
    }
}

// String types
impl Identifiable for String {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.clone()
    }
}

impl Identifiable for &str {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.to_string()
    }
}

// Tuple types (common patterns)
impl<T: Identifiable> Identifiable for (T,) {
    type Id = (T::Id,);

    fn id(&self) -> Self::Id {
        (self.0.id(),)
    }
}

impl<T1: Identifiable, T2: Identifiable> Identifiable for (T1, T2) {
    type Id = (T1::Id, T2::Id);

    fn id(&self) -> Self::Id {
        (self.0.id(), self.1.id())
    }
}

// Reference types
impl<T: Identifiable> Identifiable for &T {
    type Id = T::Id;

    fn id(&self) -> Self::Id {
        (*self).id()
    }
}

impl<T: Identifiable> Identifiable for &mut T {
    type Id = T::Id;

    fn id(&self) -> Self::Id {
        (**self).id()
    }
}

// Box
impl<T: Identifiable> Identifiable for Box<T> {
    type Id = T::Id;

    fn id(&self) -> Self::Id {
        (**self).id()
    }
}
