use mrglglgl_core::narrators::{
    self, DEFAULT_NARRATOR, Invalid, Library, MAX_FIELD_CHARS, NarratorError, Problem,
};

fn dwarf_copy() -> narrators::Narrator {
    narrators::copy_of("grumpy_dwarf", "en")
}

#[test]
fn a_copy_of_a_built_in_has_every_field_and_no_id_yet() {
    let copy = dwarf_copy();
    let original = narrators::get("grumpy_dwarf", "en");
    assert_eq!(copy.name, "The grumpy dwarf (mine)");
    assert!(copy.id.is_empty(), "an id is given when it is saved");
    assert_eq!(copy.persona, original.persona);
    assert_eq!(copy.example, original.example);
    assert_eq!(copy.hints, original.hints);
    assert!(!copy.opt_in);
}

#[test]
fn a_saved_narrator_is_listed_and_used_in_any_prose_language() {
    let dir = tempfile::tempdir().unwrap();
    let mut copy = dwarf_copy();
    copy.voice = "Kind and slow, like moss.".into();
    let saved = narrators::save(dir.path(), copy).unwrap();
    assert_eq!(saved.id, "custom_the_grumpy_dwarf_mine");
    assert!(
        dir.path()
            .join("custom_the_grumpy_dwarf_mine.toml")
            .is_file()
    );

    let library = Library::load(dir.path());
    let listed = library.list();
    let mine = listed.iter().find(|n| n.id == saved.id).unwrap();
    assert!(mine.custom);
    assert!(
        listed.iter().filter(|n| !n.custom).count() >= 5,
        "built-ins stay"
    );
    assert_eq!(
        library.get(&saved.id, "fr").voice,
        "Kind and slow, like moss."
    );

    // Saving it again keeps its id and its file.
    let mut again = saved.clone();
    again.voice = "Kinder still.".into();
    assert_eq!(narrators::save(dir.path(), again).unwrap().id, saved.id);
    assert_eq!(
        Library::load(dir.path()).get(&saved.id, "en").voice,
        "Kinder still."
    );
}

#[test]
fn built_ins_cannot_be_overwritten_and_names_that_clash_get_their_own_id() {
    let dir = tempfile::tempdir().unwrap();
    let first = narrators::save(dir.path(), dwarf_copy()).unwrap();
    let second = narrators::save(dir.path(), dwarf_copy()).unwrap();
    assert_eq!(second.id, format!("{}_2", first.id));

    let mut built_in = narrators::get("grumpy_dwarf", "en").clone();
    built_in.voice = "Changed.".into();
    let saved = narrators::save(dir.path(), built_in).unwrap();
    assert!(
        saved.id.starts_with("custom_"),
        "a built-in id becomes a copy"
    );
    assert_ne!(
        Library::load(dir.path()).get("grumpy_dwarf", "en").voice,
        "Changed."
    );
}

#[test]
fn an_invalid_profile_is_refused_naming_each_field() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = dwarf_copy();
    bad.persona = "  ".into();
    bad.voice = "x".repeat(MAX_FIELD_CHARS + 1);
    bad.example.margin_note = "Mrgl, no star.".into();
    bad.hints.insert("flying".into(), "Wings.".into());
    let problems = narrators::validate(&bad);
    let has = |field: &str, problem: Problem| {
        problems.contains(&Invalid {
            field: field.into(),
            problem,
        })
    };
    assert!(has("persona", Problem::Missing));
    assert!(has("voice", Problem::TooLong));
    assert!(has("example.margin_note", Problem::NoStar));
    assert!(has("hints.flying", Problem::UnknownHint));
    match narrators::save(dir.path(), bad) {
        Err(NarratorError::Invalid(list)) => assert_eq!(list, problems),
        other => panic!("expected invalid, got {other:?}"),
    }
    assert!(Library::load(dir.path()).list().iter().all(|n| !n.custom));
    assert!(narrators::validate(&dwarf_copy()).is_empty());
}

#[test]
fn import_and_export_share_a_narrator_as_one_file() {
    let dir = tempfile::tempdir().unwrap();
    let saved = narrators::save(dir.path(), dwarf_copy()).unwrap();
    let text = narrators::to_toml(&saved);
    assert_eq!(
        narrators::parse(&text).unwrap(),
        saved,
        "the file reads back the same"
    );

    // Importing a file whose id is taken gives it a fresh one; the name stays.
    let imported = narrators::import(dir.path(), &text).unwrap();
    assert_ne!(imported.id, saved.id);
    assert!(imported.id.starts_with("custom_"));
    assert_eq!(imported.name, saved.name);

    // A built-in's own file imports as a copy.
    let built_in = narrators::to_toml(narrators::get("pompous_noble", "en"));
    assert!(
        narrators::import(dir.path(), &built_in)
            .unwrap()
            .id
            .starts_with("custom_")
    );

    assert!(matches!(
        narrators::import(dir.path(), "not = [a narrator"),
        Err(NarratorError::Unreadable(_))
    ));
}

#[test]
fn a_deleted_narrator_falls_back_to_the_scribe() {
    let dir = tempfile::tempdir().unwrap();
    let saved = narrators::save(dir.path(), dwarf_copy()).unwrap();
    narrators::delete(dir.path(), &saved.id).unwrap();
    let library = Library::load(dir.path());
    assert!(library.list().iter().all(|n| n.id != saved.id));
    assert_eq!(library.get(&saved.id, "en").id, DEFAULT_NARRATOR);
    assert!(
        matches!(
            narrators::delete(dir.path(), "grumpy_dwarf"),
            Err(NarratorError::BuiltIn)
        ),
        "a built-in is never deleted"
    );
}

#[test]
fn a_missing_folder_or_a_broken_file_leaves_the_built_ins() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        Library::load(&dir.path().join("nowhere")).list(),
        narrators::list()
    );
    std::fs::write(dir.path().join("custom_broken.toml"), "garbage = [").unwrap();
    assert_eq!(Library::load(dir.path()).list(), narrators::list());
}

#[test]
fn a_new_narrator_starts_blank_with_the_shared_parts_filled_in() {
    let blank = narrators::blank("en");
    let scribe = narrators::get(DEFAULT_NARRATOR, "en");
    assert!(blank.id.is_empty() && blank.name.is_empty());
    assert!(blank.persona.is_empty() && blank.voice.is_empty());
    assert!(blank.example.entry.is_empty() && blank.example.margin_note.is_empty());
    assert_eq!(
        blank.example.notes, scribe.example.notes,
        "the made-up day to write from"
    );
    assert_eq!(blank.example.intro, scribe.example.intro);
    assert_eq!(blank.murloc, scribe.murloc);
    assert!(blank.hints.is_empty());

    // It cannot be saved until the player has said who the narrator is.
    let dir = tempfile::tempdir().unwrap();
    let Err(NarratorError::Invalid(problems)) = narrators::save(dir.path(), blank.clone()) else {
        panic!("a blank narrator is refused");
    };
    let fields: Vec<&str> = problems.iter().map(|p| p.field.as_str()).collect();
    assert_eq!(
        fields,
        [
            "name",
            "persona",
            "voice",
            "example.entry",
            "example.margin_note"
        ]
    );

    let mut mine = blank;
    mine.name = "The tired innkeeper".into();
    mine.persona = "You are an innkeeper who has heard every story twice.".into();
    mine.voice = "Weary, warm, short sentences.".into();
    mine.example.entry = "Another wolf story. I poured another ale.".into();
    mine.example.margin_note = "* The murloc tipped the ale over.".into();
    let saved = narrators::save(dir.path(), mine).unwrap();
    assert_eq!(saved.id, "custom_the_tired_innkeeper");
}

#[test]
fn every_built_in_narrator_speaks_french_with_a_french_example() {
    for info in narrators::all().iter().filter(|n| n.language == "en") {
        let fr = narrators::get(&info.id, "fr");
        assert_eq!(fr.language, "fr", "{} has no French profile", info.id);
        assert_ne!(fr.name, info.name, "{} keeps its English name", info.id);
        assert!(fr.example.entry.contains(" je ") || fr.example.entry.contains("j'"), "{}", info.id);
        // The instructions are the same in both languages, apart from the translated label.
        assert_eq!(fr.hints, info.hints, "{}", info.id);
    }
}
