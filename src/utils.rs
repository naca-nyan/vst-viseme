const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn note_friendly_name(note: &u8) -> String {
    let octave = (note / 12) as i8 - 2;
    let note_num = note % 12;
    let note = NOTES[note_num as usize];
    format!("{note}{octave}")
}

pub fn decode_unicode_escapes(s: &str) -> String {
    let escape_sequence = r"\u";
    if !s.contains(escape_sequence) {
        return s.to_owned();
    }
    let mut result = String::new();
    let mut rest = s;
    while let Some(pos) = rest.find(escape_sequence) {
        result.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let mut utf16 = Vec::new();
        let mut raw_parts = Vec::new();
        while rest.starts_with(escape_sequence) {
            let part = &rest[..6.min(rest.len())];
            if part.len() < 6 {
                break;
            }
            match u16::from_str_radix(&part[2..6], 16) {
                Ok(code) => {
                    utf16.push(code);
                    raw_parts.push(part);
                    rest = &rest[6..];
                }
                Err(_) => break,
            }
        }
        if utf16.is_empty() {
            result.push_str(&rest[..2]);
            rest = &rest[2..];
            continue;
        }
        let mut i = 0;
        for r in char::decode_utf16(utf16) {
            match r {
                Ok(ch) => {
                    result.push(ch);
                    i += if ch.len_utf16() == 2 { 2 } else { 1 };
                }
                Err(_) => {
                    result.push_str(raw_parts[i]);
                    i += 1;
                }
            }
        }
    }
    result.push_str(rest);
    result
}

pub fn encode_unicode_escapes(s: &str) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii() {
            result.push(c);
        } else {
            for utf16 in c.encode_utf16(&mut [0; 2]) {
                write!(result, "\\u{:04x}", utf16).unwrap();
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_friendly_name() {
        assert_eq!(note_friendly_name(&0), "C-2");
        assert_eq!(note_friendly_name(&12), "C-1");
        assert_eq!(note_friendly_name(&60), "C3");
        assert_eq!(note_friendly_name(&69), "A3");
        assert_eq!(note_friendly_name(&127), "G8");
    }

    const CASES: &[(&str, &str)] = &[
        (r"\u30c6\u30b9\u30c8", "テスト"),
        ("hello", "hello"),
        (
            r"This is \u30c6\u30b9\u30c8 for \u6f22\u5b57",
            "This is テスト for 漢字",
        ),
        (r"\u2728\ud83e\udd79", "✨🥹"),
        (r"\u2728\ud83e", r"✨\ud83e"),
        ("", ""),
    ];
    #[test]
    fn test_decode_unicode_escapes() {
        for &(encoded, decoded) in CASES {
            assert_eq!(decode_unicode_escapes(encoded), decoded,);
        }
    }
    #[test]
    fn test_encode_unicode_escapes() {
        for &(encoded, decoded) in CASES {
            assert_eq!(encode_unicode_escapes(decoded), encoded,);
        }
    }
}
