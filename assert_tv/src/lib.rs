//! Public crate API for test‑vector based deterministic testing.
//!
//! See the README for an end‑to‑end walkthrough. At a glance:
//! - Use `#[derive(TestVectorSet)]` to define `TestValue<…>` fields
//! - Parameterize code under test with `TV: TestVector`
//! - In tests, wrap functions with `#[test_vec_case]` or manually call
//!   `initialize_tv_case_from_file` and `finalize_tv_case`.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;

mod caller_location;
mod set;
mod storage;
mod test_vec_impl;

pub use test_vec_impl::{
    finalize_tv_case, initialize_tv_case_from_file, process_next_entry, TestVecEnv,
    TestVectorEntryType,
};

pub use set::{TestValue, TestVector, TestVectorActive, TestVectorNOP, TestVectorSet};

pub use assert_tv_macros::test_vec_case;
pub use assert_tv_macros::TestVectorSet;

#[cfg(feature = "tls")]
pub use storage::tls_storage::TlsEnvGuard;

#[cfg(not(feature = "tls"))]
pub use storage::storage_global::TlsEnvGuard;

/// Erased serializer used by `TestValue<T>` to persist values in test vectors.
///
/// Implemented by default via `serde_json::to_value`, but can be customized per field
/// through `#[test_vec(serialize_with = "path::to::fn")]`.
pub type DynSerializer<O> = Box<dyn Fn(&O) -> anyhow::Result<serde_json::Value> + 'static>;

/// Erased deserializer used by `TestValue<T>` to restore values from test vectors.
///
/// Implemented by default via `serde_json::from_value`, but can be customized per field
/// through `#[test_vec(deserialize_with = "path::to::fn")]`.
pub type DynDeserializer<O> = Box<dyn Fn(&serde_json::Value) -> anyhow::Result<O> + 'static>;

#[derive(Clone, Copy)]
/// File format used to read/write test vector files.
pub enum TestVectorFileFormat {
    /// JSON file (`.json`).
    Json,
    /// YAML file (`.yaml` / `.yml`).
    Yaml,
    /// TOML file (`.toml`).
    Toml,
}

#[derive(Clone, Copy)]
/// Execution mode for test vectors.
///
/// - `Init`: record observed entries and write the file if changed
/// - `Check`: load the file and validate observed entries; constants are injected
pub enum TestMode {
    Init,
    Check,
}

impl TestMode {
    /// Reads `TEST_MODE` ("init" | "check"). Defaults to `Check`.
    pub fn from_environment() -> Self {
        match env::var("TEST_MODE").as_deref() {
            Ok("init") => TestMode::Init,
            Ok("check") => TestMode::Check,
            _ => TestMode::Check, // Default fallback
        }
    }
}

/// Pluggable serializer/deserializer for a type used in test vectors.
///
/// A blanket impl exists for any `T: Serialize + DeserializeOwned`, delegating to serde_json.
pub trait TestVectorMomento<O> {
    /// Serialize `original_value` into a JSON value for storage in a test vector.
    fn serialize(&self, original_value: &O) -> anyhow::Result<serde_json::Value>;

    /// Deserialize a JSON value that was previously stored in a test vector.
    fn deserialize(&self, value: &serde_json::Value) -> anyhow::Result<O>;
}

impl<O> TestVectorMomento<O> for O
where
    O: Serialize + DeserializeOwned,
{
    fn serialize(&self, original_value: &O) -> anyhow::Result<serde_json::Value> {
        // Convert the value to a serde_json::Value, mapping errors using anyhow.
        serde_json::to_value(original_value).map_err(anyhow::Error::new)
    }

    fn deserialize(&self, value: &serde_json::Value) -> anyhow::Result<O> {
        // We clone the value because from_value takes ownership.
        serde_json::from_value(value.clone()).map_err(anyhow::Error::new)
    }
}
