use std::env;
use std::path::{PathBuf};
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use crate::{TestMode, TestVectorFileFormat};
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
    // Input,
    Const,
    Intermediate,
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
        TestMode::Record |
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
            TestMode::Init |
            TestMode::Record => {
                // In both init and record mode, the test vector file is update if necessary
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

pub fn process_next_entry<V: WrappedVal>(
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    observed_value: V,
    code_location: Option<String>,
    check_intermediate: bool) -> anyhow::Result<V::Original> {
    let value = observed_value.serialize()?;

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
                // init mode ignores (doesn't check) all entries (passes it through)
                // return the value that was observed
                Ok(observed_value.pop())
            }
            TestMode::Record => {
                // Simply check if type is matching
                // As long as the type and name matches, we allow overriding
                if let Some(loaded_entry) = loaded_entry {
                    let matches = loaded_entry.entry_type == observed_entry.entry_type &&
                        loaded_entry.name == observed_entry.name;
                    if !matches {
                        bail!("Observed value does not have the same type or name as the one that was loaded. \
                        In record mode we require that all loaded entries have types and names that match the ones that are loaded. \
                        A mismatch can happen, if the order of operations changes or new entries are added. \
                        If the test vector case changes are substantial, use the 'init' mode instead of the 'record' mode.\
                        ")
                    }
                }
                // if the type matches, we override and return the observed value
                Ok(observed_value.pop())
            }
            TestMode::Check => {
                match (observed_entry.entry_type, check_intermediate) {
                    // (TestVectorEntryType::Input, _) => {
                    //     // input values are ignored
                    // }
                    (TestVectorEntryType::Const, _) => {
                        if let Some(loaded_entry) = loaded_entry {
                            if loaded_entry.value != observed_entry.value {
                                log::warn!("Observed constant value \
                                does not equal the loaded value from the test vector. \n\
                                Loaded Entry: {:?}\n\
                                Observed Entry: {:?}",
                                    loaded_entry, observed_entry
                                )
                            }
                        } else {
                            log::warn!("Observed constant value does not exist in loaded test vector: \n observed: {:?}",
                                observed_entry
                            )
                        }
                        Ok(observed_value.pop())
                    }
                    (TestVectorEntryType::Intermediate, false) => {
                        // intermediate values are by default not checked (unless the check_intermediate is set to true)
                        let Some(loaded_entry) = loaded_entry else {
                            bail!("Intermediate value was not loaded (reached end of the list)")
                        };
                        if loaded_entry.name != observed_entry.name {
                            bail!("Observed value does not match the loaded test vectors name: \n   loaded: {:?}\n observed: {:?}", loaded_entry.name, observed_entry.name)
                        }
                        if loaded_entry.entry_type != observed_entry.entry_type {
                            bail!("Observed value does not match the loaded test vectors type: \n   loaded: {:?}\n observed: {:?}", loaded_entry.entry_type, observed_entry.entry_type)
                        }
                        V::deserialize(&loaded_entry.value)
                    }
                    (TestVectorEntryType::Intermediate, true)
                    | (TestVectorEntryType::Output, _) => {
                        if let Some(loaded_entry) = loaded_entry {
                           
                            if loaded_entry.name != observed_entry.name {
                                bail!("Observed value does not match the loaded test vectors name: \n   loaded: {:?}\n observed: {:?}", loaded_entry.name, observed_entry.name)
                            }
                            if loaded_entry.value != observed_entry.value {
                                bail!("Observed value does not match the loaded test vectors value: \n   loaded: {:?}\n observed: {:?}", loaded_entry.value, observed_entry.value)
                            }
                            if loaded_entry.entry_type != observed_entry.entry_type {
                                bail!("Observed value does not match the loaded test vectors type: \n   loaded: {:?}\n observed: {:?}", loaded_entry.entry_type, observed_entry.entry_type)
                            }
                            V::deserialize(&loaded_entry.value)
                        } else {
                            bail!("Observed value does not exist in loaded test vector: \n observed: {:?}",
                                   observed_entry
                            )
                        }
                    }
                }
            }
        }
    })
    
}

pub trait WrappedVal {
    type Original;

    fn serialize(&self) -> anyhow::Result<serde_json::Value>;

    fn deserialize(value: &serde_json::Value) -> anyhow::Result<Self::Original>;

    fn pop(self) -> Self::Original;
}

impl<T> WrappedVal for T
where
    T: Serialize + DeserializeOwned,
{
    type Original = Self;

    fn serialize(&self) -> anyhow::Result<serde_json::Value> {
        // Convert the value to a serde_json::Value, mapping errors using anyhow.
        serde_json::to_value(self).map_err(anyhow::Error::new)
    }

    fn deserialize(value: &serde_json::Value) -> anyhow::Result<Self::Original> {
        // We clone the value because from_value takes ownership.
        serde_json::from_value(value.clone()).map_err(anyhow::Error::new)
    }

    fn pop(self) -> Self::Original {
        // Simply return self, since Original is Self.
        self
    }
}