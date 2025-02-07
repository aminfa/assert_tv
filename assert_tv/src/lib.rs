#[cfg(feature = "enabled")]
mod test_vec_impl;

use std::env;
#[cfg(feature = "enabled")]
pub use test_vec_impl::{
    finalize_tv_case,
    initialize_tv_case_from_file,
    process_next_entry,
    TestVectorEntryType,
    TestVectorValue
};


#[cfg(not(feature = "enabled"))]
mod test_vec_no_impl;
#[cfg(not(feature = "enabled"))]
pub use test_vec_no_impl::{
    finalize_tv_case,
    initialize_tv_case_from_file,
    process_next_entry,
    TestVectorEntryType,
    TestVectorValue
};


#[derive(Clone, Copy)]
pub enum TestVectorFileFormat {
    Json,
    Yaml
}

#[derive(Clone, Copy)]
pub enum TestMode {
    Init,
    Record,
    Check
}

impl TestMode {
    pub fn from_environment() -> Self {
        match env::var("TEST_MODE").as_deref() {
            Ok("init") => TestMode::Init,
            Ok("record") => TestMode::Record,
            Ok("check") => TestMode::Check,
            _ => TestMode::Check, // Default fallback
        }
    }
}



#[macro_export]
macro_rules! process_tv_observation {
    ($cfg_option:meta,
        $generator:expr,
        $entry_type:expr,
        $name: expr,
        $description:expr,
        $check_intermediate:expr,
    ) => {
        {
            #[cfg($cfg_option)]
            #[allow(unused_braces)]
            {
                let value = $generator;
                $crate::process_next_entry(
                    $entry_type,
                    $description,
                    $name,
                    value,
                    Some(format!("{}:{}", file!(), line!())),
                    $check_intermediate
                )
                    .expect("Error processing observed test vector value")
            }

            #[cfg(not($cfg_option))]
            #[allow(unused_braces)]
            {
                $crate::TestVectorValue::pop($generator)
            }
        }
    }
}

#[macro_export]
macro_rules! tv_const {

    ($cfg_option:meta,
        $generator:expr,
        $name: expr,
        $description:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Const, Some($name.into()), Some($description.into()), false, )
    };

    // Version without description
    ($cfg_option:meta, $generator:expr, $name:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Const, Some($name.into()), None, false, )
    };

    // Version without name and description
    ($cfg_option:meta, $generator:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Const, None, None, false, )
    };

    // Version without cfg, and description
    ($generator:expr, $name:expr) => {
        $crate::tv_const!(test, $generator, $name)
    };

    // Version without cfg, name and description
    ($generator:expr) => {
        $crate::tv_const!(test, $generator)
    };
}

#[macro_export]
macro_rules! tv_intermediate {
    (   $cfg_option:meta,
        $generator:expr,
        $name: expr,
        $description:expr
    ) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, Some($name.into()), Some($description.into()), false, )
    };

    // Version without description
    ($cfg_option:meta, $generator:expr, $name:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, Some($name.into()), None, false, )
    };

    // Version without name and description
    ($cfg_option:meta, $generator:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, None, None, false, )
    };

}


#[macro_export]
macro_rules! tv_checked_intermediate {
    (   $cfg_option:meta,
        $generator:expr,
        $name: expr,
        $description:expr
    ) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, Some($name.into()), Some($description.into()), true, )
    };

    // Version without description
    ($cfg_option:meta, $generator:expr, $name:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, Some($name.into()), None, true, )
    };

    // Version without name and description
    ($cfg_option:meta, $generator:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Intermediate, None, None, true, )
    };

}



#[macro_export]
macro_rules! tv_output {
    ($cfg_option:meta,
        $generator:expr,
        $name: expr,
        $description:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Output, Some($name.into()), Some($description.into()), false, )
    };

    // Version without description
    ($cfg_option:meta, $generator:expr, $name:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Output, Some($name.into()), None, false, )
    };

    // Version without name and description
    ($cfg_option:meta, $generator:expr) => {
        $crate::process_tv_observation!($cfg_option, $generator, $crate::TestVectorEntryType::Output, None, None, false, )
    };

}

