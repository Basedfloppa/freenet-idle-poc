//! Display formatting helpers shared between contract/delegate tests
//! and the frontend UI. Pure functions over `u64` — no allocation
//! beyond the returned `String`.

/// Engineering-notation formatter for unbounded counters (gold,
/// boss_damage, experience, wheat, …). The UI uses this everywhere a
/// `u64` would otherwise blow out a table cell after a long session.
///
/// Format rules:
///   * `n < 1_000`            → plain digits, e.g. `"999"`
///   * `n < 10` × scale       → one decimal, e.g. `"1.2k"`, `"4.5B"`
///   * otherwise               → integer + suffix, e.g. `"200k"`, `"1B"`
///
/// Suffixes: `k` 10³, `M` 10⁶, `B` 10⁹, `T` 10¹², `Q` 10¹⁵, `E` 10¹⁸.
/// `u64::MAX ≈ 18.4 E`, so every value fits in one of these slots.
pub fn format_si(n: u64) -> String {
    const UNITS: &[(u64, &str)] = &[
        (1_000_000_000_000_000_000, "E"),
        (1_000_000_000_000_000, "Q"),
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "k"),
    ];
    for (div, suffix) in UNITS {
        if n >= *div {
            let major = n / div;
            // Avoid floating point — compute one tenths-digit by
            // integer arithmetic so the result is bit-exact across
            // every target (WASM included).
            let tenths = (n % div) * 10 / div;
            return if major >= 10 || tenths == 0 {
                format!("{major}{suffix}")
            } else {
                format!("{major}.{tenths}{suffix}")
            };
        }
    }
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_expected_examples() {
        assert_eq!(format_si(0), "0");
        assert_eq!(format_si(99), "99");
        assert_eq!(format_si(999), "999");
        assert_eq!(format_si(1_000), "1k");
        assert_eq!(format_si(1_234), "1.2k");
        assert_eq!(format_si(9_999), "9.9k");
        assert_eq!(format_si(10_000), "10k");
        assert_eq!(format_si(200_000), "200k");
        assert_eq!(format_si(1_000_000), "1M");
        assert_eq!(format_si(4_500_000), "4.5M");
        assert_eq!(format_si(1_000_000_000), "1B");
        assert_eq!(format_si(1_500_000_000_000), "1.5T");
        // Doesn't panic at the top of the u64 range.
        let _ = format_si(u64::MAX);
    }
}
