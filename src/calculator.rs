use crate::models::{FileMetadata, FileType, SearchResult};

#[derive(Debug, Clone, PartialEq)]
pub struct CalcOutput {
    pub display_title: String,
    pub subtitle: String,
    pub copy_value: String,
    pub kind: &'static str, // "Calculator" or "UnitConversion"
}

/// Evaluates a raw search query as a math expression, percentage calculation,
/// base conversion, or unit conversion. Returns a `SearchResult` if matched.
pub fn evaluate_query(raw_query: &str) -> Option<SearchResult> {
    let output = evaluate_raw(raw_query)?;
    Some(SearchResult {
        metadata: FileMetadata {
            id: None,
            name: output.display_title,
            extension: output.kind.to_string(),
            parent_folder: format!("{} — Press Enter to copy '{}'", output.subtitle, output.copy_value),
            full_path: format!("copy:{}", output.copy_value),
            modified_date: 0,
            size: 0,
            file_type: FileType::Shortcut,
        },
        score: 2.0,
        match_type: output.kind.to_string(),
        icon_base64: None,
    })
}

pub fn evaluate_raw(raw_query: &str) -> Option<CalcOutput> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return None;
    }

    let query = trimmed.strip_prefix('=').map(|s| s.trim()).unwrap_or(trimmed);
    let has_equals_prefix = trimmed.starts_with('=');

    // 1. Check Unit Conversion ("100 km in miles", "72f to c", "500 mb to gb")
    if let Some(conv) = try_unit_conversion(query) {
        return Some(conv);
    }

    // 2. Check Base Conversion ("255 in hex", "42 to binary", "0xff in dec", or bare "0xff" / "0b1010")
    if let Some(base_conv) = try_base_conversion(query) {
        return Some(base_conv);
    }

    // 3. Check Percentage Patterns ("20% of 450", "150 + 18%", "45 as % of 180")
    if let Some(pct) = try_percentage_query(query) {
        return Some(pct);
    }

    // 4. General Math Expression
    try_math_expression(query, has_equals_prefix)
}

fn format_number(val: f64) -> String {
    if !val.is_finite() {
        return String::new();
    }
    // Avoid "-0"
    let val = if val.abs() < 1e-12 { 0.0 } else { val };

    if (val - val.round()).abs() < 1e-9 && val.abs() < 1e15 {
        format!("{}", val.round() as i64)
    } else if val.abs() >= 1e12 || (val.abs() > 0.0 && val.abs() < 1e-5) {
        format!("{:.6e}", val)
    } else {
        // Format up to 8 decimal places and strip trailing zeros
        let s = format!("{:.8}", val);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        s.to_string()
    }
}

fn format_number_with_commas(num_str: &str) -> String {
    if num_str.contains('e') || num_str.contains('E') {
        return num_str.to_string();
    }
    let (sign, rest) = if let Some(stripped) = num_str.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", num_str)
    };
    let mut parts = rest.splitn(2, '.');
    let int_part = parts.next().unwrap_or("");
    let dec_part = parts.next();

    if int_part.len() <= 3 {
        return num_str.to_string();
    }

    let mut formatted_int = String::new();
    for (idx, ch) in int_part.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            formatted_int.push(',');
        }
        formatted_int.push(ch);
    }
    let formatted_int: String = formatted_int.chars().rev().collect();

    match dec_part {
        Some(dec) => format!("{}{}.{}", sign, formatted_int, dec),
        None => format!("{}{}", sign, formatted_int),
    }
}

// ==========================================
// 1. PERCENTAGE EXPRESSIONS
// ==========================================

fn eval_numeric_value(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(v) = trimmed.replace(',', "").parse::<f64>() {
        if v.is_finite() {
            return Some(v);
        }
    }
    let tokens = tokenize(trimmed)?;
    if tokens.is_empty() {
        return None;
    }
    let mut parser = Parser::new(tokens);
    let val = parser.parse_expr()?;
    if parser.pos == parser.tokens.len() && val.is_finite() {
        Some(val)
    } else {
        None
    }
}

fn rfind_top_level_op(s: &str, targets: &[char]) -> Option<(usize, char)> {
    let mut depth: i32 = 0;
    let mut found = None;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 && targets.contains(&ch) && idx > 0 => {
                // Ignore unary +/- at the very start of the expression
                found = Some((idx, ch));
            }
            _ => {}
        }
    }
    found
}

fn try_percentage_query(q: &str) -> Option<CalcOutput> {
    let lower = q.to_lowercase();

    // Pattern B: "<X> as % of <Y>" or "<X> is what % of <Y>" (check before "% of ")
    for sep in [" as % of ", " is what % of ", " as percent of "] {
        if let Some(idx) = lower.find(sep) {
            let left_str = lower[..idx].trim();
            let right_str = lower[idx + sep.len()..].trim();
            if let (Some(part), Some(total)) = (eval_numeric_value(left_str), eval_numeric_value(right_str)) {
                if total == 0.0 {
                    return None;
                }
                let pct = (part / total) * 100.0;
                let clean = format!("{}%", format_number(pct));
                return Some(CalcOutput {
                    display_title: format!("= {}", clean),
                    subtitle: format!("{} as percentage of {}", format_number(part), format_number(total)),
                    copy_value: clean,
                    kind: "Calculator",
                });
            }
        }
    }

    // Pattern A: "<X>% of <Y>"
    if let Some(idx) = lower.find("% of ") {
        let left_str = lower[..idx].trim();
        let right_str = lower[idx + 5..].trim();
        if let (Some(pct), Some(base)) = (eval_numeric_value(left_str), eval_numeric_value(right_str)) {
            let result = (pct / 100.0) * base;
            let clean = format_number(result);
            let pretty = format_number_with_commas(&clean);
            return Some(CalcOutput {
                display_title: format!("= {}", pretty),
                subtitle: format!("{}% of {}", format_number(pct), format_number_with_commas(&format_number(base))),
                copy_value: clean,
                kind: "Calculator",
            });
        }
    }

    // Pattern C: "<X> + <Y>%", "<X> - <Y>%", or "<X> * <Y>%"
    if let Some(stripped) = lower.strip_suffix('%') {
        if let Some((op_idx, op_ch)) = rfind_top_level_op(stripped, &['+', '-', '*']) {
            let base_str = stripped[..op_idx].trim();
            let pct_str = stripped[op_idx + op_ch.len_utf8()..].trim();
            if let (Some(base), Some(pct)) = (eval_numeric_value(base_str), eval_numeric_value(pct_str)) {
                let delta = base * (pct / 100.0);
                let (result, subtitle) = match op_ch {
                    '+' => (
                        base + delta,
                        format!("{} + {}% (+{})", format_number(base), format_number(pct), format_number(delta)),
                    ),
                    '-' => (
                        base - delta,
                        format!("{} - {}% (-{})", format_number(base), format_number(pct), format_number(delta)),
                    ),
                    '*' => (
                        delta,
                        format!("{}% of {}", format_number(pct), format_number_with_commas(&format_number(base))),
                    ),
                    _ => return None,
                };
                let clean = format_number(result);
                let pretty = format_number_with_commas(&clean);
                return Some(CalcOutput {
                    display_title: format!("= {}", pretty),
                    subtitle,
                    copy_value: clean,
                    kind: "Calculator",
                });
            }
        }
    }

    None
}

// ==========================================
// 2. NUMBER BASE CONVERSIONS
// ==========================================

fn try_base_conversion(q: &str) -> Option<CalcOutput> {
    let lower = q.trim().to_lowercase();

    // Bare hex literal: "0xff"
    if let Some(hex_part) = lower.strip_prefix("0x") {
        if !hex_part.is_empty() && hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            let val = i64::from_str_radix(hex_part, 16).ok()?;
            return Some(CalcOutput {
                display_title: format!("= {} (dec)", val),
                subtitle: format!("0x{} = {} decimal | 0b{:b} binary", hex_part.to_uppercase(), val, val),
                copy_value: val.to_string(),
                kind: "Calculator",
            });
        }
    }

    // Bare binary literal: "0b1010"
    if let Some(bin_part) = lower.strip_prefix("0b") {
        if !bin_part.is_empty() && bin_part.chars().all(|c| c == '0' || c == '1') {
            let val = i64::from_str_radix(bin_part, 2).ok()?;
            return Some(CalcOutput {
                display_title: format!("= {} (dec)", val),
                subtitle: format!("0b{} = {} decimal | 0x{:X} hex", bin_part, val, val),
                copy_value: val.to_string(),
                kind: "Calculator",
            });
        }
    }

    for sep in [" in ", " to ", " as "] {
        if let Some(idx) = lower.find(sep) {
            let val_str = lower[..idx].trim().replace([',', '_'], "");
            let target_base = lower[idx + sep.len()..].trim();

            let parsed_int = if let Some(h) = val_str.strip_prefix("0x") {
                i64::from_str_radix(h, 16).ok()?
            } else if let Some(b) = val_str.strip_prefix("0b") {
                i64::from_str_radix(b, 2).ok()?
            } else if let Some(o) = val_str.strip_prefix("0o") {
                i64::from_str_radix(o, 8).ok()?
            } else {
                val_str.parse::<i64>().ok()?
            };

            match target_base {
                "hex" | "hexadecimal" => {
                    let hex_out = format!("0x{:X}", parsed_int);
                    return Some(CalcOutput {
                        display_title: format!("= {}", hex_out),
                        subtitle: format!("{} in hexadecimal", parsed_int),
                        copy_value: hex_out,
                        kind: "Calculator",
                    });
                }
                "bin" | "binary" => {
                    let bin_out = format!("0b{:b}", parsed_int);
                    return Some(CalcOutput {
                        display_title: format!("= {}", bin_out),
                        subtitle: format!("{} in binary", parsed_int),
                        copy_value: bin_out,
                        kind: "Calculator",
                    });
                }
                "oct" | "octal" => {
                    let oct_out = format!("0o{:o}", parsed_int);
                    return Some(CalcOutput {
                        display_title: format!("= {}", oct_out),
                        subtitle: format!("{} in octal", parsed_int),
                        copy_value: oct_out,
                        kind: "Calculator",
                    });
                }
                "dec" | "decimal" => {
                    let dec_out = parsed_int.to_string();
                    return Some(CalcOutput {
                        display_title: format!("= {}", dec_out),
                        subtitle: format!("{} in decimal", val_str),
                        copy_value: dec_out,
                        kind: "Calculator",
                    });
                }
                _ => {}
            }
        }
    }

    None
}

// ==========================================
// 3. UNIT CONVERSIONS
// ==========================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitCategory {
    Length,
    Mass,
    Temperature,
    Data,
    Time,
    Speed,
    Volume,
    Area,
}

#[derive(Debug, Clone, Copy)]
struct UnitDef {
    category: UnitCategory,
    canonical_short: &'static str,
    canonical_plural: &'static str,
    /// Factor to convert 1 unit of this into the base unit of the category (ignored for Temperature)
    to_base_factor: f64,
}

fn lookup_unit(raw_unit: &str) -> Option<UnitDef> {
    let u = raw_unit.trim().to_lowercase();
    let u = u.trim_start_matches('°');
    match u {
        // Length (base: meters)
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "mm",
            canonical_plural: "millimeters",
            to_base_factor: 0.001,
        }),
        "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "cm",
            canonical_plural: "centimeters",
            to_base_factor: 0.01,
        }),
        "m" | "meter" | "meters" | "metre" | "metres" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "m",
            canonical_plural: "meters",
            to_base_factor: 1.0,
        }),
        "km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "km",
            canonical_plural: "kilometers",
            to_base_factor: 1000.0,
        }),
        "in" | "inch" | "inches" | "\"" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "in",
            canonical_plural: "inches",
            to_base_factor: 0.0254,
        }),
        "ft" | "foot" | "feet" | "'" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "ft",
            canonical_plural: "feet",
            to_base_factor: 0.3048,
        }),
        "yd" | "yds" | "yard" | "yards" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "yd",
            canonical_plural: "yards",
            to_base_factor: 0.9144,
        }),
        "mi" | "mile" | "miles" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "mi",
            canonical_plural: "miles",
            to_base_factor: 1609.344,
        }),
        "nmi" | "nautical mile" | "nautical miles" => Some(UnitDef {
            category: UnitCategory::Length,
            canonical_short: "nmi",
            canonical_plural: "nautical miles",
            to_base_factor: 1852.0,
        }),

        // Mass (base: grams)
        "mg" | "milligram" | "milligrams" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "mg",
            canonical_plural: "milligrams",
            to_base_factor: 0.001,
        }),
        "g" | "gram" | "grams" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "g",
            canonical_plural: "grams",
            to_base_factor: 1.0,
        }),
        "kg" | "kgs" | "kilogram" | "kilograms" | "kilo" | "kilos" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "kg",
            canonical_plural: "kilograms",
            to_base_factor: 1000.0,
        }),
        "oz" | "ounce" | "ounces" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "oz",
            canonical_plural: "ounces",
            to_base_factor: 28.349523125,
        }),
        "lb" | "lbs" | "pound" | "pounds" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "lb",
            canonical_plural: "pounds",
            to_base_factor: 453.59237,
        }),
        "st" | "stone" | "stones" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "st",
            canonical_plural: "stone",
            to_base_factor: 6350.29318,
        }),
        "t" | "ton" | "tons" | "tonne" | "tonnes" | "metric ton" | "metric tons" => Some(UnitDef {
            category: UnitCategory::Mass,
            canonical_short: "t",
            canonical_plural: "metric tons",
            to_base_factor: 1_000_000.0,
        }),

        // Temperature
        "c" | "celsius" | "centigrade" => Some(UnitDef {
            category: UnitCategory::Temperature,
            canonical_short: "°C",
            canonical_plural: "Celsius",
            to_base_factor: 1.0,
        }),
        "f" | "fahrenheit" => Some(UnitDef {
            category: UnitCategory::Temperature,
            canonical_short: "°F",
            canonical_plural: "Fahrenheit",
            to_base_factor: 1.0,
        }),
        "k" | "kelvin" => Some(UnitDef {
            category: UnitCategory::Temperature,
            canonical_short: "K",
            canonical_plural: "Kelvin",
            to_base_factor: 1.0,
        }),

        // Data Size (base: bytes)
        "b" | "byte" | "bytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "B",
            canonical_plural: "bytes",
            to_base_factor: 1.0,
        }),
        "kb" | "kilobyte" | "kilobytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "KB",
            canonical_plural: "kilobytes",
            to_base_factor: 1024.0,
        }),
        "mb" | "megabyte" | "megabytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "MB",
            canonical_plural: "megabytes",
            to_base_factor: 1024.0 * 1024.0,
        }),
        "gb" | "gigabyte" | "gigabytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "GB",
            canonical_plural: "gigabytes",
            to_base_factor: 1024.0 * 1024.0 * 1024.0,
        }),
        "tb" | "terabyte" | "terabytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "TB",
            canonical_plural: "terabytes",
            to_base_factor: 1024.0 * 1024.0 * 1024.0 * 1024.0,
        }),
        "pb" | "petabyte" | "petabytes" => Some(UnitDef {
            category: UnitCategory::Data,
            canonical_short: "PB",
            canonical_plural: "petabytes",
            to_base_factor: 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        }),

        // Time (base: seconds)
        "ms" | "millisecond" | "milliseconds" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "ms",
            canonical_plural: "milliseconds",
            to_base_factor: 0.001,
        }),
        "s" | "sec" | "secs" | "second" | "seconds" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "s",
            canonical_plural: "seconds",
            to_base_factor: 1.0,
        }),
        "min" | "mins" | "minute" | "minutes" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "min",
            canonical_plural: "minutes",
            to_base_factor: 60.0,
        }),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "hr",
            canonical_plural: "hours",
            to_base_factor: 3600.0,
        }),
        "d" | "day" | "days" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "days",
            canonical_plural: "days",
            to_base_factor: 86400.0,
        }),
        "wk" | "wks" | "week" | "weeks" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "weeks",
            canonical_plural: "weeks",
            to_base_factor: 604800.0,
        }),
        "yr" | "yrs" | "year" | "years" => Some(UnitDef {
            category: UnitCategory::Time,
            canonical_short: "years",
            canonical_plural: "years",
            to_base_factor: 31_557_600.0,
        }),

        // Speed (base: m/s)
        "mps" | "m/s" => Some(UnitDef {
            category: UnitCategory::Speed,
            canonical_short: "m/s",
            canonical_plural: "meters/second",
            to_base_factor: 1.0,
        }),
        "kmh" | "kph" | "km/h" => Some(UnitDef {
            category: UnitCategory::Speed,
            canonical_short: "km/h",
            canonical_plural: "kilometers/hour",
            to_base_factor: 1.0 / 3.6,
        }),
        "mph" => Some(UnitDef {
            category: UnitCategory::Speed,
            canonical_short: "mph",
            canonical_plural: "miles/hour",
            to_base_factor: 0.44704,
        }),
        "kn" | "knot" | "knots" => Some(UnitDef {
            category: UnitCategory::Speed,
            canonical_short: "knots",
            canonical_plural: "knots",
            to_base_factor: 0.514444,
        }),

        // Volume (base: milliliters)
        "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "mL",
            canonical_plural: "milliliters",
            to_base_factor: 1.0,
        }),
        "l" | "liter" | "liters" | "litre" | "litres" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "L",
            canonical_plural: "liters",
            to_base_factor: 1000.0,
        }),
        "tsp" | "teaspoon" | "teaspoons" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "tsp",
            canonical_plural: "teaspoons",
            to_base_factor: 4.92892159375,
        }),
        "tbsp" | "tablespoon" | "tablespoons" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "tbsp",
            canonical_plural: "tablespoons",
            to_base_factor: 14.78676478125,
        }),
        "floz" | "fl oz" | "fluid ounce" | "fluid ounces" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "fl oz",
            canonical_plural: "fluid ounces",
            to_base_factor: 29.5735295625,
        }),
        "cup" | "cups" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "cups",
            canonical_plural: "cups",
            to_base_factor: 236.5882365,
        }),
        "pt" | "pint" | "pints" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "pt",
            canonical_plural: "pints",
            to_base_factor: 473.176473,
        }),
        "qt" | "quart" | "quarts" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "qt",
            canonical_plural: "quarts",
            to_base_factor: 946.352946,
        }),
        "gal" | "gallon" | "gallons" => Some(UnitDef {
            category: UnitCategory::Volume,
            canonical_short: "gal",
            canonical_plural: "gallons",
            to_base_factor: 3785.411784,
        }),

        // Area (base: square meters)
        "sqm" | "m2" | "sq m" | "square meter" | "square meters" => Some(UnitDef {
            category: UnitCategory::Area,
            canonical_short: "m²",
            canonical_plural: "square meters",
            to_base_factor: 1.0,
        }),
        "sqkm" | "km2" | "sq km" | "square kilometer" | "square kilometers" => Some(UnitDef {
            category: UnitCategory::Area,
            canonical_short: "km²",
            canonical_plural: "square kilometers",
            to_base_factor: 1_000_000.0,
        }),
        "sqft" | "ft2" | "sq ft" | "square foot" | "square feet" => Some(UnitDef {
            category: UnitCategory::Area,
            canonical_short: "ft²",
            canonical_plural: "square feet",
            to_base_factor: 0.09290304,
        }),
        "acre" | "acres" => Some(UnitDef {
            category: UnitCategory::Area,
            canonical_short: "acres",
            canonical_plural: "acres",
            to_base_factor: 4046.8564224,
        }),
        "ha" | "hectare" | "hectares" => Some(UnitDef {
            category: UnitCategory::Area,
            canonical_short: "ha",
            canonical_plural: "hectares",
            to_base_factor: 10000.0,
        }),
        _ => None,
    }
}

fn convert_temperature(val: f64, from_short: &str, to_short: &str) -> f64 {
    let celsius = match from_short {
        "°C" => val,
        "°F" => (val - 32.0) * 5.0 / 9.0,
        "K" => val - 273.15,
        _ => val,
    };
    match to_short {
        "°C" => celsius,
        "°F" => celsius * 9.0 / 5.0 + 32.0,
        "K" => celsius + 273.15,
        _ => celsius,
    }
}

fn split_value_and_unit(s: &str) -> Option<(f64, &str)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // 1. Check if the trailing 2 whitespace-separated words form a known multi-word unit (e.g. "sq ft", "fl oz", "nautical miles")
    if let Some(last_space) = s.rfind(char::is_whitespace) {
        let before_last = s[..last_space].trim_end();
        if let Some(prev_space) = before_last.rfind(char::is_whitespace) {
            let expr_str = before_last[..prev_space].trim();
            let candidate_unit = s[prev_space..].trim();
            if !expr_str.is_empty() && lookup_unit(candidate_unit).is_some() {
                if let Some(val) = eval_numeric_value(expr_str) {
                    return Some((val, candidate_unit));
                }
            }
        }
    }

    // 2. Check if the last whitespace-separated word is a known unit (supports "(10 + 5) km", "2 * 512 mb", "100 km")
    if let Some(last_space) = s.rfind(char::is_whitespace) {
        let expr_str = s[..last_space].trim();
        let unit_str = s[last_space..].trim();
        if !expr_str.is_empty() && !unit_str.is_empty() && lookup_unit(unit_str).is_some() {
            if let Some(val) = eval_numeric_value(expr_str) {
                return Some((val, unit_str));
            }
        }
    }

    // 3. Fallback: attached unit suffix with no space (e.g. "32f", "100°C", "(10+5)km")
    let mut unit_start = s.len();
    for (idx, ch) in s.char_indices().rev() {
        if ch.is_ascii_alphabetic() || matches!(ch, '°' | '"' | '\'' | '/' | '²') {
            unit_start = idx;
        } else {
            break;
        }
    }

    if unit_start == 0 || unit_start >= s.len() {
        return None;
    }

    let expr_str = s[..unit_start].trim();
    let unit_str = s[unit_start..].trim();
    if expr_str.is_empty() || unit_str.is_empty() {
        return None;
    }

    let val = eval_numeric_value(expr_str)?;
    Some((val, unit_str))
}

fn try_unit_conversion(q: &str) -> Option<CalcOutput> {
    let lower = q.to_lowercase();
    for sep in [" in ", " to ", " as ", " -> "] {
        if let Some(idx) = lower.find(sep) {
            let left = &lower[..idx];
            let right = lower[idx + sep.len()..].trim();

            let (val, from_unit_str) = split_value_and_unit(left)?;
            let from_unit = lookup_unit(from_unit_str)?;
            let to_unit = lookup_unit(right)?;

            if from_unit.category != to_unit.category {
                return None;
            }

            let converted = if from_unit.category == UnitCategory::Temperature {
                convert_temperature(val, from_unit.canonical_short, to_unit.canonical_short)
            } else {
                let base_val = val * from_unit.to_base_factor;
                base_val / to_unit.to_base_factor
            };

            let clean_num = format_number(converted);
            let pretty_num = format_number_with_commas(&clean_num);

            return Some(CalcOutput {
                display_title: format!("= {} {}", pretty_num, to_unit.canonical_short),
                subtitle: format!(
                    "{} {} in {}",
                    format_number_with_commas(&format_number(val)),
                    from_unit.canonical_plural,
                    to_unit.canonical_plural
                ),
                copy_value: clean_num,
                kind: "UnitConversion",
            });
        }
    }
    None
}

// ==========================================
// 4. RECURSIVE DESCENT MATH PARSER
// ==========================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
    Ident(String),
}

fn tokenize(input: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' | '−' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' | '×' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    tokens.push(Token::Caret);
                    i += 2;
                } else {
                    tokens.push(Token::Star);
                    i += 1;
                }
            }
            '/' | '÷' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            'π' => {
                tokens.push(Token::Ident("pi".to_string()));
                i += 1;
            }
            _ if c.is_ascii_digit() || c == '.' => {
                // Check for 0x or 0b prefix
                if c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                    i += 2;
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_hexdigit() {
                        i += 1;
                    }
                    if start == i {
                        return None;
                    }
                    let hex_str: String = chars[start..i].iter().collect();
                    let val = i64::from_str_radix(&hex_str, 16).ok()? as f64;
                    tokens.push(Token::Number(val));
                } else if c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'b' || chars[i + 1] == 'B') {
                    i += 2;
                    let start = i;
                    while i < chars.len() && (chars[i] == '0' || chars[i] == '1') {
                        i += 1;
                    }
                    if start == i {
                        return None;
                    }
                    let bin_str: String = chars[start..i].iter().collect();
                    let val = i64::from_str_radix(&bin_str, 2).ok()? as f64;
                    tokens.push(Token::Number(val));
                } else {
                    let mut num_str = String::new();
                    let mut dot_count = 0;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',' || chars[i] == '_') {
                        if chars[i] == '.' {
                            dot_count += 1;
                            if dot_count > 1 {
                                return None;
                            }
                            num_str.push('.');
                        } else if chars[i].is_ascii_digit() {
                            num_str.push(chars[i]);
                        }
                        i += 1;
                    }
                    if num_str.is_empty() || num_str == "." {
                        return None;
                    }
                    let val = num_str.parse::<f64>().ok()?;
                    tokens.push(Token::Number(val));
                }
            }
            _ if c.is_ascii_alphabetic() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect::<String>().to_lowercase();
                if !is_known_ident(&ident) {
                    return None;
                }
                tokens.push(Token::Ident(ident));
            }
            _ => return None,
        }
    }

    // Insert implicit multiplication where appropriate:
    // e.g. `2(3+4)` -> `2 * (3+4)`, `(2+3)(4+5)` -> `(2+3) * (4+5)`, `2 pi` -> `2 * pi`
    let mut expanded = Vec::with_capacity(tokens.len() * 2);
    for idx in 0..tokens.len() {
        if idx > 0 {
            let prev = &tokens[idx - 1];
            let curr = &tokens[idx];
            let need_mul = matches!(
                (prev, curr),
                (Token::Number(_), Token::LParen)
                    | (Token::RParen, Token::LParen)
                    | (Token::RParen, Token::Number(_))
                    | (Token::Number(_), Token::Ident(_))
                    | (Token::RParen, Token::Ident(_))
            );
            if need_mul {
                expanded.push(Token::Star);
            }
        }
        expanded.push(tokens[idx].clone());
    }

    Some(expanded)
}

fn is_known_ident(ident: &str) -> bool {
    matches!(
        ident,
        "pi" | "e" | "sqrt" | "abs" | "round" | "floor" | "ceil" | "sin" | "cos" | "tan" | "ln" | "log" | "log10" | "log2"
    )
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    // expr -> term (( '+' | '-' ) term)*
    fn parse_expr(&mut self) -> Option<f64> {
        let mut val = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    val += self.parse_term()?;
                }
                Token::Minus => {
                    self.next();
                    val -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Some(val)
    }

    // term -> power (( '*' | '/' | '%' ) power)*
    fn parse_term(&mut self) -> Option<f64> {
        let mut val = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.next();
                    val *= self.parse_power()?;
                }
                Token::Slash => {
                    self.next();
                    let denom = self.parse_power()?;
                    if denom == 0.0 {
                        return None;
                    }
                    val /= denom;
                }
                Token::Percent => {
                    self.next();
                    let denom = self.parse_power()?;
                    if denom == 0.0 {
                        return None;
                    }
                    val %= denom;
                }
                _ => break,
            }
        }
        Some(val)
    }

    // power -> unary ( '^' power )?  (right-associative)
    fn parse_power(&mut self) -> Option<f64> {
        let base = self.parse_unary()?;
        if matches!(self.peek(), Some(Token::Caret)) {
            self.next();
            let exp = self.parse_power()?;
            Some(base.powf(exp))
        } else {
            Some(base)
        }
    }

    // unary -> ('+' | '-') unary | primary
    fn parse_unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(Token::Plus) => {
                self.next();
                self.parse_unary()
            }
            Some(Token::Minus) => {
                self.next();
                Some(-self.parse_unary()?)
            }
            _ => self.parse_primary(),
        }
    }

    // primary -> NUMBER | '(' expr ')' | IDENT ('(' expr ')')?
    fn parse_primary(&mut self) -> Option<f64> {
        match self.next()? {
            Token::Number(n) => Some(n),
            Token::LParen => {
                let val = self.parse_expr()?;
                if !matches!(self.next(), Some(Token::RParen)) {
                    return None;
                }
                Some(val)
            }
            Token::Ident(name) => match name.as_str() {
                "pi" => Some(std::f64::consts::PI),
                "e" => Some(std::f64::consts::E),
                func => {
                    // Support both `sqrt(16)` and `sqrt 16`
                    let arg = if matches!(self.peek(), Some(Token::LParen)) {
                        self.next();
                        let v = self.parse_expr()?;
                        if !matches!(self.next(), Some(Token::RParen)) {
                            return None;
                        }
                        v
                    } else {
                        self.parse_unary()?
                    };

                    match func {
                        "sqrt" => {
                            if arg < 0.0 {
                                return None;
                            }
                            Some(arg.sqrt())
                        }
                        "abs" => Some(arg.abs()),
                        "round" => Some(arg.round()),
                        "floor" => Some(arg.floor()),
                        "ceil" => Some(arg.ceil()),
                        "sin" => Some(arg.sin()),
                        "cos" => Some(arg.cos()),
                        "tan" => Some(arg.tan()),
                        "ln" => {
                            if arg <= 0.0 {
                                return None;
                            }
                            Some(arg.ln())
                        }
                        "log" | "log10" => {
                            if arg <= 0.0 {
                                return None;
                            }
                            Some(arg.log10())
                        }
                        "log2" => {
                            if arg <= 0.0 {
                                return None;
                            }
                            Some(arg.log2())
                        }
                        _ => None,
                    }
                }
            },
            _ => None,
        }
    }
}

fn try_math_expression(q: &str, has_equals_prefix: bool) -> Option<CalcOutput> {
    let tokens = tokenize(q)?;
    if tokens.is_empty() {
        return None;
    }

    // Reject single bare constant identifier ("e" or "pi") unless prefixed with '='
    // so typing "e" (for Edge/Explorer) or "pi" (for Pictures) doesn't hijack file/app search.
    if !has_equals_prefix && tokens.len() == 1 {
        if let Token::Ident(ref name) = tokens[0] {
            if name == "e" || name == "pi" {
                return None;
            }
        }
    }

    // Must contain an operator or function call unless prefixed with '='
    let has_op_or_func = tokens.iter().any(|t| {
        matches!(
            t,
            Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::Caret
                | Token::Ident(_)
        )
    });

    if !has_equals_prefix && !has_op_or_func {
        return None;
    }

    // Reject single negative number like "-42" unless prefixed with '='
    if !has_equals_prefix && tokens.len() <= 2 && matches!(tokens[0], Token::Minus | Token::Plus) && matches!(tokens.get(1), Some(Token::Number(_))) {
        return None;
    }

    let mut parser = Parser::new(tokens);
    let result = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() || !result.is_finite() {
        return None;
    }

    let clean = format_number(result);
    if clean.is_empty() {
        return None;
    }
    let pretty = format_number_with_commas(&clean);

    Some(CalcOutput {
        display_title: format!("= {}", pretty),
        subtitle: format!("{} = {}", q.trim(), pretty),
        copy_value: clean,
        kind: "Calculator",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let res = evaluate_raw("2 + 2").unwrap();
        assert_eq!(res.copy_value, "4");
        assert_eq!(res.display_title, "= 4");

        let res2 = evaluate_raw("125 * 8").unwrap();
        assert_eq!(res2.copy_value, "1000");
        assert_eq!(res2.display_title, "= 1,000");

        let res3 = evaluate_raw("(10 + 5) * 3 - 9 / 3").unwrap();
        assert_eq!(res3.copy_value, "42");

        let res4 = evaluate_raw("2^10").unwrap();
        assert_eq!(res4.copy_value, "1024");

        let res5 = evaluate_raw("2(3 + 4)").unwrap();
        assert_eq!(res5.copy_value, "14");
    }

    #[test]
    fn test_math_functions_and_constants() {
        let res = evaluate_raw("sqrt(144)").unwrap();
        assert_eq!(res.copy_value, "12");

        let res2 = evaluate_raw("log2(1024)").unwrap();
        assert_eq!(res2.copy_value, "10");

        let res3 = evaluate_raw("round(3.14159 * 100) / 100").unwrap();
        assert_eq!(res3.copy_value, "3.14");

        // Constants work when part of an expression or prefixed with '='
        let res4 = evaluate_raw("= pi").unwrap();
        assert!(res4.copy_value.starts_with("3.14159"));
        let res5 = evaluate_raw("2 * pi").unwrap();
        assert!(res5.copy_value.starts_with("6.28318"));
    }

    #[test]
    fn test_percentages() {
        let res = evaluate_raw("20% of 450").unwrap();
        assert_eq!(res.copy_value, "90");

        let res2 = evaluate_raw("150 + 18%").unwrap();
        assert_eq!(res2.copy_value, "177");

        let res3 = evaluate_raw("200 - 15%").unwrap();
        assert_eq!(res3.copy_value, "170");

        let res4 = evaluate_raw("45 as % of 180").unwrap();
        assert_eq!(res4.copy_value, "25%");

        let res5 = evaluate_raw("(100 + 50) + 18%").unwrap();
        assert_eq!(res5.copy_value, "177");

        let res6 = evaluate_raw("100 * 20%").unwrap();
        assert_eq!(res6.copy_value, "20");
    }

    #[test]
    fn test_unit_conversions() {
        let km_mi = evaluate_raw("100 km in miles").unwrap();
        assert!(km_mi.copy_value.starts_with("62.137"));
        assert_eq!(km_mi.kind, "UnitConversion");

        let expr_km = evaluate_raw("(10 + 5) km in miles").unwrap();
        assert!(expr_km.copy_value.starts_with("9.3205"));
        assert_eq!(expr_km.kind, "UnitConversion");

        let expr_data = evaluate_raw("2 * 512 mb to gb").unwrap();
        assert_eq!(expr_data.copy_value, "1");
        assert_eq!(expr_data.display_title, "= 1 GB");

        let temp1 = evaluate_raw("100 c to f").unwrap();
        assert_eq!(temp1.copy_value, "212");
        assert_eq!(temp1.display_title, "= 212 °F");

        let temp2 = evaluate_raw("32f in c").unwrap();
        assert_eq!(temp2.copy_value, "0");

        let data = evaluate_raw("2048 mb to gb").unwrap();
        assert_eq!(data.copy_value, "2");
        assert_eq!(data.display_title, "= 2 GB");

        let weight = evaluate_raw("10 kg in lbs").unwrap();
        assert!(weight.copy_value.starts_with("22.046"));

        let multi_word = evaluate_raw("100 fl oz in ml").unwrap();
        assert!(multi_word.copy_value.starts_with("2957.35"));
    }

    #[test]
    fn test_base_conversions() {
        let hex = evaluate_raw("255 in hex").unwrap();
        assert_eq!(hex.copy_value, "0xFF");

        let bin = evaluate_raw("42 in binary").unwrap();
        assert_eq!(bin.copy_value, "0b101010");

        let dec = evaluate_raw("0xff").unwrap();
        assert_eq!(dec.copy_value, "255");
    }

    #[test]
    fn test_non_math_queries_rejected() {
        assert!(evaluate_raw("chrome").is_none());
        assert!(evaluate_raw("e").is_none(), "Bare 'e' should not hijack app search");
        assert!(evaluate_raw("pi").is_none(), "Bare 'pi' should not hijack app search");
        assert!(evaluate_raw("42").is_none());
        assert!(evaluate_raw("-42").is_none());
        assert!(evaluate_raw("report-2024").is_none());
        assert!(evaluate_raw("10 / 0").is_none());
        assert!(evaluate_raw("100 km in gallons").is_none());
        // Multi-byte Unicode characters whose lowercase expands byte length must never panic
        assert!(evaluate_raw("İ to İ").is_none());
        assert!(evaluate_raw("İ in km").is_none());
    }
}
