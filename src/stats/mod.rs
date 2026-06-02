pub mod engine;
pub mod matcher;
pub mod observation;
pub mod stratify;

pub use observation::Observation;

pub enum TransportMsg<K, V> {
    HandleData(V),
    RemoveData(K),
}
