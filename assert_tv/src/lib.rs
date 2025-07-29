use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;

mod test_vec_impl;
mod set;
mod storage;

pub use test_vec_impl::{
    finalize_tv_case, initialize_tv_case_from_file,
    process_next_entry, TestVectorEntryType, TestVecEnv,
};

pub use set::{
    TestValue, TestVector, TestVectorSet, TestVectorNOP, TestVectorActive
};

pub use assert_tv_macros::TestVectorSet;
pub use assert_tv_macros::test_vec_case;


#[cfg(feature = "tls")]
pub use storage::tls_storage::TlsEnvGuard;

#[cfg(not(feature = "tls"))]
pub use storage::storage_global::TlsEnvGuard;


pub type DynSerializer<O> = Box<dyn Fn(&O) -> anyhow::Result<serde_json::Value> + 'static>;
pub type DynDeserializer<O> = Box<dyn Fn(&serde_json::Value) -> anyhow::Result<O> + 'static>;


#[derive(Clone, Copy)]
pub enum TestVectorFileFormat {
    Json,
    Yaml,
    Toml,
}

#[derive(Clone, Copy)]
pub enum TestMode {
    Init,
    Check,
}

impl TestMode {
    pub fn from_environment() -> Self {
        match env::var("TEST_MODE").as_deref() {
            Ok("init") => TestMode::Init,
            Ok("check") => TestMode::Check,
            _ => TestMode::Check, // Default fallback
        }
    }
}

pub trait TestVectorMomento<O> {
    fn serialize(&self, original_value: &O) -> anyhow::Result<serde_json::Value>;

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

