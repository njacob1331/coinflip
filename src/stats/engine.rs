// use std::collections::VecDeque;

// use crate::{stats::matcher::StructuralCorrelationGraph, traits::Observable};

// struct ObservationWindow<T> {
//     observations: VecDeque<T>,
//     capacity: usize,
// }

// impl<T> ObservationWindow<T> {
//     fn new(capacity: usize) -> Self {
//         Self {
//             observations: VecDeque::with_capacity(capacity),
//             capacity,
//         }
//     }

//     fn update(&mut self, obs: T) {
//         if self.observations.len() == self.capacity {
//             self.observations.pop_front();
//         }

//         self.observations.push_back(obs);
//     }
// }

// struct StatsEngine<O> {
//     similarity_graph: StructuralCorrelationGraph<O>,
// }

// impl<O> StatsEngine<O>
// where
//     O: Observable,
// {
//     fn new() -> Self {
//         Self {
//             similarity_graph: StructuralCorrelationGraph::new(),
//         }
//     }
// }
