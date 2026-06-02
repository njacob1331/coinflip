use crate::{common::SharedStr, stats::stratify::Stratified};

#[derive(Debug, Default, Copy, Clone)]
pub enum Timescale {
    #[default]
    Millis,
    Nano,
}

#[derive(Debug, Default, Clone)]
pub struct Observation<T> {
    pub id: SharedStr,
    pub ts: u64,
    pub timescale: Timescale,
    pub value: T,
    pub stratified: Stratified,
}

impl<T> Observation<T> {
    pub fn stratify(&mut self, stratified: Stratified) {
        self.stratified = stratified
    }
}
