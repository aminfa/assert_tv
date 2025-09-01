use core::panic;

use assert_tv::test_vec_case;

#[test_vec_case(mode = "init")]
#[should_panic(expected = "Attribute was not passed along, this test was supposed to fail")] // user attribute to be forwarded
fn forwarding_test() {
    // If #[should_panic] is NOT forwarded, this will FAIL the test.
    panic!("Attribute was not passed along, this test was supposed to fail");
}

#[ignore]
#[test_vec_case(mode = "init")]
fn forwarding_test_ignored() {
    panic!("Attribute was not passed along, should be ignored!")
}
