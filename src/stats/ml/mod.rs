use std::collections::HashMap;

use anyhow::Result;
use ndarray::{ArrayViewD, Axis, Ix3};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use tokenizers::Tokenizer;

#[derive(Debug)]
pub struct EmbeddingModel {
    tokenizer: Tokenizer,
    session: Session,
    cache: HashMap<String, Vec<f32>>,
}

impl EmbeddingModel {
    pub fn try_new() -> Result<Self> {
        let tokenizer = Tokenizer::from_file(
            "/Users/nick/projects/coinflip/assets/models/embedding/tokenizer.json",
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_file("/Users/nick/projects/coinflip/assets/models/embedding/model.onnx")?;

        Ok(Self {
            tokenizer,
            session,
            cache: HashMap::new(),
        })
    }

    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let enc = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!(e))?;

        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let mask_f32: Vec<f32> = enc.get_attention_mask().iter().map(|&x| x as f32).collect();
        let len = ids.len();

        // Raw (shape, boxed_slice) — the one form that always works across rc versions
        let ids_tensor = Tensor::<i64>::from_array(([1usize, len], ids.into_boxed_slice()))?;
        let mask_tensor = Tensor::<i64>::from_array(([1usize, len], mask.into_boxed_slice()))?;
        let type_ids: Vec<i64> = vec![0i64; len];
        let type_ids_tensor =
            Tensor::<i64>::from_array(([1usize, len], type_ids.into_boxed_slice()))?;

        let outputs = self.session.run(ort::inputs![
            "input_ids"      => ids_tensor,
            "attention_mask" => mask_tensor,
            "token_type_ids" => type_ids_tensor,
        ])?;

        // try_extract_array returns ArrayViewD<f32> with ndarray feature enabled
        let output: ArrayViewD<f32> = outputs["last_hidden_state"].try_extract_array()?;
        let hidden = output.into_dimensionality::<Ix3>()?;

        Ok(Self::mean_pooling(hidden, &mask_f32))
    }

    fn mean_pooling(hidden: ndarray::ArrayView3<f32>, mask: &[f32]) -> Vec<f32> {
        let seq_len = hidden.len_of(Axis(1));
        let hidden_size = hidden.len_of(Axis(2));
        let mut result = vec![0.0f32; hidden_size];
        let mut denom = 0.0f32;

        for t in 0..seq_len {
            let m = mask[t];
            denom += m;
            for h in 0..hidden_size {
                result[h] += hidden[[0, t, h]] * m;
            }
        }
        for v in &mut result {
            *v /= denom.max(1e-9);
        }

        Self::normalize(result)
    }

    fn normalize(mut v: Vec<f32>) -> Vec<f32> {
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
        for x in &mut v {
            *x /= norm;
        }
        v
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (na * nb + 1e-8)
    }

    pub fn call(&mut self, x: &str, y: &str) -> Result<f32> {
        if !self.cache.contains_key(x) {
            let embedding = self.embed(x)?;
            self.cache.insert(x.to_string(), embedding);
        }
        if !self.cache.contains_key(y) {
            let embedding = self.embed(y)?;
            self.cache.insert(y.to_string(), embedding);
        }

        let x = &self.cache[x];
        let y = &self.cache[y];

        Ok(Self::cosine(x, y))
    }
}
