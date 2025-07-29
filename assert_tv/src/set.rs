use crate::TlsEnvGuard;
use crate::{
    initialize_tv_case_from_file, DynDeserializer, DynSerializer, TestMode, TestVectorFileFormat,
};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::path::PathBuf;

pub trait TestVectorSet {
    fn start<TV: TestVector>() -> Self;
}

pub struct TestValue<O> {
    pub name: Option<String>,
    pub description: Option<String>,
    pub test_value_field_code_location: String,
    pub serializer: Option<DynSerializer<O>>,
    pub deserializer: Option<DynDeserializer<O>>,
    pub _data_marker: PhantomData<O>,
}

impl<O> Debug for TestValue<O> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestValue")
            .field("name", &self.name)
            .field("description", &self.description)
            .field(
                "test_value_field_code_location",
                &self.test_value_field_code_location,
            )
            .finish()
    }
}

impl<O> TestValue<O> {
    pub fn new(
        name: Option<String>,
        description: Option<String>,
        code_location: String,
        serializer: Option<DynSerializer<O>>,
        deserializer: Option<DynDeserializer<O>>,
    ) -> Self {
        Self {
            name,
            description,
            test_value_field_code_location: code_location,
            serializer,
            deserializer,
            _data_marker: PhantomData,
        }
    }
}

pub trait TestVector {
    fn initialize_test_vector<T: Into<PathBuf>>(
        tv_file_path: T,
        file_format: TestVectorFileFormat,
        test_mode: TestMode,
    ) -> TlsEnvGuard {
        initialize_tv_case_from_file(tv_file_path, file_format, test_mode)
            .expect("Failed to initialize test vector")
    }

    #[inline(always)]
    fn initialize_values<T: TestVectorSet>() -> T
    where
        Self: Sized,
    {
        T::start::<Self>()
    }

    #[inline(always)]
    fn expose_value<O>(test_vec_field: &TestValue<O>, mut observed_value: O) -> O {
        Self::expose_mut_value(test_vec_field, &mut observed_value);
        observed_value
    }

    #[inline(always)]
    fn expose_mut_value<O>(test_vec_field: &TestValue<O>, observed_mut_value: &mut O) {
        *observed_mut_value = crate::process_next_entry(
            crate::TestVectorEntryType::Const,
            test_vec_field.description.clone(),
            test_vec_field.name.clone(),
            observed_mut_value,
            Some(test_vec_field.test_value_field_code_location.clone()),
            test_vec_field.serializer.as_ref().unwrap_or_else(|| panic!("Serializer was not provided for test field: {test_vec_field:?}")),
            Some(
                test_vec_field.deserializer.as_ref().unwrap_or_else(|| panic!("Deserializer was not provided for test field: {test_vec_field:?}")),
            ),
        )
        .expect("Error processing observed test vector value")
        .expect("Unexpected error processing observed test vector const: no value was loaded");
    }

    // #[inline(always)]
    // fn load_value<F: TestVectorField>(observed_value: F::Value) -> F::Value {
    //     process_tv_observation_const!(
    //         observed_value,
    //         F::Momento,
    //         F::name(),
    //         F::description(),
    //         F::code_location(),
    //     )
    // }

    // #[inline(always)]
    // fn load_mut_value<F: TestVectorField>(observed_value: &mut F::Value) {
    //     *observed_value = process_tv_observation_const!(
    //         observed_value,
    //         F::Momento,
    //         F::name(),
    //         F::description(),
    //         F::code_location(),
    //     );
    // }

    #[inline(always)]
    fn check_value<O>(test_vec_field: &TestValue<O>, observed_value: &O) {
        crate::process_next_entry(
            crate::TestVectorEntryType::Output,
            test_vec_field.description.clone(),
            test_vec_field.name.clone(),
            observed_value,
            Some(test_vec_field.test_value_field_code_location.clone()),
            test_vec_field.serializer.as_ref().unwrap_or_else(|| {
                panic!(
                    "Serializer was not provided for test field: {test_vec_field:?}"
                )
            }),
            None,
        )
        .expect("Error checking observed test vector value");
    }

    fn is_test_vector_enabled() -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct TestVectorActive;

impl TestVector for TestVectorActive {}

#[derive(Clone, Debug)]
pub struct TestVectorNOP;

impl TestVector for TestVectorNOP {
    #[inline(always)]
    fn initialize_test_vector<T: Into<PathBuf>>(
        _tv_file_path: T,
        _file_format: TestVectorFileFormat,
        _test_mode: TestMode,
    ) -> TlsEnvGuard {
        panic!(
            "TestVectorNOP is used (by default) for when the code runs in production.\
             No test vector was explicitly defined."
        )
    }

    #[inline(always)]
    fn expose_value<O>(_test_vec_field: &TestValue<O>, observed_value: O) -> O {
        observed_value // return the value given
    }

    #[inline(always)]
    fn expose_mut_value<O>(_test_vec_field: &TestValue<O>, _observed_mut_value: &mut O) {
        // no impl does nothing
    }

    #[inline(always)]
    fn check_value<O>(_test_vec_field: &TestValue<O>, _observed_value: &O) {
        // no impl does nothing
    }

    #[inline(always)]
    fn is_test_vector_enabled() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{initialize_tv_case_from_file, TestMode, TestVectorFileFormat};

    #[test]
    fn it_works() {
        let _guard: crate::TlsEnvGuard =
            initialize_tv_case_from_file("a", TestVectorFileFormat::Json, TestMode::Init).unwrap();
    }
}
