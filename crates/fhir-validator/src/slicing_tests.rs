//! `ordered`, `openAtEnd` and `exists` slicing on R5 core profiles edited in memory: no
//! bundled profile has an ordered or openAtEnd slicing these rules can evaluate.
use std::path::PathBuf;
use std::sync::OnceLock;

use fhir_definitions::{ChoiceValue, Discriminator, StructureDefinition, TypeProfiles};
use fhir_parser::parse_json;

use super::*;
use crate::cardinality::own_descendants;
use crate::profile::{Layer, check_profile_object};

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../definitions/r5");
        Registry::from_definitions_dir(dir).expect("failed to load R5 definitions")
    })
}

/// The R5 core profile `name` with its snapshot edited by `edit`.
fn edited(name: &str, edit: impl FnOnce(&mut Vec<ElementDefinition>)) -> StructureDefinition {
    let url = format!("http://hl7.org/fhir/StructureDefinition/{name}");
    let core = registry().structure_definition(&url).expect("core profile");
    let mut snapshot = core.snapshot.clone();
    edit(&mut snapshot);
    StructureDefinition::new(
        Arc::from(url),
        core.name.clone(),
        core.kind.clone(),
        false,
        snapshot,
    )
}

/// Position of slice `name` of `path`.
fn slice(snapshot: &[ElementDefinition], path: &str, name: &str) -> usize {
    snapshot
        .iter()
        .position(|e| *e.path == *path && e.slice_name.as_deref() == Some(name))
        .expect("slice")
}

/// Position of the first element with `path` after `start`.
fn after(snapshot: &[ElementDefinition], start: usize, path: &str) -> usize {
    start
        + snapshot[start..]
            .iter()
            .position(|e| *e.path == *path)
            .expect("element")
}

/// `(code, path)` of every error the profile `sd` gives on the Bundle `json`.
fn errors(sd: &StructureDefinition, json: &str) -> Vec<(String, String)> {
    found(sd, json, Severity::Error)
}

/// `(code, path)` of every diagnostic of `severity` the profile `sd` gives on `json`.
fn found(sd: &StructureDefinition, json: &str, severity: Severity) -> Vec<(String, String)> {
    let resource = parse_json(json).expect("test JSON parses");
    let environment = Environment::default();
    let layer = Layer {
        registry: registry(),
        environment: &environment,
    };
    let mut diagnostics = Vec::new();
    let run = own_descendants(sd, "Bundle");
    check_profile_object(
        &resource.fields,
        sd,
        run,
        "Bundle",
        "Bundle",
        layer,
        &mut diagnostics,
    );
    diagnostics
        .into_iter()
        .filter(|d| d.severity == severity)
        .map(|d| (d.code.to_string(), d.path))
        .collect()
}

fn bundle_of(kind: &str, entries: &[&str]) -> String {
    let entries = entries.join(",");
    format!(r#"{{"resourceType":"Bundle","type":"{kind}","entry":[{entries}]}}"#)
}

fn bundle(entries: &[&str]) -> String {
    bundle_of("history", entries)
}

fn notification_bundle(entries: &[&str]) -> String {
    bundle_of("subscription-notification", entries)
}

fn history_entry(method: &str) -> String {
    format!(
        r#"{{"fullUrl":"http://example.org/fhir/Patient/p",
        "request":{{"method":"{method}","url":"Patient/p"}},"response":{{"status":"200"}}}}"#
    )
}

/// history-bundle, entry slicing set to `rules` and `ordered`, slice `patch` given the
/// method pattern R5 leaves out (so an open slicing can be evaluated).
fn history(rules: &str, ordered: bool) -> StructureDefinition {
    edited("history-bundle", |snapshot| {
        let entry = snapshot
            .iter()
            .position(|e| &*e.path == "Bundle.entry")
            .unwrap();
        let slicing = snapshot[entry].slicing.as_mut().unwrap();
        slicing.rules = Arc::from(rules);
        slicing.ordered = ordered;
        let patch = slice(snapshot, "Bundle.entry", "patch");
        let method = after(snapshot, patch, "Bundle.entry.request.method");
        snapshot[method].pattern = Some(Box::new(ChoiceValue {
            key: Arc::from("patternCode"),
            value: serde_json::json!("PATCH"),
        }));
    })
}

#[test]
fn ordered_slicing_rejects_items_out_of_slice_order() {
    // history-bundle declares slice `get` before slice `delete`.
    let sd = history("closed", true);
    let (get, delete) = (history_entry("GET"), history_entry("DELETE"));
    assert_eq!(
        errors(&sd, &bundle(&[&delete, &get])),
        [("PROFILE_008".to_owned(), "Bundle.entry[1]".to_owned())]
    );
    assert_eq!(errors(&sd, &bundle(&[&get, &delete, &delete])), []);
}

#[test]
fn unordered_slicing_accepts_any_order() {
    let sd = history("closed", false);
    let (get, delete) = (history_entry("GET"), history_entry("DELETE"));
    assert_eq!(errors(&sd, &bundle(&[&delete, &get])), []);
}

#[test]
fn open_at_end_slicing_puts_unmatched_items_last() {
    let sd = history("openAtEnd", false);
    let (get, head) = (history_entry("GET"), history_entry("HEAD"));
    assert_eq!(
        errors(&sd, &bundle(&[&head, &get])),
        [("PROFILE_009".to_owned(), "Bundle.entry[1]".to_owned())]
    );
    assert_eq!(errors(&sd, &bundle(&[&get, &head])), []);
}

#[test]
fn open_slicing_accepts_unmatched_items_anywhere() {
    let sd = history("open", false);
    let (get, head) = (history_entry("GET"), history_entry("HEAD"));
    assert_eq!(errors(&sd, &bundle(&[&head, &get])), []);
}

/// subscription-notification-bundle with its entry slices told apart by `exists` on
/// `response`: `first` forbids it (R5, max 0), `other` is made to require it (1..1).
fn notification() -> StructureDefinition {
    edited("subscription-notification-bundle", |snapshot| {
        let entry = snapshot
            .iter()
            .position(|e| &*e.path == "Bundle.entry")
            .unwrap();
        snapshot[entry].slicing.as_mut().unwrap().discriminators = vec![Discriminator {
            kind: Arc::from("exists"),
            path: Arc::from("response"),
        }];
        let other = slice(snapshot, "Bundle.entry", "other");
        let response = after(snapshot, other, "Bundle.entry.response");
        snapshot[response].min = 1;
        snapshot[response].max = Arc::from("1");
    })
}

#[test]
fn exists_discriminator_assigns_items_by_presence() {
    let sd = notification();
    let first = r#"{"fullUrl":"urn:uuid:1",
        "resource":{"resourceType":"SubscriptionStatus","type":"event-notification",
        "subscription":{"reference":"Subscription/s"}}}"#;
    let other = r#"{"fullUrl":"urn:uuid:2","response":{"status":"200"}}"#;
    assert_eq!(errors(&sd, &notification_bundle(&[first, other])), []);
    // Both entries have a response: neither is `first`, which needs one item.
    assert_eq!(
        errors(&sd, &notification_bundle(&[other, other])),
        [("PROFILE_006".to_owned(), "Bundle.entry".to_owned())]
    );
    // `other` before `first` breaks the ordered slicing.
    assert_eq!(
        errors(&sd, &notification_bundle(&[other, first])),
        [("PROFILE_008".to_owned(), "Bundle.entry[1]".to_owned())]
    );
}

/// history-bundle (closed) with its entry discriminator replaced by `kind` on `path`.
fn history_discriminated(kind: &str, path: &str) -> StructureDefinition {
    let sd = history("closed", false);
    edited_from(&sd, |snapshot| {
        let entry = snapshot
            .iter()
            .position(|e| &*e.path == "Bundle.entry")
            .unwrap();
        snapshot[entry].slicing.as_mut().unwrap().discriminators = vec![Discriminator {
            kind: Arc::from(kind),
            path: Arc::from(path),
        }];
    })
}

fn edited_from(
    sd: &StructureDefinition,
    edit: impl FnOnce(&mut Vec<ElementDefinition>),
) -> StructureDefinition {
    let mut snapshot = sd.snapshot.clone();
    edit(&mut snapshot);
    StructureDefinition::new(
        sd.url.clone(),
        sd.name.clone(),
        sd.kind.clone(),
        false,
        snapshot,
    )
}

#[test]
fn pattern_discriminator_matches_like_value() {
    let sd = history_discriminated("pattern", "request.method");
    let (get, head) = (history_entry("GET"), history_entry("HEAD"));
    assert_eq!(errors(&sd, &bundle(&[&get])), []);
    assert_eq!(
        errors(&sd, &bundle(&[&head])),
        [("PROFILE_007".to_owned(), "Bundle.entry[0]".to_owned())]
    );
}

#[test]
fn unknown_discriminator_profile_skips_the_slicing() {
    let sd = history_discriminated("profile", "$this");
    let sd = edited_from(&sd, |snapshot| {
        let unknown = TypeProfiles {
            code: Arc::from("BackboneElement"),
            profiles: vec![Arc::from("http://example.org/StructureDefinition/unknown")],
            target_profiles: vec![],
        };
        for e in snapshot.iter_mut().filter(|e| &*e.path == "Bundle.entry") {
            if e.slice_name.is_some() {
                e.type_profiles = vec![unknown.clone()];
            }
        }
    });
    let json = bundle(&[&history_entry("GET")]);
    assert_eq!(errors(&sd, &json), []);
    assert_eq!(
        found(&sd, &json, Severity::Information),
        [("SLICING_SKIPPED".to_owned(), "Bundle.entry".to_owned())]
    );
}

#[test]
fn unsupported_discriminator_kind_skips_the_slicing() {
    let sd = history_discriminated("position", "$this");
    let json = bundle(&[&history_entry("HEAD")]);
    assert_eq!(errors(&sd, &json), []);
    assert_eq!(
        found(&sd, &json, Severity::Information),
        [("SLICING_SKIPPED".to_owned(), "Bundle.entry".to_owned())]
    );
}
