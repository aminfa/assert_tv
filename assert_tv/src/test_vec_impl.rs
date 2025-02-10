use std::path::{PathBuf};
use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use crate::{TestMode, TestVectorFileFormat, TestVectorMomento};
use crate::test_vec_impl::storage::TlsEnvGuard;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TestVectorEntry {
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    value: serde_json::Value,
    code_location: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Copy, Eq, PartialEq, Clone)]
pub enum TestVectorEntryType {
    Const,
    Output
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct TestVectorData {
    entries: Vec<TestVectorEntry>
}

struct TestVecEnv {
    tv_file_path: PathBuf,
    file_format: TestVectorFileFormat,
    loaded_tv_data: TestVectorData,
    recorded_tv_data: TestVectorData,
    test_mode: TestMode
}

#[cfg(not(feature = "tls"))]
mod storage {
    use std::marker::PhantomData;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    struct TestVecEnvSingleton(
        OnceLock<Mutex<Option<crate::test_vec_impl::TestVecEnv>>>
    );

    static TEST_VEC_ENV: TestVecEnvSingleton = TestVecEnvSingleton(OnceLock::new());
    // because it is a global env that all threads write to, we need a lock
    static GLOBAL_ENV_LOCK: Mutex<()> = Mutex::new(());

    pub struct TlsEnvGuard {
        // Prevents Send implementation to ensure the guard is dropped in the same thread.
        _marker: PhantomData<*const ()>,
        single_threaded_lock: Option<MutexGuard<'static, ()>>
    }


    impl Drop for TlsEnvGuard {
        fn drop(&mut self) {
            drop(self.single_threaded_lock.take());
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global().lock().expect("Global poisoned");
            test_vec_env_lock.take();
        }
    }

    impl crate::test_vec_impl::TestVecEnv {
        pub(super)  fn get_global() -> &'static Mutex<Option<crate::test_vec_impl::TestVecEnv>> {
            TEST_VEC_ENV.0.get_or_init(|| Mutex::new(None))
        }

        pub(super) fn initialize_with(self) -> anyhow::Result<TlsEnvGuard> {
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global().lock().expect("Global poisoned");
            test_vec_env_lock.replace(self);
            let global_lock_guard = GLOBAL_ENV_LOCK.lock().expect("Global env lock poisoned");
            Ok(crate::test_vec_impl::TlsEnvGuard {
                single_threaded_lock: Some(global_lock_guard),
                _marker: PhantomData,
            })
        }

        pub(super) fn with_global<F, R>(f: F) -> anyhow::Result<R>
        where F: FnOnce(&mut crate::test_vec_impl::TestVecEnv) -> anyhow::Result<R>
        {
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global().lock().expect("Global poisoned");
            f(test_vec_env_lock.as_mut().ok_or_else(|| anyhow::anyhow!("TestEnv not initialized."))?)
        }
    }
}

#[cfg(feature = "tls")]
mod storage {
    use std::cell::RefCell;
    use std::marker::PhantomData;
    use crate::test_vec_impl::TestVecEnv;

    thread_local! {
        static TEST_VEC_ENV: RefCell<Option<TestVecEnv>> = RefCell::new(None);
    }

    pub struct TlsEnvGuard {
        // Prevents Send implementation to ensure the guard is dropped in the same thread.
        _marker: PhantomData<*const ()>,
    }


    impl Drop for TlsEnvGuard {
        fn drop(&mut self) {
            TEST_VEC_ENV.with(|tls| {
                tls.replace(None);
            });
        }
    }

    impl TestVecEnv {
        pub(super) fn initialize_with(self) -> anyhow::Result<TlsEnvGuard> {
            TEST_VEC_ENV.with(|tv_env_cell| {
                tv_env_cell.replace(Some(self))
            });
            Ok(TlsEnvGuard {
                _marker: PhantomData,
            })
        }

        pub(super) fn with_global<F, R>(f: F) -> anyhow::Result<R>
        where F: FnOnce(&mut TestVecEnv) -> anyhow::Result<R>
        {
            TEST_VEC_ENV.with(|tv_env_cell| {
                let mut tv_env_borrowed = tv_env_cell.borrow_mut();
                let tv_env: &mut TestVecEnv = tv_env_borrowed.as_mut().ok_or_else(|| anyhow::anyhow!("TestEnv not initialized."))?;
                f(tv_env)
            })
        }
    }
}


impl TestVectorData {
    fn load_from_file<T: Into<PathBuf>>(tv_file_path: T, file_format: TestVectorFileFormat) -> anyhow::Result<Self> {
        let tv_file_path = tv_file_path.into();
        
        let tv_file = std::fs::File::open(tv_file_path.clone())
            .map_err(|e| anyhow::anyhow!("Failed to open test vector file ({:?}): {}", tv_file_path, e))?;
        let tv_data: TestVectorData = match file_format {
            TestVectorFileFormat::Json => {
                serde_json::from_reader(tv_file)
                    .map_err(|e| anyhow::anyhow!("Failed to parse test vector file ({:?}) as json: {}", tv_file_path, e))?
            },
            TestVectorFileFormat::Yaml => {
                serde_yaml::from_reader(tv_file)
                    .map_err(|e| anyhow::anyhow!("Failed to parse test vector file ({:?}) as yaml: {}", tv_file_path, e))?
            }
        };
        Ok(tv_data)
    }

    fn store_to_file<T: Into<PathBuf>>(&self, tv_file_path: T, file_format: TestVectorFileFormat) -> anyhow::Result<()> {
        let tv_file_path = tv_file_path.into();
        if let Some(parent) = tv_file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create parent directories for test vector file ({:?}): {}", tv_file_path, e))?;
        }
        let tv_file = std::fs::File::create(tv_file_path)
            .map_err(|e| anyhow::anyhow!("Failed to create test vector file: {}", e))?;
        match file_format {
            TestVectorFileFormat::Json => {
                serde_json::to_writer_pretty(tv_file, &self)
                    .map_err(|e| anyhow::anyhow!("Failed to write test vector file as json: {}", e))?
            },
            TestVectorFileFormat::Yaml => {
                serde_yaml::to_writer(tv_file, &self)
                    .map_err(|e| anyhow::anyhow!("Failed to write test vector file as yaml: {}", e))?
            }
        };
        Ok(())
    }
}


pub fn initialize_tv_case_from_file<T: Into<PathBuf>>(
    tv_file_path: T,
    file_format: TestVectorFileFormat,
    test_mode: TestMode
) -> anyhow::Result<TlsEnvGuard> {

    let tv_file_path: PathBuf = tv_file_path.into();
    let loaded_tv_data = match test_mode {
        TestMode::Init => {
            TestVectorData { entries: Vec::new() }
        }
        TestMode::Check => {
            TestVectorData::load_from_file(&tv_file_path, file_format)
                .map_err(|_| anyhow!("Error loading test vector. You may need to switch to init mode"))?
        }
    };
    let tv_env = TestVecEnv {
        tv_file_path,
        loaded_tv_data,
        recorded_tv_data: TestVectorData { entries: Vec::new() },
        file_format,
        test_mode
    };
    TestVecEnv::initialize_with(tv_env)
}

pub fn finalize_tv_case() -> anyhow::Result<()> {
    TestVecEnv::with_global(|tv_env| {
        match tv_env.test_mode {
            TestMode::Check => {
                // In check mode, test vectors are not updated
            }
            TestMode::Init => {
                // In both init mode, the test vector file is update if necessary
                let update_required =
                    tv_env.loaded_tv_data != tv_env.recorded_tv_data ||  // Test vectors have changed
                        !tv_env.tv_file_path.is_file();     // OR test vector file does not exist
                if update_required {
                    tv_env.recorded_tv_data.store_to_file(&tv_env.tv_file_path, tv_env.file_format)?;
                }
            }
        }
        Ok(())
    })
}

pub fn process_next_entry_infer_type<V: TestVectorMomento<Originator=V>>(
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    observed_value: &V::Originator,
    code_location: Option<String>) -> anyhow::Result<Option<V::Originator>> {
    process_next_entry::<V>(entry_type, description, name, observed_value, code_location)
}

pub fn process_next_entry<V: TestVectorMomento>(
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    observed_value: &V::Originator,
    code_location: Option<String>) -> anyhow::Result<Option<V::Originator>> {
    let value = V::serialize(observed_value)?;
    let observed_entry = TestVectorEntry {
        entry_type,
        description,
        name,
        value,
        code_location,
    };

    TestVecEnv::with_global(|tv_env| {
        let entry_index = tv_env.recorded_tv_data.entries.len();
        let loaded_entry = tv_env.loaded_tv_data.entries.get(entry_index).cloned();
        tv_env.recorded_tv_data.entries.push(
            observed_entry.clone()
        );
        match tv_env.test_mode {
            TestMode::Init => {
                // init mode ignores (doesn't check) all entries (passes it through to be stored)
                // Entry types of type const are however deserialized and returned anyway
                // This is done to have exact same behaviour as check mode, where consts are loaded and replaced
                match observed_entry.entry_type {
                    TestVectorEntryType::Const => {
                        Ok(Some(V::deserialize(&observed_entry.value)
                            .with_context(|| "Failed to deserialize constant value right after serializing it. \
                        There probably is a bug in the TestVectorMomento implementation")?))
                    }
                    TestVectorEntryType::Output => {
                        // Nothing will be outputted if the entry type is output (as there is nothing to be replaced
                        Ok(None)
                    }
                }
            }
            TestMode::Check => {
                let Some(loaded_entry) = loaded_entry else {
                    bail!("Observed value does not exist in loaded test vector: \n observed: {:?}", observed_entry)
                };
                let diff = || format!(
                    "\n\
                                     loaded name: {:?}\n\
                                   observed name: {:?}\n\
                                    loaded value: {:?}\n\
                                  observed value: {:?}\n\
                                    loaded entry_type: {:?}\n\
                                  observed entry_type: {:?}\n",
                    loaded_entry.name, observed_entry.name,
                    loaded_entry.value, observed_entry.value,
                    loaded_entry.entry_type, observed_entry.entry_type
                );
                // check entry types
                match observed_entry.entry_type {
                    TestVectorEntryType::Const |
                    TestVectorEntryType::Output => {

                        if loaded_entry.name != observed_entry.name {
                            bail!("Observed value does not match the loaded test vectors name:{}", diff())
                        }
                        if loaded_entry.entry_type != observed_entry.entry_type {
                            bail!("Observed value does not match the loaded test vectors type:{}", diff())
                        }
                    }
                }

                // check the value if it is output
                match loaded_entry.entry_type {
                    TestVectorEntryType::Const => {}
                    TestVectorEntryType::Output => {
                        if loaded_entry.value != observed_entry.value {
                            bail!("Observed value does not match the loaded test vectors value:{}", diff())
                        }
                    }
                };

                // Deserialize const values
                match loaded_entry.entry_type {
                    TestVectorEntryType::Const => {
                        V::deserialize(&loaded_entry.value).map(|v| Some(v))
                    }
                    TestVectorEntryType::Output => {
                        Ok(None)
                    }
                }

            }
        }
    })
}



#[macro_export]
macro_rules! process_tv_observation_const {
    (
        $observed_value:expr,
        $momento_type:ty,
        $name: expr,
        $description:expr,
        $code_location:expr,
    ) => {
        {
            #[allow(unused_braces)]
            {
                let value = &$observed_value;
                $crate::process_next_entry::<$momento_type>(
                    $crate::TestVectorEntryType::Const,
                    $description,
                    $name,
                    value,
                    Some($code_location),
                )
                    .expect("Error processing observed test vector value")
                    .expect("Unexpected error processing observed test vector const: no value was loaded")
            }
        }
    }
}

#[macro_export]
macro_rules! process_tv_observation_output {
    (
        $observed_value:expr,
        $momento_type:ty,
        $name: expr,
        $description:expr,
        $code_location:expr,
    ) => {
        {
            #[allow(unused_braces)]
            {
                let value = &$observed_value;
                $crate::process_next_entry::<$momento_type>(
                    $crate::TestVectorEntryType::Output,
                    $description,
                    $name,
                    value,
                    Some($code_location),
                )
                    .expect("Error processing observed test vector value");
            }
        }
    }
}

// Define helper functions so that the compiler can infer the momento type.
pub fn helper_infer_const<T: crate::TestVectorMomento<Originator = T>>(observed: T, name: Option<String>, description: Option<String>,
                                                                       code_location: String) -> T {
    crate::process_tv_observation_const!(observed, T, name, description, 
                    code_location,)
}
pub fn helper_infer_output<T: crate::TestVectorMomento<Originator = T>>(observed: T, name: Option<String>, description: Option<String>,
                                                                        code_location: String) {
    crate::process_tv_observation_output!(observed, T, name, description, 
                    code_location,)
}

#[macro_export]
macro_rules! tv_const {
    (
        $observed_value:expr,
        $momento_type:ty,
        $name: expr,
        $description:expr
    ) => {
        $crate::process_tv_observation_const!($observed_value, $momento_type, Some($name.into()), Some($description.into()), format!("{}:{}", file!(), line!()), )
    };
    // Version without description
    ($observed_value:expr, $momento_type:ty, $name:expr) => {
        $crate::process_tv_observation_const!($observed_value, $momento_type, Some($name.into()), None, format!("{}:{}", file!(), line!()), )
    };
    // Version without name and description
    ($observed_value:expr, $momento_type:ty) => {
        $crate::process_tv_observation_const!($observed_value, $momento_type, None, None, format!("{}:{}", file!(), line!()), )
    };
    // Version without momento_type
    // 3-argument version: we want to infer the type of the observed value.
    ($observed_value:expr, $name:expr, $description:expr) => {
        $crate::helper_infer_const($observed_value, Some($name.into()), Some($description.into()), format!("{}:{}", file!(), line!()))
    };
    // Version without description, and momento_type
    ($observed_value:expr, $name:expr) => {
        $crate::helper_infer_const($observed_value, Some($name.into()), None, format!("{}:{}", file!(), line!()))
    };
    // Version without name and description, and momento_type
    ($observed_value:expr) => {
        $crate::helper_infer_const($observed_value, None, None, format!("{}:{}", file!(), line!()))
    };
}

#[macro_export]
macro_rules! tv_output {
    (
        $observed_value:expr,
        $momento_type:ty,
        $name: expr,
        $description:expr
    ) => {
        $crate::process_tv_observation_output!($observed_value, $momento_type, Some($name.into()), Some($description.into()), format!("{}:{}", file!(), line!()),)
    };
    // Version without description
    ($observed_value:expr, $momento_type:ty, $name:expr) => {
        $crate::process_tv_observation_output!($observed_value, $momento_type, Some($name.into()), None, format!("{}:{}", file!(), line!()),)
    };
    // Version without name and description
    ($observed_value:expr, $momento_type:ty) => {
        $crate::process_tv_observation_output!($observed_value, $momento_type, None, None, format!("{}:{}", file!(), line!()),)
    };
    // Version without momento_type
    // 3-argument version: we want to infer the type of the observed value.
    ($observed_value:expr, $name:expr, $description:expr) => {
        $crate::helper_infer_output(&$observed_value, Some($name.into()), Some($description.into()), format!("{}:{}", file!(), line!()))
    };
    // Version without description, and momento_type
    ($observed_value:expr, $name:expr) => {
        $crate::helper_infer_output(&$observed_value, Some($name.into()), None, format!("{}:{}", file!(), line!()))
    };
    // Version without name and description, and momento_type
    ($observed_value:expr) => {
        $crate::helper_infer_output(&$observed_value, None, None, format!("{}:{}", file!(), line!()))
    };
}
