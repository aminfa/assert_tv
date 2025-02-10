use std::env;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(feature = "enabled")]
mod test_vec_impl;
#[cfg(feature = "enabled")]
pub use test_vec_impl::{
    finalize_tv_case,
    initialize_tv_case_from_file,
    process_next_entry,
    process_next_entry_infer_type,
    TestVectorEntryType,
    helper_infer_const,
    helper_infer_output
};


#[cfg(not(feature = "enabled"))]
mod test_vec_no_impl;
#[cfg(not(feature = "enabled"))]
pub use test_vec_no_impl::{
    finalize_tv_case,
    initialize_tv_case_from_file,
};


#[derive(Clone, Copy)]
pub enum TestVectorFileFormat {
    Json,
    Yaml
}

#[derive(Clone, Copy)]
pub enum TestMode {
    Init,
    Check
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



// pub trait TestVectorMomento {
//     type Originator;
// 
//     fn serialize(original_value: &Self::Originator) -> anyhow::Result<serde_json::Value>;
// 
//     fn deserialize(value: &serde_json::Value) -> anyhow::Result<Self::Originator>;
// }
// impl<T> TestVectorMomento for T
// where
//     T: Serialize + DeserializeOwned,
// {
//     type Originator = Self;
// 
//     fn serialize(original_value: &Self::Originator) -> anyhow::Result<serde_json::Value> {
//         // Convert the value to a serde_json::Value, mapping errors using anyhow.
//         serde_json::to_value(original_value).map_err(anyhow::Error::new)
//     }
// 
//     fn deserialize(value: &serde_json::Value) -> anyhow::Result<Self::Originator> {
//         // We clone the value because from_value takes ownership.
//         serde_json::from_value(value.clone()).map_err(anyhow::Error::new)
//     }
// }


pub trait TestVectorMomento<O> {

    fn serialize(original_value: &O) -> anyhow::Result<serde_json::Value>;

    fn deserialize(value: &serde_json::Value) -> anyhow::Result<O>;
}

impl <O> TestVectorMomento<O> for O
where O: Serialize + DeserializeOwned {

    fn serialize(original_value: &O) -> anyhow::Result<serde_json::Value> {
        // Convert the value to a serde_json::Value, mapping errors using anyhow.
        serde_json::to_value(original_value).map_err(anyhow::Error::new)
    }

    fn deserialize(value: &serde_json::Value) -> anyhow::Result<O> {
        // We clone the value because from_value takes ownership.
        serde_json::from_value(value.clone()).map_err(anyhow::Error::new)
    }
}