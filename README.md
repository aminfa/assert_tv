# assert_tv: Test Vector Assertion Library for Rust

A Rust library designed to simplify testing with persistent test vectors, enabling automatic generation, validation, and updates of test case outputs.

## Features

- **Test Vector Macros**: Use `tv_const!`, `tv_intermediate!`, and `tv_output!` to capture inputs, intermediates, and outputs in tests.
- **Modes of Operation**:
    - **Init**: Generate test vector files from test runs.
    - **Check**: Validate runtime values against persisted test vectors.
    - **Record**: Update test vectors with new values during test runs.
- **Production Transparency**: Compiles to empty calls when the `enabled` feature is disabled (default).
- **Helper Macro**: Simplify test setup with `#[test_vec]` for automatic test vector management.

## Installation

Add `assert_tv` and `assert_tv_macros` to your `Cargo.toml`:

```toml

[dependencies]
assert_tv = { git = "https://github.com/aminfa/assert_tv" }

[dev-dependencies]
assert_tv = { git = "https://github.com/aminfa/assert_tv", features = ["enabled"] }
assert_tv_macros = { git = "https://github.com/aminfa/assert_tv" }
```
## Usage

### Basic Example

Integrate the library as follows:

```rust
use assert_tv::{tv_const, tv_output, tv_intermediate};

fn add(a: i32, b: i32) -> i32 {
    let sum = tv_intermediate!(a + b);
    sum
}

#[test_vec()] 
fn test_add() {
    let a = tv_const!(test, 2, "A", "First input");
    let b = tv_const!(test, 3, "B", "Second input");
    let result = add(a, b);
    tv_output!(test, result, "Result", "Final output");
}
```

The test macro `test_vec`, adds `#[test]` and `#[ignore]` attributes. 
To run the test, it is required to set the parallelism to 1.
Also, on first run, set the environment variable to `TEST_MODE=init`.

```bash
TEST_MODE=init cargo test -- --ignored --test-threads=1
```

In init mode running the test vector will be recorded from the runtime values and stored in your repository. 
The name of the test vector file is derived from the function name. 

In this `.test_vectors/test_add.yaml`:

```yaml
entries:
- entry_type: Const
  description: First input
  name: A
  value: 2
  code_location: example/src/main.rs:51
- entry_type: Const
  description: Second input
  name: B
  value: 3
  code_location: example/src/main.rs:52
- entry_type: Intermediate
  description: null
  name: null
  value: 5
  code_location: example/src/main.rs:45
- entry_type: Output
  description: Final output
  name: Result
  value: 5
  code_location: example/src/main.rs:54
```

Now, with environment variable `TEST_MODE=check`, running the test again, will load the test vectors from the file.
Intermediate values are replaced by the values found in the test vector. 
Output values loaded from the test vector are checked to be exactly equal to the observed runtime values. 

Here, if we change the value of Output to 6 and run `TEST_MODE=check cargo test` we will get a test error:

```
failures:

---- tests::test_add stdout ----
thread 'tests::test_add' panicked at example/src/main.rs:54:9:
Error processing observed test vector value: Observed value does not match the loaded test vectors value: 
   loaded: Number(6)
 observed: Number(5)

Stack backtrace:
```

## Modes Explained
1. Init Mode (`TEST_MODE="init"`):
Generates a test vector file (e.g., YAML) containing captured values.
Run once to create the initial test vectors. On repeated runs the test vector file will be completely overwritten.

2. Check Mode (`TEST_MODE="check"`):
This is the default mode, if the `TEST_MODE` environment variable is not defined.
Compares runtime values against the persisted test vectors.
Use in CI/CD to ensure consistency.

3. Record Mode (`TEST_MODE="RECORD"`):
Updates test vectors with new values, useful when intentional changes occur.

## Production Transparency
When the `enabled` feature-flag is disabled (default), all macros compile to no-ops, ensuring zero overhead in production:

```rust
// With `enabled` off: Expands to `let sum = a + b;`
let sum = tv_intermediate!(a + b, "sum");
```

See the way the dependency were defined above.

## Helper Macro #[test_vec]
Replace #[test] with #[test_vec] to automate test vector setup:

```rust
#[test_vec(mode = "check")] // Validate against existing test vectors
fn test_case() {
    // Test logic with tv_* macros
}
```
