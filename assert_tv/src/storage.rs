#[cfg(not(feature = "tls"))]
pub(crate) mod storage_global {
    use std::marker::PhantomData;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    struct TestVecEnvSingleton(OnceLock<Mutex<Option<crate::test_vec_impl::TestVecEnv>>>);

    static TEST_VEC_ENV: TestVecEnvSingleton = TestVecEnvSingleton(OnceLock::new());
    // because it is a global env that all threads write to, we need a lock
    static GLOBAL_ENV_LOCK: Mutex<()> = Mutex::new(());

    pub struct TlsEnvGuard {
        // Prevents Send implementation to ensure the guard is dropped in the same thread.
        _marker: PhantomData<*const ()>,
        single_threaded_lock: Option<MutexGuard<'static, ()>>,
    }

    impl Drop for TlsEnvGuard {
        fn drop(&mut self) {
            drop(self.single_threaded_lock.take());
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global()
                .lock()
                .expect("Global poisoned");
            test_vec_env_lock.take();
        }
    }

    impl crate::test_vec_impl::TestVecEnv {
        pub(crate) fn get_global() -> &'static Mutex<Option<crate::test_vec_impl::TestVecEnv>> {
            TEST_VEC_ENV.0.get_or_init(|| Mutex::new(None))
        }

        pub(crate) fn initialize_with(self) -> anyhow::Result<TlsEnvGuard> {
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global()
                .lock()
                .expect("Global poisoned");
            test_vec_env_lock.replace(self);
            let global_lock_guard = GLOBAL_ENV_LOCK.lock().expect("Global env lock poisoned");
            Ok(TlsEnvGuard {
                single_threaded_lock: Some(global_lock_guard),
                _marker: PhantomData,
            })
        }

        pub(crate) fn with_global<F, R>(f: F) -> anyhow::Result<R>
        where
            F: FnOnce(&mut crate::test_vec_impl::TestVecEnv) -> anyhow::Result<R>,
        {
            let mut test_vec_env_lock = crate::test_vec_impl::TestVecEnv::get_global()
                .lock()
                .expect("Global poisoned");
            f(test_vec_env_lock
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("TestEnv not initialized."))?)
        }
    }
}

#[cfg(feature = "tls")]
pub(crate) mod tls_storage {
    use crate::TestVecEnv;
    use anyhow::bail;
    use std::cell::RefCell;
    use std::marker::PhantomData;

    thread_local! {
        static TEST_VEC_ENV: RefCell<Option<TestVecEnv>> = const { RefCell::new(None) };
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
        pub(crate) fn initialize_with(self) -> anyhow::Result<TlsEnvGuard> {
            let previous = TEST_VEC_ENV.replace(Some(self));
            if let Some(previous) = previous {
                bail!("Initialized a new test vector while a previous test vector is already initialized: {:?}",
                    previous.tv_file_path);
            }
            Ok(TlsEnvGuard {
                _marker: PhantomData,
            })
        }

        pub(crate) fn with_global<F, R>(f: F) -> anyhow::Result<R>
        where
            F: FnOnce(&mut TestVecEnv) -> anyhow::Result<R>,
        {
            TEST_VEC_ENV.with(|tv_env_cell| {
                let mut tv_env_borrowed = tv_env_cell.borrow_mut();
                let tv_env: &mut TestVecEnv = tv_env_borrowed
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("TestEnv not initialized."))?;
                f(tv_env)
            })
        }
    }
}
