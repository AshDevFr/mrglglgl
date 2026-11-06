use std::path::PathBuf;

use mrglglgl_core::sv::{self, Key, MAX_DEPTH, ParseErrorKind, Value};

fn data(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn table<'a>(value: &'a Value, path: &[&str]) -> &'a sv::Table {
    let mut current = value.as_table().expect("a table");
    for key in path {
        current = current
            .get(key)
            .and_then(Value::as_table)
            .unwrap_or_else(|| panic!("no table at {key}"));
    }
    current
}

#[test]
fn every_test_file_parses() {
    for name in [
        "journal.lua",
        "journal-indented.lua",
        "journal-schema0.lua",
        "journal-unknown-field.lua",
        "journal-newer.lua",
        "settings.lua",
        "escapes.lua",
    ] {
        sv::parse(&data(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

/// The fixtures the addon generates from its own code, shared with its specs: whatever the addon
/// writes, the parser reads.
#[test]
fn every_shared_fixture_parses() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/savedvariables");
    let mut parsed = 0;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "lua") {
            let text = std::fs::read_to_string(&path).unwrap();
            sv::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            parsed += 1;
        }
    }
    assert!(
        parsed >= 5,
        "expected the shared fixtures in {}",
        dir.display()
    );
}

#[test]
fn reads_the_shape_the_live_client_writes() {
    let file = sv::parse(&data("journal.lua")).unwrap();
    let db = file.get("MrglglglCharDB").expect("the journal variable");
    let char = table(db, &["char"]);
    assert_eq!(char.get("name").and_then(Value::as_str), Some("Wren"));
    assert_eq!(
        char.get("surname").and_then(Value::as_str),
        Some("Hollowbrook")
    );

    let day = table(db, &["days", "2026-11-05"]);
    let events: Vec<_> = day
        .get("ev")
        .and_then(Value::as_table)
        .unwrap()
        .sequence()
        .collect();
    assert_eq!(events.len(), 8);
    let first = events[0].as_table().unwrap();
    assert_eq!(
        first.get("k").and_then(Value::as_str),
        Some("session_start")
    );
    assert_eq!(first.get("t").and_then(Value::as_i64), Some(1_793_941_742));

    let kills = table(db, &["days", "2026-11-05", "kills"]);
    let wolves = kills.get_index(2958).and_then(Value::as_table).unwrap();
    assert_eq!(wolves.get("n").and_then(Value::as_i64), Some(23));
    assert_eq!(wolves.get("murloc").and_then(Value::as_bool), Some(false));

    let empty_day = table(db, &["days", "2026-11-04", "ev"]);
    assert!(empty_day.is_empty());
}

#[test]
fn reads_the_indented_shape_with_index_comments() {
    let file = sv::parse(&data("journal-indented.lua")).unwrap();
    let db = file.get("MrglglglCharDB").unwrap();
    let events = table(db, &["days", "2026-11-05", "ev"]);
    assert_eq!(events.sequence().count(), 2);
    let second = events.get_index(2).and_then(Value::as_table).unwrap();
    assert_eq!(second.get("t"), Some(&Value::Float(1_793_941_750.5)));
}

#[test]
fn reads_the_account_settings_file() {
    let file = sv::parse(&data("settings.lua")).unwrap();
    let record = table(file.get("MrglglglDB").unwrap(), &["settings", "record"]);
    assert_eq!(record.get("kills").and_then(Value::as_bool), Some(false));
}

#[test]
fn decodes_strings_escaped_by_lua_itself() {
    let file = sv::parse(&data("escapes.lua")).unwrap();
    let escapes = file.get("Escapes").and_then(Value::as_table).unwrap();
    assert_eq!(
        escapes.get("q").and_then(Value::as_str),
        Some("He said \"mrgl\"\\ then\nleft\tnow \0 and \u{1} \u{7f} done")
    );
    assert_eq!(
        escapes.get("plain").and_then(Value::as_str),
        Some("line one\nline two")
    );
}

#[test]
fn reads_numbers_booleans_nil_and_shorthand_keys() {
    let file =
        sv::parse("X = { a = -5, b = 0x1F, c = 1e3, d = -2.5, e = nil, f = true, [3.0] = 'x' }")
            .unwrap();
    let x = file.get("X").and_then(Value::as_table).unwrap();
    assert_eq!(x.get("a"), Some(&Value::Integer(-5)));
    assert_eq!(x.get("b"), Some(&Value::Integer(31)));
    assert_eq!(x.get("c").and_then(Value::as_f64), Some(1000.0));
    assert_eq!(x.get("d"), Some(&Value::Float(-2.5)));
    assert_eq!(x.get("e"), None, "a nil value removes the key");
    assert_eq!(x.get("f"), Some(&Value::Bool(true)));
    assert_eq!(x.get_index(3), Some(&Value::String("x".into())));
}

#[test]
fn keeps_positional_and_explicit_integer_keys_apart_like_lua() {
    let file = sv::parse("X = { 'a', 'b', [5] = 'e' }").unwrap();
    let x = file.get("X").and_then(Value::as_table).unwrap();
    assert_eq!(x.sequence().count(), 2, "ipairs stops at the gap");
    assert_eq!(x.len(), 3);
    assert!(x.iter().any(|(k, _)| *k == Key::Integer(5)));
}

#[test]
fn rejects_tables_nested_too_deep_without_overflowing_the_stack() {
    let deep = format!("X = {}{}", "{".repeat(100_000), "}".repeat(100_000));
    let err = sv::parse(&deep).unwrap_err();
    assert_eq!(err.kind, ParseErrorKind::TooDeep);

    let fine = format!("X = {}{}", "{".repeat(MAX_DEPTH), "}".repeat(MAX_DEPTH));
    assert!(sv::parse(&fine).is_ok());
}

#[test]
fn reports_where_a_broken_file_breaks() {
    let err = sv::parse("\nX = {\n[\"a\"] = \"unclosed,\n}\n").unwrap_err();
    assert_eq!(err.kind, ParseErrorKind::UnterminatedString);
    assert_eq!(err.line, 3);
    assert!(err.to_string().starts_with("line 3,"));
}

#[test]
fn never_runs_anything_it_reads() {
    // Code is not data: a function call where a value belongs is a parse error, not a call.
    assert!(sv::parse("X = os.execute('rm -rf /')").is_err());
    assert!(sv::parse("X = { [\"a\"] = print(1) }").is_err());
}

/// Stands in for libFuzzer on stable Rust (the `fuzz/` target needs nightly): thousands of
/// deterministic mutations of real files must each parse or fail cleanly, never panic or hang.
#[test]
fn survives_mutated_input() {
    let seeds = [
        data("journal.lua"),
        data("escapes.lua"),
        data("journal-indented.lua"),
    ];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let alphabet = b"{}[]=,;-\"'\\\n0123456789.xeE_abc\x00\xff";
    for round in 0..4000 {
        let mut bytes = seeds[round % seeds.len()].clone().into_bytes();
        for _ in 0..1 + next() % 8 {
            if bytes.is_empty() {
                break;
            }
            let at = (next() as usize) % bytes.len();
            match next() % 4 {
                0 => bytes[at] = alphabet[(next() as usize) % alphabet.len()],
                1 => {
                    let end = (at + (next() as usize) % 32).min(bytes.len());
                    bytes.drain(at..end);
                }
                2 => bytes.insert(at, alphabet[(next() as usize) % alphabet.len()]),
                _ => bytes.truncate(at),
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        let _ = sv::parse(&text);
    }
}
