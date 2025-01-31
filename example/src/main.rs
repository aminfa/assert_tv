use assert_tv::{tv_checked_intermediate, tv_intermediate};

fn main() {
    // tv_record!(
    //     test,
    //     let x = { 1 },
    //     let x = { 2 }
    // );




    // load_test_vector!(
    //     test,
    //     let x = { 1 },
    //     let y = { 2 },
    // )
    let r = c1(1, 2);
    println!("{:?}", r)

}

fn c1(x1: i32, x2: i32) -> i32 {
    let m = tv_checked_intermediate!(test, x1*4, "m", "Some intermediate value");
    return m + x2;
}




#[cfg(test)]
mod tests {
    use assert_tv::{finalize_tv_case, initialize_tv_case_from_file, TestMode, TestVectorFileFormat, tv_const, tv_output};
    use assert_tv_macros::test_vec;
    use crate::{c1, main};

    #[test]
    fn it_works() {
        main();
    }
    
    #[test_vec(mode="init")]
    fn test_vector_case_2() {
        let a = tv_const!(test, 3, "a", "a is the first input");
        let b = tv_const!(test, 4, "b", "b is the second input");
        let output = c1(a, b);
        tv_output!(test, output, "output", "");
    }

    #[test]
    fn tv_test() {
        let _guard = initialize_tv_case_from_file(".test_vectors/tv.yaml", TestVectorFileFormat::Yaml, TestMode::Init).expect("Error initializing test vector case");
        let a = tv_const!(test, 2, "a", "a is the first input");
        let b = tv_const!(test, 3, "b", "b is the second input");
        let output = c1(a, b);
        tv_output!(test, output, "output", "");
        finalize_tv_case().expect("Error finalizing test vector case");
    }
}