use std::{collections::HashMap, fmt::Debug, hash::Hash};

#[derive(Debug)]
pub struct IndexedMatrix<K, V> {
    index: HashMap<K, u8>,
    matrix: Vec<Vec<V>>,
    row_index: u8,
    free: Vec<u8>,
}

impl<K, V> IndexedMatrix<K, V>
where
    K: Clone + Hash + Eq,
    V: Default + Clone + Debug,
{
    pub fn new(rows: usize) -> Self {
        Self {
            index: HashMap::with_capacity(rows),
            matrix: Vec::with_capacity(rows),
            row_index: 0,
            free: Vec::new(),
        }
    }

    fn claim_row_index(&mut self) -> u8 {
        if let Some(idx) = self.free.pop() {
            idx
        } else {
            let idx = self.row_index;
            self.row_index += 1;
            idx
        }
    }

    fn insert(&mut self, key: K, value: V) -> u8 {
        let row_index = self.claim_row_index();
        self.index.insert(key, row_index);
        if row_index as usize >= self.matrix.len() {
            self.matrix.push(vec![value]);
        } else {
            self.matrix[row_index as usize] = vec![value];
        }
        row_index
    }

    pub fn push(&mut self, key: &K, value: V) {
        let Some(row_index) = self.index.get(key) else {
            self.insert(key.to_owned(), value);
            return;
        };
        self.matrix[*row_index as usize].push(value);
    }

    pub fn remove(&mut self, key: &K) {
        let Some(row_index) = self.index.remove(key) else {
            return;
        };
        self.matrix[row_index as usize].clear();
        self.free.push(row_index);
    }

    pub fn get(&self, key: &K) -> Option<&Vec<V>> {
        let row_index = self.index.get(key)?;
        Some(&self.matrix[*row_index as usize])
    }

    // pub fn as_matrix(&self) -> Option<ndarray::Array2<V>> {
    //     if self.index.is_empty() {
    //         return None;
    //     }
    //     let ncols = self.matrix.iter().map(|r| r.len()).max().unwrap_or(0);
    //     if ncols == 0 {
    //         return None;
    //     }
    //     let nrows = self.index.len();
    //     let mut flat = vec![V::default(); nrows * ncols];
    //     for (i, (_, &row_idx)) in self.index.iter().enumerate() {
    //         let row = &self.matrix[row_idx as usize];
    //         for (j, &val) in row.iter().enumerate() {
    //             flat[i * ncols + j] = val;
    //         }
    //     }
    //     ndarray::Array2::from_shape_vec((nrows, ncols), flat).ok()
    // }
}
