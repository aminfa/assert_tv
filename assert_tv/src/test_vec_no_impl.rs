use std::path::PathBuf;
use crate::{TestMode, TestVectorFileFormat};

pub struct TestVectorEntryType;

#[allow(unused_variables)]
pub fn initialize_tv_case_from_file<T: Into<PathBuf>>(tv_file_path: T,
                                                      file_format: TestVectorFileFormat,
                                                      test_mode: TestMode)
                                                      -> anyhow::Result<()>
{
    unimplemented!()
}

pub fn finalize_tv_case() -> anyhow::Result<()> {
    unimplemented!()
}

#[allow(unused_variables)]
pub fn process_next_entry<V>(
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    observed_value: V,
    code_location: Option<String>,
    check_intermediate: bool) -> anyhow::Result<V> {
    // if the library is not enabled, do nothing
    unimplemented!()
    // Ok(observed_value)
}

pub trait WrappedVal {
    type Original;

    fn serialize(&self) -> anyhow::Result<()>;

    fn deserialize(value: &()) -> anyhow::Result<()>;

    fn pop(self) -> Self::Original;
}

impl<T> WrappedVal for T
{
    type Original = Self;

    fn serialize(&self) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn deserialize(value: &()) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn pop(self) -> Self::Original {
        self
    }
}
