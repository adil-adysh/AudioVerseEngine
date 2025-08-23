//! Feature-gated Bevy ecosystem extras wiring (navmesh + raycast).
//! Enabled via the "bevy-extras" Cargo feature.

// Intentionally minimal: keep a helper to mark where extras could be registered.

/// Helper that conditionally registers Bevy extras when the feature is enabled.
pub fn register_extras_if_enabled(_app: &mut bevy_app::App) {
    let _ = _app; // Placeholder: downstream may register specific plugins as needed.
}
