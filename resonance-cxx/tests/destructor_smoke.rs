// Smoke test for C++ destructor behavior via the cxx bridge.
// Create and drop Api repeatedly to ensure the C++ destructor task queue runs cleanly.

#[test]
fn destructor_runs_cleanly() {
    for _ in 0..10 {
        let mut api = resonance_cxx::Api::new(2, 64, 48000).expect("create api");
        // optionally call a setter
        api.set_master_volume(0.5);
        drop(api);
    }
}
