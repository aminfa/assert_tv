use std::path::PathBuf;
use crate::{TestMode, TestVectorFileFormat};


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

#[macro_export]
macro_rules! tv_const {
    (
        $observed_value:expr
        $(, $rest:tt)*
    ) => {
        // compiles into identity function
        $observed_value
    };
}

#[macro_export]
macro_rules! tv_output {
    (
        $(,)*
    ) => {
        // compiles into nothing
    };
}