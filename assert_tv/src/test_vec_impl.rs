use std::cell::RefCell;
use std::env;
use std::marker::PhantomData;
use std::path::{PathBuf};
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::sync::MutexGuard;
use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use crate::{TestMode, TestVectorFileFormat};

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

struct TestVecEnvSingleton(
    OnceLock<Mutex<Option<TestVecEnv>>>
);

static TEST_VEC_ENV: TestVecEnvSingleton = TestVecEnvSingleton(OnceLock::new());

// thread_local! {
//     static TEST_VEC_ENV: RefCell<Option<TestVecEnv>> = RefCell::new(None);
// }


pub struct TlsEnvGuard {
    // Prevents Send implementation to ensure the guard is dropped in the same thread.
    _marker: PhantomData<*const ()>,
}


impl Drop for TlsEnvGuard {
    fn drop(&mut self) {
        let mut test_vec_env_lock = TestVecEnv::get_global().lock().expect("Global poisoned");
        test_vec_env_lock.take();
    }
}



// fn get_global_tv_env() -> &'static Mutex<Option<TestVecEnv>> {
//     TEST_VEC_ENV.with(|env| Mutex::new(None))
// }

impl TestVecEnv {
    fn get_global() -> &'static Mutex<Option<TestVecEnv>> {
        TEST_VEC_ENV.0.get_or_init(|| Mutex::new(None))
    }
    
    fn initialize_with(self) -> anyhow::Result<TlsEnvGuard> {
        let mut test_vec_env_lock = TestVecEnv::get_global().lock().expect("Global poisoned");
        test_vec_env_lock.replace(self);
        Ok(crate::test_vec_impl::TlsEnvGuard {
            _marker: PhantomData,
        })
    }
    
    fn with_global<F, R>(f: F) -> anyhow::Result<R>
    where F: FnOnce(&mut TestVecEnv) -> anyhow::Result<R>
    {
        let mut test_vec_env_lock = TestVecEnv::get_global().lock().expect("Global poisoned");
        f(test_vec_env_lock.as_mut().ok_or_else(|| anyhow::anyhow!("TestEnv not initialized."))?)
    }
}

impl TestVectorData {
    fn load_from_file<T: Into<PathBuf>>(tv_file_path: T, file_format: TestVectorFileFormat) -> anyhow::Result<Self> {
        let tv_file_path = tv_file_path.into();
        if let Some(parent) = tv_file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create parent directories for test vector file ({:?}): {}", tv_file_path, e))?;
        }
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
    if !is_single_threaded_test() {
        panic!("Error: Vector-based tests require a single-threaded environment. \
        Run the test with `--test-threads=1` or `RUST_TEST_THREADS`.")
    }
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

pub fn process_next_entry<V: Serialize + DeserializeOwned + 'static>(
    entry_type: TestVectorEntryType,
    description: Option<String>,
    name: Option<String>,
    observed_value: V,
    code_location: Option<String>,
    check_intermediate: bool) -> anyhow::Result<V> {

    // The implementation needs access to globals and requires the tests to be isolated from each other
    if !is_single_threaded_test() {
        return Ok(observed_value)
    }

    let observed_entry = TestVectorEntry {
        entry_type,
        description,
        name,
        value: serde_json::to_value(&observed_value).expect("Error serializing test vector value."),
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
                Ok(observed_value)
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
                Ok(observed_value)
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
                        Ok(observed_value)
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
                        serde_json::from_value(loaded_entry.value.clone())
                            .map_err(|e| anyhow!("Error deserializing loaded test vector value: {:?}. Error: {}", loaded_entry, e))
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
                            
                            serde_json::from_value(loaded_entry.value.clone())
                                .map_err(|e| anyhow!("Error deserializing loaded test vector value: {:?}. Error: {}", loaded_entry, e))
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


fn is_single_threaded_test() -> bool {
    // check if RUST_TEST_THREADS is set to 1
    if let Ok(Ok(num_threads)) = env::var("RUST_TEST_THREADS").map(|t| u32::from_str(&t)) {
        if num_threads == 1 {
            return true
        } else {
            return false
        }
    }
    // parse `--test-threads` from args
    // hand both way of setting the option: `--test-threads=1` or `--test-threads 1`
    let args: Vec<String> = env::args().collect();
    let mut test_threads: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--test-threads" && i + 1 < args.len() {
            test_threads = Some(&args[i + 1]);
            i += 1;
        } else if args[i].starts_with("--test-threads=") {
            let parts: Vec<&str> = args[i].splitn(2, '=').collect();
            if parts.len() == 2 {
                test_threads = Some(parts[1]);
            }
        }
        i += 1;
    }
    test_threads == Some("1")
}
