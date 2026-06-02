use std::{collections::HashMap, hash::Hash};

pub struct Entry<'a, T> {
    pub index: u32,
    pub data: &'a mut T,
}

#[derive(Debug)]
pub struct IndexedStore<K, V> {
    index: HashMap<K, u32>,
    reverse: Vec<K>,
    data: Vec<V>,
}

impl<K, V> IndexedStore<K, V>
where
    K: Clone + Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            reverse: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            index: HashMap::with_capacity(capacity),
            reverse: Vec::with_capacity(capacity),
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.index.clear();
        self.reverse.clear();
        self.data.clear();
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some(index) = self.index.get(&key) {}

        let idx = self.data.len() as u32;
        self.index.insert(key.clone(), idx);
        self.reverse.push(key);
        self.data.push(value);
    }

    pub fn get_mut(&mut self, key: &K) -> Option<Entry<'_, V>> {
        let &index = self.index.get(key)?;
        Some(Entry {
            index,
            data: &mut self.data[index as usize],
        })
    }

    pub fn update_data_at(&mut self, index: usize, new: V) {
        self.data[index] = new
    }

    pub fn remove(&mut self, key: &K) {
        if let Some(removed) = self.index.remove(key) {
            let last = self.data.len() - 1;
            let removed = removed as usize;

            if removed != last {
                self.data.swap(removed, last);
                self.reverse.swap(removed, last);
                self.index
                    .insert(self.reverse[removed].clone(), removed as u32);
            }

            self.data.pop();
            self.reverse.pop();
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter_data(&self) -> impl Iterator<Item = &V> {
        self.data.iter()
    }
}
