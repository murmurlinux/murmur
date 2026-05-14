//! Local smoke test for the `keyring` crate against a real
//! secret-service daemon. Marked #[ignore] so CI runners (which have
//! no D-Bus session bus or keyring daemon) skip it; run locally with
//! `cargo test --test byok_keyring_smoke -- --ignored` on a desktop
//! session that has gnome-keyring or kwallet running.

#[test]
#[ignore = "needs a real session-bus keyring (gnome-keyring/kwallet)"]
fn keyring_round_trips_with_real_daemon() {
    let service = "murmur-test";
    let user = format!("byok-smoke-{}", std::process::id());
    let entry = keyring::Entry::new(service, &user).expect("entry create");
    let value = format!("gsk_smoke_{}", std::process::id());

    entry.set_password(&value).expect("set_password");
    let got = entry.get_password().expect("get_password");
    assert_eq!(got, value, "round-trip mismatch");
    entry.delete_credential().expect("delete_credential");

    match entry.get_password() {
        Err(keyring::Error::NoEntry) => {}
        other => panic!("expected NoEntry after delete, got {other:?}"),
    }
}
