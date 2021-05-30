extern crate systemd;

#[test]
fn escape_name() {
    let samples = vec![
        // (input, escaped)
        ("test", "test"),
        ("a:b_c.d", "a:b_c.d"),
        ("/foo/", "-foo-"),
        (".foo", "\\x2efoo"),
        ("Hallöchen, Meister", "Hall\\xc3\\xb6chen\\x2c\\x20Meister"),
    ];

    for (input, expected) in samples {
        assert_eq!(systemd::unit::escape_name(input), expected);
    }
}