use serde::Deserialize;

/// The subset of Claude Code's status-line JSON we read from stdin. Every field
/// is optional: a missing key or an explicit `null` both deserialize to `None`, so
/// first-boot payloads (which send some fields as `null`) parse cleanly instead of
/// failing the whole line.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Input {
    pub(crate) model: Option<Model>,
    pub(crate) effort: Option<Effort>,
    pub(crate) context_window: Option<ContextWindow>,
    pub(crate) cwd: Option<String>,
    pub(crate) transcript_path: Option<String>,
    pub(crate) rate_limits: Option<RateLimits>,
}

/// 5h/7d usage windows, sent by Claude Code for Pro/Max subscribers after the
/// session's first API response; absent otherwise.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RateLimits {
    pub(crate) five_hour: Option<RateWindow>,
    pub(crate) seven_day: Option<RateWindow>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RateWindow {
    pub(crate) used_percentage: Option<f64>,
    /// Unix epoch seconds.
    pub(crate) resets_at: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Model {
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Effort {
    pub(crate) level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ContextWindow {
    pub(crate) used_percentage: Option<f64>,
    pub(crate) total_input_tokens: Option<u64>,
    pub(crate) total_output_tokens: Option<u64>,
    pub(crate) context_window_size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_and_missing_fields_parse_to_none() {
        // First-boot shape: structs and numbers arrive as `null`.
        let json = r#"{
            "model": null,
            "effort": null,
            "context_window": {
                "used_percentage": null,
                "total_input_tokens": null,
                "total_output_tokens": null,
                "context_window_size": null
            },
            "cwd": null,
            "transcript_path": null,
            "rate_limits": null
        }"#;
        let input: Input = serde_json::from_str(json).expect("null fields must parse");
        assert!(input.model.is_none());
        assert!(input.effort.is_none());
        let cw = input.context_window.expect("object present");
        assert!(cw.used_percentage.is_none());
        assert!(cw.context_window_size.is_none());

        // A null context_window object and an empty payload also parse.
        assert!(serde_json::from_str::<Input>(r#"{"context_window":null}"#).is_ok());
        assert!(serde_json::from_str::<Input>("{}").is_ok());

        // rate_limits with null windows / null members also parses.
        let input: Input = serde_json::from_str(
            r#"{"rate_limits": {"five_hour": null,
                "seven_day": {"used_percentage": null, "resets_at": null}}}"#,
        )
        .expect("null rate-limit fields must parse");
        let rl = input.rate_limits.expect("object present");
        assert!(rl.five_hour.is_none());
        let sd = rl.seven_day.expect("object present");
        assert!(sd.used_percentage.is_none());
        assert!(sd.resets_at.is_none());
    }
}
