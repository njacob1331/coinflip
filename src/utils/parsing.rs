use std::sync::Arc;

use serde::{Deserialize, Deserializer};

use crate::common::SharedStr;

pub fn deserialize_sharedstr<'de, D>(deserializer: D) -> Result<SharedStr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(deserializer)?;
    Ok(Arc::from(s))
}
