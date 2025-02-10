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
    // When extra tokens (the metadata) are provided.
    ($observed_value:expr, $($rest:tt)*) => {
        $observed_value
    };
    // When only the observed value is provided.
    ($observed_value:expr) => {
        $observed_value
    };
}


#[macro_export]
macro_rules! tv_output {
    (   
        $observed_value:expr
        $(, $rest:tt)*
    ) => {
        // compiles into nothing
    };
}