/// Escape a string for use in a systemd unit name.
///
/// See [String Escaping for Inclusion in Unit Names][1] for more information.
///
/// [1]: https://www.freedesktop.org/software/systemd/man/systemd.unit.html#String%20Escaping%20for%20Inclusion%20in%20Unit%20Names
pub fn escape_name(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() * 2);
    for (index, b) in s.bytes().enumerate() {
        match b {
            b'/' => escaped.push('-'),
            // Do not escape '.' unless it's the first character
            b'.' if 0 < index => escaped.push(char::from(b)),
            // Do not escape _ and : and
            b'_' | b':' => escaped.push(char::from(b)),
            // all ASCII alphanumeric characters
            _ if b.is_ascii_alphanumeric() => escaped.push(char::from(b)),
            _ => escaped.push_str(&format!("\\x{:02x}", b)),
        }
    }
    escaped
}
