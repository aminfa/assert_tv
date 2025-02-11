# assert_tv: Test Vector Assertion Library for Rust

A Rust library designed to simplify testing with persistent test vectors, enabling automatic generation, validation, and updates of test case outputs.

## Features

- **Non-distruptive test vector integration**: Use `tv_if_enabled!` to conditionally integrate test vectors without affecting production code.
- **Test Vector Macros**: Use `tv_const!` and `tv_output!` to capture inputs, intermediates, and outputs in tests.
- **Modes of Operation**:
    - **Init**: Persist test vector files from observed runtime values.
    - **Check**: Validate runtime values against persisted test vectors.
- **Production Transparency**: Compiles to empty calls when the `enabled` feature is disabled (default).
- **Helper Macro**: Simplify test setup with `#[test_vec]` for automatic test vector management.

## Installation

Add `assert_tv` and `assert_tv_macros` to your `Cargo.toml`:

```toml

[dependencies]
assert_tv = { git = "https://github.com/aminfa/assert_tv" }

[dev-dependencies]
assert_tv_macros = { git = "https://github.com/aminfa/assert_tv" }
```
## Usage

Replace #[test] with #[test_vec] to create a test-vector-based test:

```rust
#[test_vec()] // Validate against existing test vectors
fn test_case() {
  // Test logic with tv_* macros
  
}
```

Use `tv_const!` when you want to define a test-vector value that is considered a constant. 
Constant values are injected back into the runtime when the test vector is checked. 
This can be used to de-randomize a value, for example those that are drawn randomly or time-based values.

Use `tc_output!` to specify an output that is considered a result. These value are considered deterministically calculated.
When test vectors are checked, runtime output values are compared to the output values that are stored in the test vector.

### Basic Example

```rust
use assert_tv::tv_const;

fn add_and_mask(a: i32, b: i32) -> i32 {
  let random_val = tv_const!(
            rand::random::<i32>(),
            "rand",
            "Random Value"
        );
  a.overflowing_add(b).0.overflowing_add(random_val).0
}

#[cfg(test)]
mod test {
  use assert_tv::tv_const;
  use assert_tv_macros::test_vec;
  use super::add_and_mask;

  #[test_vec(file="basic_example_tv.json", format="json")]
  fn test_add() {
    use assert_tv::{tv_const, tv_output};
    let a = tv_const!(
            2,
            "A",
            "First input");
    let b = tv_const!(3, "B", "Second input");
    let result = add_and_mask(a, b);
    tv_output!(result, "Result", "Final output");
  }
}
```

In this example a function `add_and_mask` is a function that usually adds a random value.
It is hard to test such a function in a black-box manner because it is not pure. 
With assert_tv it is possible to de-randomized this function.

The test macro `test_vec` creates a test-vector based function by adding `#[test]` attribute to the test. 
But it only conditionally compiles if `assert_tv/enabled` is enabled. 
Also, on first run, set the environment variable to `TEST_MODE=init`, to initialize a test vector file from runtime values.

```bash
TEST_MODE=init cargo test basic_example::test::test_add --features assert_tv/enabled -- --exact
```

In init mode running the test vector will be recorded from the runtime values and stored in your repository. 
If the name of the test vector file is not specified, it is derived from the function name. 

In this example it is stored under `basic_example_tv.json`:

```json
{
  "entries": [
    {
      "entry_type": "Const",
      "description": "First input",
      "name": "A",
      "value": 2,
      "code_location": "example/src/basic_example.rs:19"
    },
    {
      "entry_type": "Const",
      "description": "Second input",
      "name": "B",
      "value": 3,
      "code_location": "example/src/basic_example.rs:23"
    },
    {
      "entry_type": "Const",
      "description": "Random Value",
      "name": "rand",
      "value": 1711591467,
      "code_location": "example/src/basic_example.rs:8"
    },
    {
      "entry_type": "Output",
      "description": "Final output",
      "name": "Result",
      "value": 1711591472,
      "code_location": "example/src/basic_example.rs:25"
    }
  ]
}
```

Now, with environment variable `TEST_MODE=check`, running the test again, will load the test vectors from the file.
Const values are replaced by the values found in the test vector. 
Output values loaded from the test vector are checked to be exactly equal to the observed runtime values. 

Here, if we change the value of `rand` in the test vector to `8` and run `TEST_MODE=check cargo test` we will get a test error:

```
failures:

---- tests::test_add stdout ----
thread 'tests::test_add' panicked at example/src/main.rs:54:9:
Error processing observed test vector value: Observed value does not match the loaded test vectors value: 
   loaded: Number(1711591472)
 observed: Number(13)

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

## Production Transparency
When you add `assert_tv` to your dependencies and use the macros `tv_const!` and `tv_output!`,
they will expand into "blank" no-ops implementation. 
This makes sure the crate does not disrupt your production code and becomes transparent.

```rust
let a = 1;
let b = tv_const!(rand::random::<i32>());
let sum = a + b;
tv_output!(sum);
```

In production, without the feature `assert_tv/enabled`, expands to:

```
let a = 1;
let b = rand::random::<i32>();
let sum = a+b;
```

If you run cargo with `--features assert_tv/enabled`, then it expands into calls that assume a test_vec environment has been set up and performs the tests based on the test mode.

### Complex use case 
It is not always possible to integrate `tv_const!` in a non-disruptive way. 
In these cases, you can use the `tv_if_enabled!{ .. }` macro which only conditionally compiles an entire block if `assert_tv/enabled` feature has been set.

```rust
// production code:
let a = &mut vec![0;8];
a[..4].copy_from_slice(
    &[rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>()]
);
// tv integration:
tv_if_enabled! {
    let a_const = tv_const!(a[..4].as_ref().to_vec());
    a[..4].copy_from_slice(a_const.as_slice());
}
```

This will expand into:

```rust
// production code:
let a = &mut vec![0;8];
a[..4].copy_from_slice(
    &[rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>()]
);
// tv integration:
```

## Momento implementation

To serialize and deserialize runtime values, assert_tv requires values to implement `serde::Serialize` and `serde::DeserializeOwned`.
If you cannot implement these for your type (because it is a foreign type), or if you don't want to,
you may create a Momento type, which implements `TestVectorMomento` for the type you want to add to test vectors.

```rust
use anyhow::bail;
use serde_json::json;
use assert_tv::{tv_const, TestVectorMomento};
use crate::momento_example::foreign::Point;

mod foreign {
    use rand::distr::{Distribution, StandardUniform};
    use rand::Rng;

    pub struct Point {pub x: u32, pub y: u32}
    impl Distribution<Point> for StandardUniform {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Point {
            Point {
                x: rng.random(),
                y: rng.random()
            }
        }
    }
}

fn display_point_randomly(
    p: &mut Point
) {
    let displacement: Point = rand::random();

    let displacement = tv_const!(displacement, PointMomento);

    p.x = p.x.overflowing_add(displacement.x).0;
    p.y = p.y.overflowing_add(displacement.y).0;
}

struct PointMomento;

impl TestVectorMomento<Point> for PointMomento {
    fn serialize(original_value: &Point) -> anyhow::Result<serde_json::value::Value> {
        Ok(json!({
            "x": original_value.x,
            "y": original_value.y,
        }))
    }

    fn deserialize(value: &serde_json::value::Value) -> anyhow::Result<Point> {
        let Some(map) = value.as_object() else {
            bail!("expected an object")
        };
        let Some(Some(x)) = map.get("x").map(|y| y.as_u64()) else {
            bail!("field x is missing")
        };
        let Some(Some(y)) = map.get("y").map(|y| y.as_u64()) else {
            bail!("field y is missing")
        };
        Ok(Point{
            x: x as u32,
            y: y as u32
        })
    }
}
```
