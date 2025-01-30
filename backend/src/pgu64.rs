// Copied from 
// https://github.com/alpenlabs/batch-explorer/ (in backend -> model)

use serde::{Deserialize, Serialize};
pub struct PgU64(pub u64);

impl PgU64 {
    // Converts u64 to i64 for database storage
    pub fn to_i64(&self) -> i64 {
        (self.0 as i128 - 2_i128.pow(63)) as i64
    }

    // Converts from i64 back to u64 for application use
    pub fn from_i64(value: i64) -> Self {
        PgU64((value as i128 + 2_i128.pow(63)) as u64)
    }

    pub fn from_u64(value: u64) -> Self {
        PgU64(value)
    }

    pub fn new(value: u64) -> Self {
        PgU64(value)
    }
}

impl Serialize for PgU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for PgU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = u64::deserialize(deserializer)?;
        Ok(PgU64(raw))
    }
}