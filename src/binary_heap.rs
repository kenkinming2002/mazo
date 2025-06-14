use std::{collections::{hash_map, HashMap}, hash::Hash};

/// Trait required for item to be put into [BinaryHashHeap].
pub trait BinaryHashHeapItem {
    type Key: Hash + Eq + Clone;
    type Value: Ord;

    fn key(&self) -> &Self::Key;
    fn value(&self) -> &Self::Value;
}

/// Similar to [std::collections::BinaryHeap] but additionally support push with decrease key or
/// increase key operation by maintaining a hash map internally.
#[derive(Debug, Clone)]
pub struct BinaryHashHeap<T: BinaryHashHeapItem> {
    items: Vec<T>,
    map: HashMap<T::Key, usize>,
}

impl<T: BinaryHashHeapItem> Default for BinaryHashHeap<T> {
    fn default() -> Self {
        Self {
            items: Default::default(),
            map: Default::default(),
        }
    }
}

/// Action to be taken if an item with the same key has already been inserted into the heap.
pub enum PushAction {
    Keep,
    DecreaseKey,
    IncreaseKey,
}

impl<T: BinaryHashHeapItem> BinaryHashHeap<T> {
    /// Create an empty heap.
    pub fn new() -> Self {
        Default::default()
    }

    fn sift_up(&mut self, index: &mut usize) {
        while *index != 0 {
            let parent_index = (*index - 1) / 2;
            if self.items[parent_index].value() < self.items[*index].value() {
                return;
            }

            let parent_index = self.map.get_mut(self.items[parent_index].key()).unwrap();
            std::mem::swap(index, parent_index);
            self.items.swap(*index, *parent_index);
        }
    }

    fn sift_down(&mut self, index: &mut usize) {
        loop {
            let left_child_index = *index * 2 + 1;
            let right_child_index = *index * 2 + 2;
            let child_index = match (self.items.get(left_child_index), self.items.get(right_child_index)) {
                (None, None) => return,
                (None, Some(_)) => right_child_index,
                (Some(_), None) => left_child_index,
                (Some(left_node), Some(right_node)) => if left_node.value() < right_node.value() {
                    left_child_index
                } else {
                    right_child_index
                }
            };

            if self.items[*index].value() < self.items[child_index].value() {
                return;
            }

            let child_index = self.map.get_mut(self.items[child_index].key()).unwrap();
            std::mem::swap(index, child_index);
            self.items.swap(*index, *child_index);
        }
    }

    /// Push item onto the heap.
    ///
    /// If the item already exist, carry out action specified by action.
    pub fn push(&mut self, action: PushAction, item: T) -> bool {
        match self.map.entry(item.key().clone()) {
            hash_map::Entry::Occupied(mut occupied_entry) => {
                let index = unsafe { &mut *(occupied_entry.get_mut() as *mut usize) }; // SAFETY: Trust me
                match action {
                    PushAction::Keep => return false,
                    PushAction::DecreaseKey => {
                        if self.items[*index].value() <= item.value() {
                            return false;
                        }
                        self.items[*index] = item;
                        self.sift_up(index);
                    },
                    PushAction::IncreaseKey => {
                        if self.items[*index].value() >= item.value() {
                            return false;
                        }
                        self.items[*index] = item;
                        self.sift_down(index);
                    },
                }
            },
            hash_map::Entry::Vacant(vacant_entry) => {
                let index = self.items.len();
                let index = unsafe { &mut *(vacant_entry.insert(index) as *mut usize) }; // SAFETY: Trust me

                self.items.push(item);
                self.sift_up(index);
            },
        }

        true
    }

    /// Pop an item from the heap.
    pub fn pop(&mut self) -> Option<T> {
        if self.items.is_empty() {
            return None;
        }

        let result = self.items.swap_remove(0);
        self.map.remove(result.key());

        if let Some(item) = self.items.first() {
            let index = self.map.get_mut(item.key()).unwrap();
            *index = 0;

            let index = unsafe { &mut *(index as *mut usize) }; // SAFETY: Trust me
            self.sift_down(index);
        }

        Some(result)
    }

    #[cfg(test)]
    fn sanity_check(&self) {
        for (i, item) in self.items.iter().enumerate() {
            assert!(self.map.contains_key(item.key()));
            assert_eq!(*self.map.get(item.key()).unwrap(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    #[test]
    fn test() {
        let mut rng = StdRng::seed_from_u64(0xe3d685fba7d55302);

        #[derive(Debug)]
        struct Item {
            key: usize,
            value: usize,
        }

        impl BinaryHashHeapItem for Item {
            type Key = usize;
            type Value = usize;

            fn key(&self) -> &Self::Key {
                &self.key
            }

            fn value(&self) -> &Self::Value {
                &self.value
            }
        }

        let mut heap = BinaryHashHeap::default();
        for _ in 0..4 {
            for _ in 0..1024 {
                match rng.random_range(0..=10) {
                    0..3 => {
                        let key = rng.random_range(0..100);
                        let value = rng.random_range(0..100);

                        heap.push(PushAction::Keep, Item { key, value });
                        heap.sanity_check();
                    }
                    3..6 => {
                        let key = rng.random_range(0..100);
                        let value = rng.random_range(0..100);

                        heap.push(PushAction::DecreaseKey, Item { key, value });
                        heap.sanity_check();
                    }
                    6..9 => {
                        let key = rng.random_range(0..100);
                        let value = rng.random_range(0..100);

                        heap.push(PushAction::IncreaseKey, Item { key, value });
                        heap.sanity_check();
                    }
                    9..10 => {
                        heap.pop();
                        heap.sanity_check();
                    }
                    10 => {
                        if let Some(node) = heap.pop() {
                            let mut value = *node.value();
                            while let Some(node) = heap.pop() {
                                assert!(value <= *node.value());
                                value = *node.value();
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
