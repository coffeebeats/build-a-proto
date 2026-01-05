/* -------------------------------------------------------------------------- */
/*                        Function: assert_contains_all                       */
/* -------------------------------------------------------------------------- */

/// Asserts that content contains all expected substrings.
pub fn assert_contains_all(content: &str, expected: &[&str]) {
    for substring in expected {
        assert!(
            content.contains(substring),
            "Expected content to contain '{}', but it was not found.\nContent:\n{}",
            substring,
            content
        );
    }
}

/* -------------------------------------------------------------------------- */
/*                        Function: assert_not_contains                       */
/* -------------------------------------------------------------------------- */

/// Asserts that content does not contain any forbidden substrings.
#[allow(dead_code)]
pub fn assert_not_contains(content: &str, forbidden: &[&str]) {
    for substring in forbidden {
        assert!(
            !content.contains(substring),
            "Content should not contain '{}', but it was found.\nContent:\n{}",
            substring,
            content
        );
    }
}

/* -------------------------------------------------------------------------- */
/*                       Function: assert_error_contains                      */
/* -------------------------------------------------------------------------- */

/// Asserts that an error message contains expected diagnostic text.
pub fn assert_error_contains(error: &anyhow::Error, expected: &str) {
    let error_msg = format!("{:?}", error);
    assert!(
        error_msg.contains(expected),
        "Expected error to contain '{}', but got:\n{}",
        expected,
        error_msg
    );
}
