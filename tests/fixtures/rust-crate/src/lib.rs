//! Fixture for `.github/workflows/self-test.yml`.

/// Adds two numbers.
///
/// ```
/// assert_eq!(ci_rust_fixture::add(1, 2), 3);
/// ```
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// Only compiled with `--all-features`, which clippy in ci-rust.yml passes.
#[cfg(feature = "extra")]
pub fn double(a: u32) -> u32 {
    add(a, a)
}

#[cfg(test)]
mod tests {
    #[test]
    fn adds() {
        assert_eq!(super::add(2, 2), 4);
    }
}
