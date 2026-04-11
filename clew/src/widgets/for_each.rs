use crate::identifiable::Identifiable;

use super::{builder::BuildContext, scope::scope};

pub struct ForEachBuilder<I> {
    items: I,
}

impl<I> ForEachBuilder<I>
where
    I: IntoIterator,
    I::Item: Identifiable,
{
    pub fn build<F>(self, context: &mut BuildContext, mut callback: F)
    where
        F: FnMut(&mut BuildContext, I::Item),
    {
        for item in self.items {
            let key = item.id();

            scope(key).build(context, |context| {
                callback(context, item);
            });
        }
    }
}

pub fn for_each<I>(items: I) -> ForEachBuilder<I> {
    ForEachBuilder { items }
}
