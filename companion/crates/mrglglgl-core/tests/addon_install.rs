use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use mrglglgl_core::addon_install::{
    InstallError, Ownership, Takeover, compare_versions, install, latest_release, managed_target,
    ownership, remove, rollback_package, rollbacks, verify,
};
use mrglglgl_core::discovery::Flavor;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

const TAG: &str = "addon-v0.2.0";
const NOW: i64 = 1_794_000_000;

fn toc(version: &str, schema: Option<i64>) -> String {
    let schema = schema
        .map(|s| format!("## X-Mrglglgl-Schema: {s}\n"))
        .unwrap_or_default();
    format!(
        "## Interface: 16001\n## Title: Mrglglgl\n## Version: {version}\n{schema}\
         ## SavedVariables: MrglglglDB\n\nCore.lua\n"
    )
}

/// A zip with the given entries; a name ending in `/` is a folder.
fn zip_of(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (name, content) in entries {
        if name.ends_with('/') {
            writer.add_directory(*name, options).unwrap();
        } else {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}

fn package(version: &str) -> Vec<u8> {
    zip_of(&[
        ("Mrglglgl/", ""),
        ("Mrglglgl/Mrglglgl.toc", &toc(version, Some(1))),
        ("Mrglglgl/Core.lua", &format!("-- core {version}\n")),
        ("Mrglglgl/data/", ""),
        ("Mrglglgl/data/Junk.lua", "-- junk\n"),
    ])
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn touch(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A game folder with a Forever flavor: its saved variables, screenshots and another addon.
struct Game {
    _dir: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    flavor: Flavor,
}

fn game() -> Game {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("World of Warcraft");
    let path = root.join("_forever_");
    touch(
        &path.join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    );
    touch(
        &path.join("WTF/Account/ACCOUNT1/93/Wren-Hollowbrook/SavedVariables/Mrglglgl.lua"),
        "MrglglglCharDB = {}\n",
    );
    touch(
        &path.join("WTF/Account/ACCOUNT1/SavedVariables/Mrglglgl.lua"),
        "MrglglglDB = {}\n",
    );
    touch(
        &path.join("Screenshots/WoWScrnShot_110826_120000.jpg"),
        "jpg",
    );
    touch(
        &path.join("Interface/AddOns/Altoholic/Altoholic.toc"),
        "## Interface: 16001\n",
    );
    Game {
        data: dir.path().join("data"),
        root,
        flavor: Flavor {
            name: "_forever_".into(),
            path,
        },
        _dir: dir,
    }
}

/// Every file under a folder with its content, links shown as links.
fn tree(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let kind = entry.file_type().unwrap();
            let name = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if kind.is_symlink() {
                out.insert(name, "<link>".into());
            } else if kind.is_dir() {
                out.insert(format!("{name}/"), String::new());
                stack.push(entry.path());
            } else {
                out.insert(
                    name,
                    String::from_utf8_lossy(&fs::read(entry.path()).unwrap()).into(),
                );
            }
        }
    }
    out
}

/// Only the addon's own folder (and nothing beside it) may differ between the two trees.
fn only_the_addon_changed(before: &BTreeMap<String, String>, after: &BTreeMap<String, String>) {
    let outside = |t: &BTreeMap<String, String>| -> BTreeMap<String, String> {
        t.iter()
            .filter(|(k, _)| !k.starts_with("_forever_/Interface/AddOns/Mrglglgl"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };
    assert_eq!(outside(before), outside(after));
    assert!(
        !after
            .keys()
            .any(|k| k.starts_with("_forever_/Interface/AddOns/Mrglglgl-staging")),
        "no working folder is left behind"
    );
}

#[test]
fn installs_a_checked_package_and_changes_nothing_else_in_the_game_folder() {
    let g = game();
    let before = tree(&g.root);
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    let done = install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW).unwrap();
    assert_eq!(done.version.as_deref(), Some("0.2.0"));
    assert!(!done.replaced);
    let after = tree(&g.root);
    only_the_addon_changed(&before, &after);
    let addon = g.flavor.path.join("Interface/AddOns/Mrglglgl");
    assert_eq!(
        fs::read_to_string(addon.join("Core.lua")).unwrap(),
        "-- core addon-v0.2.0\n"
    );
    assert!(addon.join("data/Junk.lua").is_file());
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::Managed);
}

#[test]
fn creates_the_addons_folder_only_when_the_interface_folder_exists() {
    let g = game();
    fs::remove_dir_all(g.flavor.path.join("Interface")).unwrap();
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    assert_eq!(
        install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW),
        Err(InstallError::NoInterface("_forever_".into()))
    );
    assert!(!g.flavor.path.join("Interface").exists());

    fs::create_dir(g.flavor.path.join("Interface")).unwrap();
    install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW).unwrap();
    assert!(
        g.flavor
            .path
            .join("Interface/AddOns/Mrglglgl/Mrglglgl.toc")
            .is_file()
    );
}

#[test]
fn an_update_replaces_the_folder_and_keeps_three_copies_in_the_data_folder() {
    let g = game();
    for (i, minor) in [1, 2, 3, 4, 5].iter().enumerate() {
        let tag = format!("addon-v0.{minor}.0");
        let zip = package(&tag);
        let checked = verify(&zip, &sha(&zip), &tag).unwrap();
        let done = install(
            &g.flavor,
            &checked,
            &g.data,
            false,
            Takeover::Refuse,
            NOW + i as i64,
        )
        .unwrap();
        assert_eq!(done.replaced, i > 0);
    }
    let addon = g.flavor.path.join("Interface/AddOns/Mrglglgl");
    assert_eq!(
        fs::read_to_string(addon.join("Core.lua")).unwrap(),
        "-- core addon-v0.5.0\n"
    );
    let kept = rollbacks(&g.data, &g.flavor);
    assert_eq!(
        kept.iter()
            .map(|r| r.version.clone().unwrap())
            .collect::<Vec<_>>(),
        vec!["0.4.0", "0.3.0", "0.2.0"]
    );
    assert!(kept.iter().all(|r| r.path.starts_with(&g.data)));

    // Rolling back is installing a kept copy again.
    let back = rollback_package(&kept[0].path).unwrap();
    install(&g.flavor, &back, &g.data, false, Takeover::Refuse, NOW + 10).unwrap();
    assert_eq!(
        fs::read_to_string(addon.join("Core.lua")).unwrap(),
        "-- core addon-v0.4.0\n"
    );
}

#[test]
fn refuses_while_the_game_runs_for_any_change() {
    let g = game();
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    let before = tree(&g.root);
    assert_eq!(
        install(&g.flavor, &checked, &g.data, true, Takeover::Allow, NOW),
        Err(InstallError::GameRunning)
    );
    install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW).unwrap();
    let installed = tree(&g.root);
    assert_eq!(
        remove(&g.flavor, &g.data, true, Takeover::Allow, NOW),
        Err(InstallError::GameRunning)
    );
    assert_eq!(tree(&g.root), installed);
    assert_ne!(before, installed);
}

#[test]
fn never_writes_through_a_linked_development_copy() {
    let g = game();
    let source = g.root.parent().unwrap().join("checkout");
    touch(
        &source.join("Mrglglgl.toc"),
        "## Version: @project-version@\n",
    );
    let link = g.flavor.path.join("Interface/AddOns/Mrglglgl");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&source, &link).unwrap();
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::Development);
    let before = tree(&source);
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    assert_eq!(
        install(&g.flavor, &checked, &g.data, false, Takeover::Allow, NOW),
        Err(InstallError::DevelopmentCopy)
    );
    assert_eq!(
        remove(&g.flavor, &g.data, false, Takeover::Allow, NOW),
        Err(InstallError::DevelopmentCopy)
    );
    assert_eq!(tree(&source), before);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn only_a_forever_flavor_is_ever_a_target() {
    let g = game();
    touch(
        &g.flavor.path.join(".flavor.info"),
        "Product Flavor!STRING:0\nwow_classic_era\n",
    );
    assert_eq!(
        managed_target(&g.flavor),
        Err(InstallError::NotForever("_forever_".into()))
    );
}

#[test]
fn a_copy_installed_by_something_else_is_left_alone_unless_taken_over() {
    let g = game();
    touch(
        &g.flavor.path.join("Interface/AddOns/Mrglglgl/Mrglglgl.toc"),
        &toc("addon-v0.1.0", None),
    );
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::Foreign);
    let before = tree(&g.root);
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    assert_eq!(
        install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW),
        Err(InstallError::NotManaged)
    );
    assert_eq!(
        remove(&g.flavor, &g.data, false, Takeover::Refuse, NOW),
        Err(InstallError::NotManaged)
    );
    assert_eq!(tree(&g.root), before);

    install(&g.flavor, &checked, &g.data, false, Takeover::Allow, NOW).unwrap();
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::Managed);
    // A file changed by hand afterwards is noticed.
    touch(
        &g.flavor.path.join("Interface/AddOns/Mrglglgl/Core.lua"),
        "-- edited\n",
    );
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::Changed);
}

#[test]
fn removing_takes_only_the_addon_folder_and_keeps_a_copy() {
    let g = game();
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    let before = tree(&g.root);
    install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW).unwrap();
    remove(&g.flavor, &g.data, false, Takeover::Refuse, NOW + 1).unwrap();
    let after = tree(&g.root);
    let mut expected = before.clone();
    expected.insert("_forever_/Interface/AddOns/".into(), String::new());
    assert_eq!(
        after, expected,
        "the game folder is as it was, WTF included"
    );
    assert_eq!(ownership(&g.data, &g.flavor), Ownership::NotInstalled);
    assert_eq!(rollbacks(&g.data, &g.flavor).len(), 1);
    assert_eq!(
        remove(&g.flavor, &g.data, false, Takeover::Refuse, NOW + 2),
        Err(InstallError::NotInstalled)
    );
}

#[test]
fn working_folders_left_by_a_crash_are_cleared_on_the_next_install() {
    let g = game();
    let addons = g.flavor.path.join("Interface/AddOns");
    touch(
        &addons.join("Mrglglgl-staging-dead/Mrglglgl.toc"),
        "## half\n",
    );
    touch(
        &addons.join("Mrglglgl-staging-dead-old/Core.lua"),
        "-- old\n",
    );
    let zip = package(TAG);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    install(&g.flavor, &checked, &g.data, false, Takeover::Refuse, NOW).unwrap();
    assert!(!addons.join("Mrglglgl-staging-dead").exists());
    assert!(!addons.join("Mrglglgl-staging-dead-old").exists());
    assert!(addons.join("Altoholic/Altoholic.toc").is_file());
}

/// Whether a refusal is the one a case expects.
type Expect = Box<dyn Fn(&InstallError) -> bool>;

#[test]
fn every_bad_package_is_refused_before_anything_is_written() {
    let good_toc = toc(TAG, Some(1));
    let cases: Vec<(&str, Vec<u8>, Expect)> = vec![
        (
            "a path out of the folder",
            zip_of(&[
                ("Mrglglgl/Mrglglgl.toc", &good_toc),
                ("Mrglglgl/../../WTF/x.lua", "x"),
            ]),
            Box::new(|e| matches!(e, InstallError::BadPath(_))),
        ),
        (
            "an absolute path",
            zip_of(&[
                ("Mrglglgl/Mrglglgl.toc", &good_toc),
                ("/etc/passwd.lua", "x"),
            ]),
            Box::new(|e| matches!(e, InstallError::BadPath(_))),
        ),
        (
            "a second folder",
            zip_of(&[
                ("Mrglglgl/Mrglglgl.toc", &good_toc),
                ("Other/Other.toc", "x"),
            ]),
            Box::new(|e| matches!(e, InstallError::BadPath(_))),
        ),
        (
            "a program",
            zip_of(&[
                ("Mrglglgl/Mrglglgl.toc", &good_toc),
                ("Mrglglgl/run.exe", "x"),
            ]),
            Box::new(|e| matches!(e, InstallError::BadFile(_))),
        ),
        (
            "no TOC",
            zip_of(&[("Mrglglgl/Core.lua", "x")]),
            Box::new(|e| *e == InstallError::NoToc),
        ),
        (
            "another game's interface",
            zip_of(&[("Mrglglgl/Mrglglgl.toc", &good_toc.replace("16001", "11507"))]),
            Box::new(|e| matches!(e, InstallError::WrongInterface(_))),
        ),
        (
            "another release's version",
            zip_of(&[("Mrglglgl/Mrglglgl.toc", &toc("addon-v0.1.0", Some(1)))]),
            Box::new(|e| matches!(e, InstallError::WrongVersion { .. })),
        ),
        (
            "a newer journal schema",
            zip_of(&[("Mrglglgl/Mrglglgl.toc", &toc(TAG, Some(99)))]),
            Box::new(|e| *e == InstallError::NewerSchema(99)),
        ),
        (
            "not a zip",
            b"not a zip".to_vec(),
            Box::new(|e| matches!(e, InstallError::Unreadable(_))),
        ),
    ];
    for (what, bytes, expected) in cases {
        let error = verify(&bytes, &sha(&bytes), TAG).expect_err(what);
        assert!(expected(&error), "{what}: {error:?}");
    }
    let zip = package(TAG);
    assert_eq!(
        verify(&zip, &sha(b"other"), TAG),
        Err(InstallError::Checksum)
    );
    let huge = vec![0u8; 20 * 1024 * 1024 + 1];
    assert_eq!(verify(&huge, &sha(&huge), TAG), Err(InstallError::TooLarge));
}

#[test]
fn a_licence_and_other_plain_text_files_without_an_extension_are_allowed() {
    // The published packages include the repository's LICENSE.
    let zip = zip_of(&[
        ("Mrglglgl/Mrglglgl.toc", &toc(TAG, Some(1))),
        ("Mrglglgl/LICENSE", "MIT"),
        ("Mrglglgl/README", "read me"),
    ]);
    let checked = verify(&zip, &sha(&zip), TAG).unwrap();
    assert!(checked.files.contains_key(Path::new("LICENSE")));
    // Any other file without an extension, such as a program, is still refused.
    let zip = zip_of(&[
        ("Mrglglgl/Mrglglgl.toc", &toc(TAG, Some(1))),
        ("Mrglglgl/install", "#!/bin/sh"),
    ]);
    assert!(matches!(
        verify(&zip, &sha(&zip), TAG),
        Err(InstallError::BadFile(_))
    ));
}

#[test]
fn a_package_from_before_the_schema_field_reads_as_schema_one() {
    let zip = zip_of(&[("Mrglglgl/Mrglglgl.toc", &toc(TAG, None))]);
    assert_eq!(verify(&zip, &sha(&zip), TAG).unwrap().schema, 1);
}

#[test]
fn a_symlink_in_a_package_is_refused() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("Mrglglgl/Mrglglgl.toc", options).unwrap();
    writer.write_all(toc(TAG, Some(1)).as_bytes()).unwrap();
    writer
        .add_symlink("Mrglglgl/Core.lua", "/etc/passwd", options)
        .unwrap();
    let zip = writer.finish().unwrap().into_inner();
    assert!(matches!(
        verify(&zip, &sha(&zip), TAG),
        Err(InstallError::BadPath(_))
    ));
}

const LISTING: &str = r#"[
  {"tag_name": "addon-v0.3.0-beta.1", "draft": false, "prerelease": true,
   "published_at": "2026-11-05T00:00:00Z",
   "assets": [{"name": "Mrglglgl-addon-v0.3.0-beta.1-forever.zip", "size": 27000,
     "browser_download_url": "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.3.0-beta.1/Mrglglgl-addon-v0.3.0-beta.1-forever.zip",
     "digest": "sha256:13EE512F04DFED45BD7446FFD0D5CFC051FBF3D8B283683029C541C9762EBF70"}]},
  {"tag_name": "companion-v0.5.0", "draft": false, "prerelease": false, "assets": []},
  {"tag_name": "addon-v0.9.0", "draft": true, "prerelease": false, "assets": []},
  {"tag_name": "addon-v0.2.0", "draft": false, "prerelease": false,
   "published_at": "2026-11-01T00:00:00Z",
   "assets": [
     {"name": "release.json", "size": 187,
      "browser_download_url": "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.2.0/release.json",
      "digest": "sha256:b5dc93e6735b91cfacefc38b14afa90a48bc0111ec96ddc9afdf60f6eddd7dc8"},
     {"name": "Mrglglgl-addon-v0.2.0-forever.zip", "size": 26000,
      "browser_download_url": "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.2.0/Mrglglgl-addon-v0.2.0-forever.zip",
      "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000001"}]},
  {"tag_name": "addon-v0.2.5", "draft": false, "prerelease": false,
   "assets": [{"name": "Mrglglgl-addon-v0.2.5-forever.zip", "size": 26000,
     "browser_download_url": "https://example.com/Mrglglgl-addon-v0.2.5-forever.zip",
     "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000002"}]},
  {"tag_name": "addon-v0.2.6", "draft": false, "prerelease": false,
   "assets": [{"name": "Mrglglgl-addon-v0.2.6-forever.zip", "size": 26000,
     "browser_download_url": "https://github.com/AshDevFr/mrglglgl/releases/download/addon-v0.2.6/Mrglglgl-addon-v0.2.6-forever.zip"}]}
]"#;

#[test]
fn picks_the_newest_forever_package_from_the_pinned_repository_with_a_digest() {
    let stable = latest_release(LISTING, false).unwrap();
    assert_eq!(stable.tag, "addon-v0.2.0");
    assert_eq!(stable.version, "0.2.0");
    assert!(stable.url.ends_with("Mrglglgl-addon-v0.2.0-forever.zip"));
    assert!(stable.sha256.ends_with('1'));

    let any = latest_release(LISTING, true).unwrap();
    assert_eq!(any.tag, "addon-v0.3.0-beta.1");
    assert!(any.prerelease);
    assert_eq!(
        any.sha256,
        "13ee512f04dfed45bd7446ffd0d5cfc051fbf3d8b283683029c541c9762ebf70"
    );

    assert_eq!(latest_release("[]", true), None);
    assert_eq!(latest_release("not json", true), None);
}

#[test]
fn versions_order_releases_above_their_test_versions() {
    use std::cmp::Ordering::*;
    assert_eq!(compare_versions("0.2.0", "0.1.9"), Greater);
    assert_eq!(compare_versions("0.10.0", "0.9.0"), Greater);
    assert_eq!(compare_versions("0.2.0", "0.2.0-beta.3"), Greater);
    assert_eq!(compare_versions("0.2.0-beta.10", "0.2.0-beta.9"), Greater);
    assert_eq!(compare_versions("0.2.0-beta.1", "0.2.0-beta.1"), Equal);
    assert_eq!(compare_versions("0.1.0", "0.2.0-beta.1"), Less);
}
