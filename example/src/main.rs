use assert_tv::{tv_checked_intermediate};

fn main() {
    let r = c1(1, 2);
    println!("{:?}", r)
}

fn c1(x1: i32, x2: i32) -> i32 {
    let m = tv_checked_intermediate!(feature = "tv", x1*4);
    return m + x2;
}

#[cfg(test)]
mod tests {
    use assert_tv::{finalize_tv_case, initialize_tv_case_from_file, TestMode, TestVectorFileFormat, tv_const, tv_intermediate, tv_output};
    use assert_tv_macros::test_vec;
    use crate::{c1, main};

    #[test]
    fn it_works() {
        main();
    }
    
    #[test_vec(feature="tv", mode="init")]
    fn test_vector_case_2() -> Result<(), String> {
        let a = tv_const!(feature = "tv", 3);
        let b = tv_const!(feature = "tv", 4, "b", "b is the second input");
        let output = c1(a, b);
        tv_output!(test, output, "output", "");
        Ok(())
    }

    #[test]
    fn tv_test() {
        let _guard = initialize_tv_case_from_file(".test_vectors/tv.yaml", TestVectorFileFormat::Yaml, TestMode::Init).expect("Error initializing test vector case");
        let a = tv_const!(feature = "tv", 2, "a", "a is the first input");
        let b = tv_const!(feature = "tv", 3, "b", "b is the second input");
        let output = c1(a, b);
        tv_output!(test, output, "output", "");
        finalize_tv_case().expect("Error finalizing test vector case");
    }


    fn add(a: i32, b: i32) -> i32 {
        let sum = tv_intermediate!(feature = "tv", a + b);
        sum
    }

    #[test_vec(feature="tv", mode = "init", format = "json")] // Initialize test vectors on first run
    fn test_add() {
        let a = tv_const!(feature = "tv", 3, "A", "First input");
        let b = tv_const!(feature = "tv", 3, "B", "Second input");
        let result = add(a, b);
        tv_output!(test, result, "Result", "Final output");
    }

}