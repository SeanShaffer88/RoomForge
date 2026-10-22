//! Length/area formatting and parsing for metric and imperial display.
//! Storage is always metres; this module only affects what the user
//! sees and types.

const INCH_M: f64 = 0.0254;

/// Format a length. Metric: "2.5 m" / "85 cm". Imperial: 10' 6.5".
pub fn fmt_len(m: f64, imperial: bool) -> String {
    if imperial {
        let total_in = m / INCH_M;
        let mut feet = (total_in / 12.0).floor();
        let mut inches = total_in - feet * 12.0;
        inches = (inches * 2.0).round() / 2.0; // nearest half inch
        if inches >= 12.0 && feet > 0.0 {
            feet += 1.0;
            inches = 0.0;
        }
        if feet == 0.0 {
            format!("{inches}\"")
        } else if inches == 0.0 {
            format!("{feet}'")
        } else {
            format!("{feet}' {inches}\"")
        }
    } else if m < 1.0 {
        format!("{:.0} cm", m * 100.0)
    } else {
        let s = format!("{m:.2}");
        format!("{} m", s.trim_end_matches('0').trim_end_matches('.'))
    }
}

/// Format an area. Metric: "20.0 m²". Imperial: "215.3 ft²".
pub fn fmt_area(m2: f64, imperial: bool) -> String {
    if imperial {
        format!("{:.1} ft²", m2 * 10.763_910_4)
    } else {
        format!("{m2:.1} m²")
    }
}

/// Parse a user-typed length into metres.
/// Metric accepts "2.5", "2.5 m", "250 cm", "2500 mm".
/// Imperial accepts 10'6", 10' 6.5", 10.5', 126", "6 1/2" (inches with
/// fraction after feet), or a bare number meaning feet.
pub fn parse_len(text: &str, imperial: bool) -> Option<f64> {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    if imperial {
        parse_imperial(&t)
    } else {
        let (num, mult) = if let Some(s) = t.strip_suffix("mm") {
            (s, 0.001)
        } else if let Some(s) = t.strip_suffix("cm") {
            (s, 0.01)
        } else if let Some(s) = t.strip_suffix('m') {
            (s, 1.0)
        } else {
            (t.as_str(), 1.0)
        };
        num.trim().parse::<f64>().ok().map(|v| v * mult)
    }
}

fn parse_inches_part(s: &str) -> Option<f64> {
    // "6.5", "6", or "6 1/2"
    let mut total = 0.0;
    for tok in s.split_whitespace() {
        if let Some((a, b)) = tok.split_once('/') {
            total += a.parse::<f64>().ok()? / b.parse::<f64>().ok()?;
        } else {
            total += tok.parse::<f64>().ok()?;
        }
    }
    Some(total)
}

fn parse_imperial(t: &str) -> Option<f64> {
    if let Some((feet_s, rest)) = t.split_once('\'') {
        let feet: f64 = feet_s.trim().parse().ok()?;
        let rest = rest.trim().trim_end_matches('"').trim();
        let inches = if rest.is_empty() {
            0.0
        } else {
            parse_inches_part(rest)?
        };
        Some((feet * 12.0 + inches) * INCH_M)
    } else if let Some(in_s) = t.strip_suffix('"') {
        Some(parse_inches_part(in_s.trim())? * INCH_M)
    } else {
        t.parse::<f64>().ok().map(|feet| feet * 12.0 * INCH_M)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_formats() {
        assert_eq!(fmt_len(2.5, false), "2.5 m");
        assert_eq!(fmt_len(0.85, false), "85 cm");
        assert_eq!(fmt_len(3.0, false), "3 m");
        assert_eq!(fmt_area(20.0, false), "20.0 m²");
    }

    #[test]
    fn imperial_formats() {
        assert_eq!(fmt_len(3.2, true), "10' 6\"");
        assert_eq!(fmt_len(0.3, true), "12\"");
        assert_eq!(fmt_area(20.0, true), "215.3 ft²");
    }

    #[test]
    fn parses_both_systems() {
        assert!((parse_len("250 cm", false).unwrap() - 2.5).abs() < 1e-9);
        assert!((parse_len("2.5", false).unwrap() - 2.5).abs() < 1e-9);
        assert!((parse_len("10'6\"", true).unwrap() - 3.2004).abs() < 1e-6);
        assert!((parse_len("10' 6 1/2\"", true).unwrap() - 3.2131).abs() < 1e-6);
        assert!((parse_len("126\"", true).unwrap() - 3.2004).abs() < 1e-6);
        assert!((parse_len("10.5", true).unwrap() - 3.2004).abs() < 1e-6);
        assert!(parse_len("abc", false).is_none());
        assert!(parse_len("", true).is_none());
    }

    #[test]
    fn format_parse_roundtrip() {
        for m in [0.5, 1.27, 2.5, 3.2, 5.0] {
            let back = parse_len(&fmt_len(m, true), true).unwrap();
            assert!(
                (back - m).abs() < 0.013,
                "{m} -> {} -> {back}",
                fmt_len(m, true)
            );
        }
    }
}
