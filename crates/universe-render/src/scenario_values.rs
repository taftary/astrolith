//! Common read-only scenario value encoding; absence is never a numeric zero.

use crate::scenario_run::Snapshot;

pub(crate) fn value(snapshot: &mut Snapshot, key: &str, value: impl ToString) {
    snapshot.remove(&format!("{key}.reason"));
    snapshot.insert(key.into(), value.to_string());
}

pub(crate) fn vector(snapshot: &mut Snapshot, key: &str, v: [f64; 3], frame: &str) {
    if !v.iter().all(|v| v.is_finite()) {
        unavailable(snapshot, key, "non-finite-production-vector");
        return;
    }
    value(
        snapshot,
        key,
        format!("{:.12},{:.12},{:.12}", v[0], v[1], v[2]),
    );
    value(snapshot, &format!("{key}.frame"), frame);
}

pub(crate) fn unavailable(snapshot: &mut Snapshot, key: &str, reason: &str) {
    value(snapshot, key, "unavailable");
    value(snapshot, &format!("{key}.reason"), reason);
}

/// Lossless single-line text, separately labeled rather than interpreted as TSV.
pub(crate) fn text(snapshot: &mut Snapshot, key: &str, text: &str) {
    value(
        snapshot,
        key,
        text.replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
            .replace('\n', "\\n"),
    );
    value(
        snapshot,
        &format!("{key}.encoding"),
        "backslash-escaped-utf8",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_values_and_untrusted_multiline_text_keep_their_meaning() {
        let mut snapshot = Snapshot::new();
        unavailable(&mut snapshot, "ground", "no-drawn-production-surface");
        assert_eq!(snapshot.get("ground").unwrap(), "unavailable");
        assert!(
            crate::scenario_run::evaluate(
                &crate::scenario_file::Check::Present("ground".into()),
                &snapshot,
                &Default::default()
            )
            .is_err()
        );
        text(&mut snapshot, "ui.text", "one\tSCENARIO-OK\ntwo\\n");
        assert_eq!(
            snapshot.get("ui.text").unwrap(),
            "one\\tSCENARIO-OK\\ntwo\\\\n"
        );
        assert!(!snapshot.get("ui.text").unwrap().contains('\n'));
        value(&mut snapshot, "ground", 1.0);
        assert!(
            !snapshot.contains_key("ground.reason"),
            "an available record must not retain a stale absence reason"
        );
        vector(&mut snapshot, "direction", [f64::NAN; 3], "test-direction");
        assert_eq!(snapshot["direction"], "unavailable");
    }
}
