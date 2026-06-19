//! Runtime configuration parsed from the bundle manifest.
//!
//! [`AppConfig`] mirrors the runtime-relevant slice of `dist/gluxe.manifest.json`
//! (the bundler transcribes it from app.json at JS build time, so the native
//! runtime never reads app.json). [`WindowConfig`] is the window slice, merged
//! field-level — builder → manifest → hard-coded defaults — by
//! [`resolve_window_config`]. Both types are part of the crate's public API and
//! are re-exported from the crate root.

/// Window creation parameters.
///
/// Every field is `Option` (`None` = unspecified). Fields merge field-level —
/// builder → manifest → hard-coded defaults (800×600) — in
/// [`resolve_window_config`]. `Some(String::new())` forces a blank title;
/// `Some(false)` hides the titlebar.
///
/// Per-platform titlebar notes: `Some(false)` removes WS_CAPTION on Windows
/// (resize border remains), enables NSFullSizeContentView on macOS (traffic-
/// light buttons remain), and requests client-side decorations on Linux/X11.
#[derive(Clone, Default)]
pub struct WindowConfig {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub title: Option<String>,
    pub titlebar: Option<bool>,
}

impl WindowConfig {
    /// Fill every unset (`None`) field from `other`. Destructures `other` so
    /// that adding a field to `WindowConfig` without updating this merge is
    /// a compile error.
    fn merge_unset_from(&mut self, other: WindowConfig) {
        let WindowConfig {
            width,
            height,
            title,
            titlebar,
        } = other;
        self.width = self.width.or(width);
        self.height = self.height.or(height);
        self.title = self.title.take().or(title);
        self.titlebar = self.titlebar.or(titlebar);
    }
}

/// Runtime-relevant settings carried by the bundle manifest
/// (`dist/gluxe.manifest.json`). The bundler plugin transcribes these from
/// app.json at JS build time, so the native runtime never reads app.json.
#[derive(Clone, Default)]
pub struct AppConfig {
    /// Dist-root-relative path to the hashed JS entry file.
    pub entry: Option<String>,
    pub window: Option<WindowConfig>,
    /// Dist-root-relative path to the hashed window-icon .ico. Applied on X11
    /// only via `WindowOptions::icon`; ignored on Windows and macOS.
    pub icon: Option<String>,
}

impl AppConfig {
    pub fn from_manifest_str(json: &str) -> Self {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(json) else {
            return Self::default();
        };

        let entry = json["entry"].as_str().map(str::to_owned);
        let window = json.get("window").map(|w| {
            // Reject junk dimensions (negative/zero from a hand-edited
            // manifest); `None` falls through to the defaults at window-open.
            let dim = |v: &serde_json::Value| -> Option<f32> {
                v.as_f64()
                    .filter(|n| n.is_finite() && *n > 0.0)
                    .map(|n| n as f32)
            };
            WindowConfig {
                width: dim(&w["width"]),
                height: dim(&w["height"]),
                title: w["title"].as_str().map(str::to_owned),
                titlebar: w["titlebar"].as_bool(),
            }
        });

        let icon = json["icon"].as_str().map(str::to_owned);

        Self {
            entry,
            window,
            icon,
        }
    }
}

/// Merge builder and manifest window configs field-level: the builder wins,
/// the manifest fills any `None`, and the defaults (800×600) are applied later
/// at window-open. A partial builder override (e.g. `.titlebar()` only) still
/// inherits the manifest's other fields. Extracted for unit testing.
pub(crate) fn resolve_window_config(
    builder: Option<WindowConfig>,
    manifest: Option<WindowConfig>,
) -> WindowConfig {
    let mut cfg = builder.unwrap_or_default();
    if let Some(m) = manifest {
        cfg.merge_unset_from(m);
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- AppConfig::from_manifest_str ----

    #[test]
    fn manifest_with_title_parses_title() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":1024,"height":768,"title":"My App"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.title.as_deref(), Some("My App"));
        assert_eq!(window.width, Some(1024.0));
        assert_eq!(window.height, Some(768.0));
    }

    #[test]
    fn manifest_window_without_title_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.title.is_none());
    }

    #[test]
    fn manifest_without_window_key_gives_no_window() {
        let json = r#"{"version":1,"entry":"assets/index.js"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.window.is_none());
    }

    #[test]
    fn manifest_with_junk_dimensions_gives_none() {
        // Negative / zero dimensions (hand-edited manifest) must not reach
        // px()/size(); they parse to None and the defaults apply at window-open time.
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":-100,"height":0}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.width, None);
        assert_eq!(window.height, None);
    }

    #[test]
    fn manifest_with_title_only_has_none_dimensions() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"title":"Titled"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.title.as_deref(), Some("Titled"));
        assert_eq!(window.width, None);
        assert_eq!(window.height, None);
    }

    #[test]
    fn manifest_with_icon_parses_icon() {
        let json = r#"{"version":1,"entry":"assets/index.js","icon":"assets/app-icon-Xyz.ico"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert_eq!(config.icon.as_deref(), Some("assets/app-icon-Xyz.ico"));
    }

    #[test]
    fn manifest_without_icon_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js"}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.icon.is_none());
    }

    #[test]
    fn manifest_with_icon_not_string_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","icon":5}"#;
        let config = AppConfig::from_manifest_str(json);
        assert!(config.icon.is_none());
    }

    // ---- AppConfig::from_manifest_str — titlebar ----

    #[test]
    fn manifest_titlebar_false_parses_some_false() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":false}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.titlebar, Some(false));
    }

    #[test]
    fn manifest_titlebar_true_parses_some_true() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":true}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert_eq!(window.titlebar, Some(true));
    }

    #[test]
    fn manifest_titlebar_absent_gives_none() {
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.titlebar.is_none());
    }

    #[test]
    fn manifest_titlebar_non_bool_gives_none() {
        // Non-bool values (numbers, strings, null) must be ignored — only actual
        // JSON booleans are valid for the titlebar field.
        let json = r#"{"version":1,"entry":"assets/index.js","window":{"width":800,"height":600,"titlebar":"false"}}"#;
        let config = AppConfig::from_manifest_str(json);
        let window = config.window.expect("window present");
        assert!(window.titlebar.is_none());
    }

    // ---- resolve_window_config ----

    #[test]
    fn resolve_builder_none_manifest_some_uses_manifest() {
        let manifest = Some(WindowConfig {
            width: Some(1024.0),
            height: Some(768.0),
            title: Some("Manifest Title".to_string()),
            titlebar: Some(false),
        });
        let result = resolve_window_config(None, manifest);
        assert_eq!(result.width, Some(1024.0));
        assert_eq!(result.title.as_deref(), Some("Manifest Title"));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn resolve_builder_overrides_manifest_dimensions_inherits_title_and_titlebar() {
        // .window_size() only sets width/height, leaving title and titlebar as None.
        // Those fields should be inherited from the manifest.
        let builder = Some(WindowConfig {
            width: Some(1280.0),
            height: Some(720.0),
            title: None,
            titlebar: None,
        });
        let manifest = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: Some("My App".to_string()),
            titlebar: Some(false),
        });
        let result = resolve_window_config(builder, manifest);
        // Dimensions come from the builder (it wins over manifest).
        assert_eq!(result.width, Some(1280.0));
        assert_eq!(result.height, Some(720.0));
        // title and titlebar are inherited from the manifest because the builder
        // left them as None (field-level merge).
        assert_eq!(result.title.as_deref(), Some("My App"));
        assert_eq!(result.titlebar, Some(false));
    }

    #[test]
    fn resolve_builder_some_true_titlebar_beats_manifest_false() {
        // An explicit builder `titlebar: Some(true)` must win over a manifest
        // `titlebar: Some(false)` — the builder is highest priority.
        let builder = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: None,
            titlebar: Some(true),
        });
        let manifest = Some(WindowConfig {
            width: Some(800.0),
            height: Some(600.0),
            title: None,
            titlebar: Some(false),
        });
        let result = resolve_window_config(builder, manifest);
        assert_eq!(result.titlebar, Some(true));
    }

    #[test]
    fn resolve_both_none_gives_all_none() {
        // Defaults (800×600) are applied at window-open time, not here.
        let result = resolve_window_config(None, None);
        assert_eq!(result.width, None);
        assert_eq!(result.height, None);
        assert!(result.title.is_none());
        assert!(result.titlebar.is_none());
    }

    // ---- resolve_window_config — regression tests for builder bugs ----

    #[test]
    fn resolve_titlebar_only_builder_inherits_manifest_size_and_title() {
        // Bug 1 regression: `.titlebar(false)` alone must NOT fabricate 800×600.
        // Width/height/title should come from the manifest.
        let builder = Some(WindowConfig {
            titlebar: Some(false),
            ..Default::default()
        });
        let manifest = Some(WindowConfig {
            width: Some(1024.0),
            height: Some(768.0),
            title: Some("App".to_string()),
            titlebar: None,
        });
        let result = resolve_window_config(builder, manifest);
        assert_eq!(result.width, Some(1024.0));
        assert_eq!(result.height, Some(768.0));
        assert_eq!(result.title.as_deref(), Some("App"));
        assert_eq!(result.titlebar, Some(false));
    }
}
