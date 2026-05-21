use std::collections::HashMap;

struct Tokenizer {
    vocab: HashMap<String, usize>,
    idf: Vec<f32>,
    doc_count: usize,
}

impl Tokenizer {
    fn tokenize(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.vocab.len()];
        let tokens: Vec<&str> = text.split_whitespace().collect();
        for token in &tokens {
            if let Some(&idx) = self.vocab.get(*token) {
                vec[idx] += 1.0 / tokens.len() as f32 * self.idf[idx];
            }
        }

        vec
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
