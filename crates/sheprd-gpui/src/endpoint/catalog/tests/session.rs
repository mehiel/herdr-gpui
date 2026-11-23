use super::*;

fn session(name: &str) -> ConnectTarget {
    ConnectTarget::Session {
        name: name.into(),
        development: false,
    }
}

#[test]
fn the_catalog_reads_the_machines_of_the_session_it_was_opened_on() {
    assert_eq!(
        Catalog::new(&session("factory")).session.as_deref(),
        Some("factory")
    );
    assert_eq!(
        Catalog::new(&ConnectTarget::Socket("/unused.sock".into())).session,
        None
    );
}

#[test]
fn switching_session_drops_the_old_load_and_reloads_at_once() {
    let mut catalog = Catalog::new(&session("factory"));
    let (tx, rx) = mpsc::sync_channel(1);
    catalog.pending = Some(rx);
    let later = Instant::now() + Duration::from_secs(60);
    catalog.next_poll = later;

    // The same session keeps the load in flight and the poll schedule.
    catalog.follow(Some("factory".into()));
    assert!(catalog.pending.is_some());
    assert_eq!(catalog.next_poll, later);

    catalog.follow(Some("horizon".into()));
    assert_eq!(catalog.session.as_deref(), Some("horizon"));
    assert!(catalog.pending.is_none());
    assert!(catalog.next_poll <= Instant::now());
    // A late result for the session the window left is never seen.
    let _ = tx.send(Err(std::io::Error::other("factory's catalog").into()));
    drop(tx);
    assert!(catalog.pending.is_none());
}
