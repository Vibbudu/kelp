use crate::models::{FileMetadata, FileType, SearchResult};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static RNG_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates pseudo-random 64-bit values seeded by OS RandomState, system clock, and an atomic counter.
fn next_random_u64() -> u64 {
    let count = RNG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(nanos ^ count.wrapping_mul(0x9E3779B97F4A7C15));
    hasher.finish()
}

/// Generates an RFC 4122 v4 UUID string.
pub fn generate_uuid_v4() -> String {
    let hi = next_random_u64();
    let lo = next_random_u64();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&hi.to_le_bytes());
    bytes[8..].copy_from_slice(&lo.to_le_bytes());

    // Set version to 4 (0100xxxx)
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant to RFC 4122 (10xxxxxx)
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// Converts a Unix timestamp (seconds) into a formatted UTC string `YYYY-MM-DD HH:MM:SS UTC`.
pub fn format_unix_utc(secs: i64) -> String {
    // Civil date algorithm (Howard Hinnant's algorithm)
    let days = secs.div_euclid(86400);
    let rem_secs = secs.rem_euclid(86400);

    let hour = rem_secs / 3600;
    let minute = (rem_secs % 3600) / 60;
    let second = rem_secs % 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", year, m, d, hour, minute, second)
}

/// Decodes a Base64 string into bytes.
pub fn from_base64(input: &str) -> Option<Vec<u8>> {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if clean.is_empty() || !clean.len().is_multiple_of(4) {
        return None;
    }

    fn val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }

    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let c0 = val(chunk[0])?;
        let c1 = val(chunk[1])?;
        let n0 = (c0 << 2) | (c1 >> 4);
        out.push(n0);

        if chunk[2] != b'=' {
            let c2 = val(chunk[2])?;
            let n1 = ((c1 & 0x0f) << 4) | (c2 >> 2);
            out.push(n1);

            if chunk[3] != b'=' {
                let c3 = val(chunk[3])?;
                let n2 = ((c2 & 0x03) << 6) | c3;
                out.push(n2);
            }
        }
    }

    Some(out)
}

/// URL percent-encodes a string.
pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Decodes a URL percent-encoded string.
pub fn url_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let val = u8::from_str_radix(hex, 16).ok()?;
            out.push(val);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Converts RGB (0..=255) to HSL (h: 0..360, s: 0..100, l: 0..100).
pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-6 {
        return (0, 0, (l * 100.0).round() as u8);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - rf).abs() < 1e-6 {
        let mut t = (gf - bf) / d;
        if gf < bf {
            t += 6.0;
        }
        t
    } else if (max - gf).abs() < 1e-6 {
        (bf - rf) / d + 2.0
    } else {
        (rf - gf) / d + 4.0
    } / 6.0;

    (
        (h * 360.0).round() as u16 % 360,
        (s * 100.0).round() as u8,
        (l * 100.0).round() as u8,
    )
}

/// Gets the local IPv4 address using a non-blocking UDP routing lookup (zero packets sent).
pub fn get_local_ipv4() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

fn make_tool_result(title: String, subtitle: String, copy_val: String, icon: &str) -> SearchResult {
    SearchResult {
        metadata: FileMetadata {
            id: None,
            name: title,
            // Store the Material Symbol icon name in `extension` so the UI renders the exact icon
            extension: icon.to_string(),
            parent_folder: format!("{} — Press Enter to copy", subtitle),
            full_path: format!("copy:{}", copy_val),
            modified_date: 0,
            size: 0,
            file_type: FileType::Shortcut,
        },
        score: 1.9,
        match_type: "QuickTool".to_string(),
        icon_base64: None,
    }
}

/// Returns true if the query invokes a non-deterministic generator (UUID, current timestamp,
/// password generator, coin flip, or dice roll) that must bypass the exact result cache.
pub fn is_dynamic_query(raw_query: &str) -> bool {
    let lower = raw_query.trim().to_lowercase();
    matches!(
        lower.as_str(),
        "uuid"
            | "guid"
            | "new uuid"
            | "generate uuid"
            | "gen uuid"
            | "uuid4"
            | "uuid v4"
            | "now"
            | "utc"
            | "timestamp"
            | "epoch"
            | "unix time"
            | "unix timestamp"
            | "epoch time"
            | "password"
            | "pass"
            | "pw"
            | "gen password"
            | "gen pass"
            | "generate password"
            | "flip coin"
            | "coin flip"
            | "toss coin"
            | "roll dice"
            | "dice"
            | "roll a die"
    ) || lower.starts_with("password ")
        || lower.starts_with("pass ")
        || lower.starts_with("pw ")
        || parse_dice_spec(&lower).is_some()
}

/// Parses dice notation like `"roll d20"`, `"roll 2d6"`, `"d20"`, `"2d6"` into `(count, sides)`.
fn parse_dice_spec(lower: &str) -> Option<(u32, u32)> {
    let s = lower.strip_prefix("roll ").map(|r| r.trim()).unwrap_or(lower);
    let (count_str, sides_str) = s.split_once('d')?;
    if sides_str.is_empty() || !sides_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !count_str.is_empty() && !count_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let count = if count_str.is_empty() {
        1
    } else {
        count_str.parse::<u32>().ok()?
    };
    let sides = sides_str.parse::<u32>().ok()?;
    if (1..=20).contains(&count) && (2..=1000).contains(&sides) {
        // Require either "roll " prefix or standard tabletop die sizes for bare "d<N>" to avoid false positives
        if lower.starts_with("roll ")
            || count > 1
            || matches!(sides, 4 | 6 | 8 | 10 | 12 | 20 | 100)
        {
            return Some((count, sides));
        }
    }
    None
}

/// Evaluates whether a query matches any built-in Quick Developer / Productivity Tool.
pub fn evaluate_quick_tool(raw_query: &str) -> Vec<SearchResult> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let lower = trimmed.to_lowercase();
    let mut results = Vec::new();

    // 1. UUID / GUID Generator
    if matches!(
        lower.as_str(),
        "uuid" | "guid" | "new uuid" | "generate uuid" | "gen uuid" | "uuid4" | "uuid v4"
    ) {
        let id = generate_uuid_v4();
        results.push(make_tool_result(
            id.clone(),
            "UUID v4 Generator".to_string(),
            id,
            "fingerprint",
        ));
        return results;
    }

    // 2. Unix Timestamp / Epoch Converter
    if matches!(
        lower.as_str(),
        "now" | "utc" | "timestamp" | "epoch" | "unix time" | "unix timestamp" | "epoch time"
    ) {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let utc_str = format_unix_utc(now_secs);
        results.push(make_tool_result(
            format!("{} ({})", now_secs, utc_str),
            "Current Unix Timestamp (seconds)".to_string(),
            now_secs.to_string(),
            "schedule",
        ));
        return results;
    }

    for prefix in ["epoch ", "timestamp ", "unix "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let num_str = rest.trim();
            if let Ok(val) = num_str.parse::<i64>() {
                // Detect milliseconds (13 digits) vs seconds (10 digits)
                let secs = if val.abs() >= 1_000_000_000_000 {
                    val / 1000
                } else {
                    val
                };
                let utc_str = format_unix_utc(secs);
                results.push(make_tool_result(
                    utc_str.clone(),
                    format!("Unix Timestamp {} converted to UTC", val),
                    utc_str,
                    "schedule",
                ));
                return results;
            }
        }
    }

    // 3. Base64 Encode / Decode
    for prefix in ["base64 encode ", "b64 encode ", "b64e ", "base64 "] {
        if trimmed.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix))
            && !lower.starts_with("base64 decode ")
        {
            let payload = trimmed[prefix.len()..].trim();
            if !payload.is_empty() {
                let encoded = crate::utilities::to_base64(payload.as_bytes());
                results.push(make_tool_result(
                    encoded.clone(),
                    format!("Base64 Encoded ('{}')", payload),
                    encoded,
                    "code",
                ));
                return results;
            }
        }
    }

    for prefix in ["base64 decode ", "b64 decode ", "b64d "] {
        if trimmed.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix)) {
            let payload = trimmed[prefix.len()..].trim();
            if !payload.is_empty() {
                if let Some(decoded_bytes) = from_base64(payload) {
                    if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                        results.push(make_tool_result(
                            decoded_str.clone(),
                            "Base64 Decoded UTF-8 String".to_string(),
                            decoded_str,
                            "code",
                        ));
                        return results;
                    }
                }
            }
        }
    }

    // 4. URL Encode / Decode
    for prefix in ["url encode ", "urlencode "] {
        if trimmed.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix)) {
            let payload = trimmed[prefix.len()..].trim();
            if !payload.is_empty() {
                let encoded = url_encode(payload);
                results.push(make_tool_result(
                    encoded.clone(),
                    "URL Percent-Encoded".to_string(),
                    encoded,
                    "link",
                ));
                return results;
            }
        }
    }

    for prefix in ["url decode ", "urldecode "] {
        if trimmed.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix)) {
            let payload = trimmed[prefix.len()..].trim();
            if !payload.is_empty() {
                if let Some(decoded) = url_decode(payload) {
                    results.push(make_tool_result(
                        decoded.clone(),
                        "URL Decoded".to_string(),
                        decoded,
                        "link",
                    ));
                    return results;
                }
            }
        }
    }

    // 5. Color Converter (#HEX or rgb(...))
    if let Some(color_res) = try_color_converter(trimmed) {
        results.push(color_res);
        return results;
    }

    // 6. Word & Character Counter
    for prefix in ["count ", "chars ", "words ", "length ", "len "] {
        if trimmed.get(..prefix.len()).is_some_and(|p| p.eq_ignore_ascii_case(prefix)) {
            let text = &trimmed[prefix.len()..];
            if !text.is_empty() {
                let char_count = text.chars().count();
                let word_count = text.split_whitespace().count();
                let byte_count = text.len();
                let summary = format!("{} chars • {} words • {} bytes", char_count, word_count, byte_count);
                results.push(make_tool_result(
                    summary,
                    format!("Text Statistics for \"{}\"", text),
                    char_count.to_string(),
                    "tag",
                ));
                return results;
            }
        }
    }

    // 7. Local IPv4 Address
    if matches!(
        lower.as_str(),
        "ip" | "my ip" | "local ip" | "ip address" | "ipv4" | "what is my ip"
    ) {
        if let Some(ip) = get_local_ipv4() {
            results.push(make_tool_result(
                format!("Local IP: {}", ip),
                "Active Network Interface IPv4 Address".to_string(),
                ip,
                "lan",
            ));
        }
        return results;
    }

    // 8. Password Generator ("password", "pass", "pw", "password 20", "pass 24", "pw 24", "gen pass")
    let pwd_len_opt = if matches!(
        lower.as_str(),
        "password" | "pass" | "pw" | "gen password" | "gen pass" | "generate password"
    ) {
        Some(16usize)
    } else if let Some(rest) = lower
        .strip_prefix("password ")
        .or_else(|| lower.strip_prefix("pass "))
        .or_else(|| lower.strip_prefix("pw "))
    {
        rest.trim().parse::<usize>().ok().map(|n| n.clamp(8, 64))
    } else {
        None
    };

    if let Some(len) = pwd_len_opt {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+";
        let mut pwd = String::with_capacity(len);
        for _ in 0..len {
            let idx = (next_random_u64() as usize) % CHARSET.len();
            pwd.push(CHARSET[idx] as char);
        }
        results.push(make_tool_result(
            pwd.clone(),
            format!("Generated {}-character Secure Password", len),
            pwd,
            "key",
        ));
        return results;
    }

    // 9. Coin Flip & Dice Roll
    if matches!(lower.as_str(), "flip coin" | "coin flip" | "toss coin") {
        let side = if next_random_u64().is_multiple_of(2) { "Heads" } else { "Tails" };
        results.push(make_tool_result(
            format!("Coin Flip: {}", side),
            "50/50 Random Coin Toss".to_string(),
            side.to_string(),
            "casino",
        ));
        return results;
    }

    if matches!(lower.as_str(), "roll dice" | "dice" | "roll a die") {
        let roll = (next_random_u64() % 6) + 1;
        results.push(make_tool_result(
            format!("Rolled a {}", roll),
            "6-Sided Die Roll (1–6)".to_string(),
            roll.to_string(),
            "casino",
        ));
        return results;
    }

    if let Some((count, sides)) = parse_dice_spec(&lower) {
        let mut rolls = Vec::with_capacity(count as usize);
        let mut total: u64 = 0;
        for _ in 0..count {
            let r = (next_random_u64() % (sides as u64)) + 1;
            total += r;
            rolls.push(r.to_string());
        }
        let (title, subtitle) = if count == 1 {
            (
                format!("Rolled d{}: {}", sides, total),
                format!("{}-Sided Die Roll (1–{})", sides, sides),
            )
        } else {
            (
                format!("Rolled {}d{}: {} ({})", count, sides, total, rolls.join(" + ")),
                format!("{} × {}-Sided Dice Roll", count, sides),
            )
        };
        results.push(make_tool_result(
            title,
            subtitle,
            total.to_string(),
            "casino",
        ));
        return results;
    }

    results
}

fn try_color_converter(trimmed: &str) -> Option<SearchResult> {
    let lower = trimmed.to_lowercase();
    let candidate = lower.strip_prefix("color ").unwrap_or(&lower).trim();

    // Hex format: #RGB or #RRGGBB
    if let Some(hex) = candidate.strip_prefix('#') {
        let (r, g, b) = if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b)
        } else if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        } else {
            return None;
        };

        let (h, s, l) = rgb_to_hsl(r, g, b);
        let rgb_str = format!("rgb({}, {}, {})", r, g, b);
        let hsl_str = format!("hsl({}, {}%, {}%)", h, s, l);
        let hex_upper = format!("#{:02X}{:02X}{:02X}", r, g, b);

        return Some(make_tool_result(
            format!("{}  •  {}  •  {}", hex_upper, rgb_str, hsl_str),
            "Color Converter (Copies RGB)".to_string(),
            rgb_str,
            "palette",
        ));
    }

    // RGB format: rgb(255, 87, 51)
    if let Some(inner) = candidate.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let hex_upper = format!("#{:02X}{:02X}{:02X}", r, g, b);
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let hsl_str = format!("hsl({}, {}%, {}%)", h, s, l);
            return Some(make_tool_result(
                format!("{}  •  rgb({}, {}, {})  •  {}", hex_upper, r, g, b, hsl_str),
                "Color Converter (Copies HEX)".to_string(),
                hex_upper,
                "palette",
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v4_format() {
        let id1 = generate_uuid_v4();
        let id2 = generate_uuid_v4();
        assert_eq!(id1.len(), 36);
        assert_ne!(id1, id2);
        assert_eq!(&id1[14..15], "4");

        let res = evaluate_quick_tool("uuid");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].metadata.extension, "fingerprint");
    }

    #[test]
    fn test_epoch_conversion() {
        assert_eq!(format_unix_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_unix_utc(1700000000), "2023-11-14 22:13:20 UTC");

        let res = evaluate_quick_tool("epoch 1700000000");
        assert_eq!(res.len(), 1);
        assert!(res[0].metadata.name.contains("2023-11-14"));

        let now_res = evaluate_quick_tool("now");
        assert_eq!(now_res.len(), 1);
        assert!(now_res[0].metadata.name.contains("UTC"));
        assert!(is_dynamic_query("now"));
    }

    #[test]
    fn test_password_and_dice_triggers() {
        let pw1 = evaluate_quick_tool("pw");
        assert_eq!(pw1.len(), 1);
        assert_eq!(pw1[0].metadata.name.len(), 16);
        assert_eq!(pw1[0].metadata.extension, "key");
        assert!(is_dynamic_query("pw"));

        let pw2 = evaluate_quick_tool("pass 24");
        assert_eq!(pw2.len(), 1);
        assert_eq!(pw2[0].metadata.name.len(), 24);
        assert!(is_dynamic_query("pass 24"));

        let d20 = evaluate_quick_tool("roll d20");
        assert_eq!(d20.len(), 1);
        assert!(d20[0].metadata.name.starts_with("Rolled d20:"));
        assert_eq!(d20[0].metadata.extension, "casino");
        assert!(is_dynamic_query("roll d20"));

        let d6 = evaluate_quick_tool("roll 2d6");
        assert_eq!(d6.len(), 1);
        assert!(d6[0].metadata.name.starts_with("Rolled 2d6:"));
    }

    #[test]
    fn test_base64_encode_decode() {
        let enc = evaluate_quick_tool("base64 encode hello world");
        assert_eq!(enc.len(), 1);
        assert_eq!(enc[0].metadata.name, "aGVsbG8gd29ybGQ=");

        let dec = evaluate_quick_tool("base64 decode aGVsbG8gd29ybGQ=");
        assert_eq!(dec.len(), 1);
        assert_eq!(dec[0].metadata.name, "hello world");
    }

    #[test]
    fn test_color_converter() {
        let res = evaluate_quick_tool("#ff5733");
        assert_eq!(res.len(), 1);
        assert!(res[0].metadata.name.contains("rgb(255, 87, 51)"));

        let res2 = evaluate_quick_tool("rgb(255, 87, 51)");
        assert_eq!(res2.len(), 1);
        assert!(res2[0].metadata.name.contains("#FF5733"));
    }

    #[test]
    fn test_char_word_count() {
        let res = evaluate_quick_tool("count hello rust world");
        assert_eq!(res.len(), 1);
        assert!(res[0].metadata.name.contains("16 chars"));
        assert!(res[0].metadata.name.contains("3 words"));
    }
}
