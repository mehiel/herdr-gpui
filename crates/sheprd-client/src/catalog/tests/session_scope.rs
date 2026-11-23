use super::*;

fn host(id: char, target: &str) -> SavedHost {
    SavedHost {
        id: id.to_string().repeat(32),
        label: format!("Device {target}"),
        target: target.into(),
        session: "default".into(),
        enabled: true,
    }
}

/// Write `hosts` as upstream writes a catalog file, creating its directory.
fn write(path: &Path, hosts: &[SavedHost]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let content = serde_json::to_vec(&Catalog {
        version: 1,
        selected_profile: None,
        ssh: hosts.to_vec(),
    })
    .unwrap();
    fs::write(path, content).unwrap();
}

fn session_file(root: &Path, session: &str) -> PathBuf {
    root.join("sessions").join(session).join("endpoints.json")
}

#[test]
fn session_catalogs_sit_beside_the_shared_one_and_refuse_escaping_names() {
    let shared = Path::new("/state/herdr/client/endpoints.json");
    assert_eq!(
        session_catalog_path(shared, "factory").unwrap(),
        Path::new("/state/herdr/client/sessions/factory/endpoints.json")
    );
    for name in ["", ".", "..", "a/b", "../client"] {
        assert_eq!(session_catalog_path(shared, name), None, "{name:?}");
    }
}

#[test]
fn a_session_shows_the_shared_machines_then_only_its_own() {
    let root = tempfile::tempdir().unwrap();
    let shared = root.path().join("endpoints.json");
    let build = host('a', "build");
    let xps = host('b', "xps");
    let other = host('c', "other");
    write(&shared, std::slice::from_ref(&build));
    write(
        &session_file(root.path(), "factory"),
        std::slice::from_ref(&xps),
    );
    write(
        &session_file(root.path(), "horizon"),
        std::slice::from_ref(&other),
    );

    let factory = load_merged(&shared, Some("factory")).unwrap().ssh;
    assert_eq!(factory, vec![build.clone(), xps]);
    let horizon = load_merged(&shared, Some("horizon")).unwrap().ssh;
    assert_eq!(horizon, vec![build.clone(), other]);
    // A session without a file of its own, and no session at all, read the
    // shared catalog alone, as before.
    for session in [Some("default"), None] {
        assert_eq!(
            load_merged(&shared, session).unwrap().ssh,
            std::slice::from_ref(&build)
        );
    }
}

#[test]
fn a_session_machine_can_be_selected_and_restored_only_in_its_session() {
    let root = tempfile::tempdir().unwrap();
    let shared = root.path().join("endpoints.json");
    let xps = host('b', "xps");
    write(&shared, &[host('a', "build")]);
    write(
        &session_file(root.path(), "factory"),
        std::slice::from_ref(&xps),
    );

    store_selection(&shared, Some("factory"), Some(&xps.id)).unwrap();
    // The shared file keeps the format older clients read.
    assert_eq!(load_path(&shared).unwrap().len(), 1);
    let (hosts, selected) = load_with_selection(&shared, Some("factory")).unwrap();
    assert_eq!(hosts.len(), 2);
    assert_eq!(selected.as_deref(), Some(xps.id.as_str()));
    // Another session neither offers nor restores it.
    assert_eq!(
        load_with_selection(&shared, Some("horizon")).unwrap().1,
        None
    );
    assert!(matches!(
        store_selection(&shared, Some("horizon"), Some(&xps.id)),
        Err(Error::Storage { source, .. }) if matches!(*source, Error::SelectionUnavailable)
    ));
}

#[test]
fn a_session_file_is_held_to_the_shared_schema_and_unique_ids() {
    let root = tempfile::tempdir().unwrap();
    let shared = root.path().join("endpoints.json");
    let build = host('a', "build");
    write(&shared, std::slice::from_ref(&build));
    let factory = session_file(root.path(), "factory");

    write(&factory, &[build]);
    let error = load_merged(&shared, Some("factory")).err().unwrap();
    let Error::Storage {
        operation,
        path,
        source,
    } = &error
    else {
        panic!("missing storage context")
    };
    assert_eq!(*operation, StorageOperation::Validate);
    assert_eq!(path, &factory);
    assert!(matches!(source.as_ref(), Error::ProfileId));

    fs::write(&factory, br#"{"version":1,"ssh":[],"local_session":"x"}"#).unwrap();
    assert!(matches!(
        load_merged(&shared, Some("factory")),
        Err(Error::Storage { source, .. }) if matches!(*source, Error::CatalogSchema(_))
    ));
}
