//! Smoke test: the `shared_setup` symbol is reachable from outside the
//! crate with the exact signature the Pro desktop binary (in
//! murmurlinux/murmur-pro) will consume. If this compiles, the open-core
//! composition point works. Runtime exercise is via manual smoke tests of
//! the two desktop binaries.

#[test]
fn shared_setup_symbol_exists() {
    let _f: fn(&mut tauri::App) -> Result<(), Box<dyn std::error::Error>> =
        murmur_lib::shared_setup;
}
