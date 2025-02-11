use assert_tv::{tv_const, tv_if_enabled, tv_output};

fn add_and_mask(a: i32, b: i32) -> i32 {
    let random_val = tv_const!(
            rand::random::<i32>(),
            "rand",
            "Random Value"
        );
    a.overflowing_add(b).0.overflowing_add(random_val).0
}

fn example(a: &mut[u8]) {
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
    let a = 1;
    let b = tv_const!(rand::random::<i32>());
    let sum = a + b;
    tv_output!(sum);
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