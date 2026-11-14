use fhir_definitions::{ElementDefinition, Registry, StructureDefinition};
use fhir_fhirpath::{Collection, Environment, Value as FhirPathValue};
use fhir_parser::{Node, Resource, Span, Value};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::sync::Arc;

use crate::cardinality::check_children_cardinality;
use crate::diagnostic::Diagnostic;
use crate::embedded::{self, ContainedIds, Embedded, child_element, holds_resource, typed_element};
use crate::profile::check_type_profiles;
use crate::resolve::{complex_type, is_primitive, resolve};
use crate::type_check::{check_type, is_valid_id};
use crate::{invariants, references, terminology};

/// What every level of the walk shares.
struct Walk<'a, 'e> {
    registry: &'a Registry,
    /// `%resource`, `%rootResource` and FHIR type inheritance for every invariant.
    environment: &'e Environment<'e>,
    /// This resource's `%rootResource`, which is also the one of the resources it contains.
    root_resource: &'e FhirPathValue,
    /// JSON input: array shapes are meaningful (`STRUCTURE_002`).
    json: bool,
    /// Where `#id` references resolve (see [`Embedded::container_ids`]).
    contained_ids: ContainedIds,
    /// Inside a contained resource, where `#` alone names the container.
    in_contained: bool,
    /// The resource `id`, which is lifted out of `fields`; it pairs with a root `_id`.
    root_id: Option<Value>,
    /// The instance path of the resource being walked.
    root_path: String,
    /// Resources met in `Resource`-typed elements, validated after this one.
    embedded: RefCell<Vec<Embedded<'a>>>,
}

/// Validate the resource's elements at every depth, starting at the top level (instance
/// `path`), and return the resources embedded in it, for the caller to validate in turn.
///
/// Applies the structural (`STRUCTURE_001`, and `STRUCTURE_002` array shape when `json`),
/// cardinality (`CARDINALITY_001/002/003`), primitive type (`TYPE_*`), terminology
/// (`TERMINOLOGY_*`), reference (`REFERENCE_*`) and invariant (`INVARIANT_*`) checks
/// recursively into backbone elements, complex types and `contentReference` targets. Each
/// node gets the constraints of its element and of its type's root element (`qty-3` on
/// every Quantity), once per key. The root's own invariants and the profile layer run
/// outside the walk. `root` is the resource's FHIRPath value (`resource_to_typed_value`);
/// each object's part of it is handed down the walk, so the tree is converted once.
/// `environment` evaluates every invariant (`%resource` is `root`).
///
/// A `contentReference` can refer to its own ancestor (`Questionnaire.item.item`); the
/// recursion follows the instance data, which the parsers cap at `MAX_DEPTH` levels.
#[allow(clippy::too_many_arguments)] // the resource, its definition and its place
pub(crate) fn validate<'a>(
    resource: &Resource,
    root: &FhirPathValue,
    environment: &Environment<'_>,
    sd: &StructureDefinition,
    registry: &'a Registry,
    json: bool,
    path: &str,
    container_ids: Option<ContainedIds>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Embedded<'a>> {
    // The parser lifts `id` out of `fields` (already rejecting non-strings), without a span.
    if let Some(id) = resource.id.as_deref().filter(|id| !is_valid_id(id)) {
        let id_path = format!("{path}.id");
        diagnostics.push(Diagnostic::error(
            "TYPE_005",
            format!("Invalid id format at '{id_path}': '{id}'"),
            id_path,
        ));
    }
    if let Some(warning) = resource
        .id
        .as_deref()
        .and_then(|id| crate::type_check::control_char_warning(id, &format!("{path}.id")))
    {
        diagnostics.push(warning);
    }

    let in_contained = container_ids.is_some();
    let context = Walk {
        registry,
        environment,
        root_resource: environment.root_resource.unwrap_or(root),
        json,
        contained_ids: container_ids.unwrap_or_else(|| references::contained_ids(resource).into()),
        in_contained,
        root_id: resource.id.clone().map(Value::Str),
        root_path: path.to_owned(),
        embedded: RefCell::new(Vec::new()),
    };
    walk(
        &resource.fields,
        typed_fields(std::slice::from_ref(root)),
        sd,
        &resource.resource_type,
        &context,
        path,
        diagnostics,
    );
    context.embedded.into_inner()
}

/// The FHIRPath children of a single object value.
fn typed_fields(value: &[FhirPathValue]) -> Option<&IndexMap<Arc<str>, Collection>> {
    match value {
        [FhirPathValue::Object(fields)] => Some(fields),
        _ => None,
    }
}

/// Check the value of `elem` (instance `path`), recursing into objects.
///
/// `typed` is the node's FHIRPath value as converted with its parent, when known: an object
/// converts to exactly one value, so an array of objects aligns item by item. Anything else
/// (nulls, a stray `_name`) is converted here instead, so each subtree is converted once.
#[allow(clippy::too_many_arguments)] // recursion state threaded through walk/descend
fn descend(
    node: &Node,
    typed: Option<&[FhirPathValue]>,
    elem: &ElementDefinition,
    ty: &str,
    sd: &StructureDefinition,
    context: &Walk<'_, '_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let registry = context.registry;
    match &node.value {
        Value::Array(items) => {
            let aligned = typed.filter(|typed| {
                typed.len() == items.len()
                    && items.iter().all(|n| matches!(n.value, Value::Object(_)))
            });
            for (i, item) in items.iter().enumerate() {
                descend(
                    item,
                    aligned.map(|typed| &typed[i..=i]),
                    elem,
                    ty,
                    sd,
                    context,
                    &format!("{path}[{i}]"),
                    diagnostics,
                );
            }
        }
        Value::Object(fields) => {
            let start = diagnostics.len();
            let backbone = ty == "BackboneElement" || ty == "Element";
            let type_sd = complex_type(ty, registry);
            if !backbone && type_sd.is_none() {
                return;
            }
            let converted;
            let value = match typed.filter(|typed| typed.len() == 1) {
                Some(typed) => typed,
                None => {
                    // Same definition path as `Registry::child_element`, so both conversions agree.
                    let definition = if backbone { &elem.path } else { ty };
                    converted = node_value(Some(node), None, definition, ty, registry);
                    &converted
                }
            };
            check_invariants(value, elem, ty, path, node.span, context, diagnostics);
            if ty == "Reference" {
                references::check(
                    node,
                    elem,
                    path,
                    &context.contained_ids,
                    context.in_contained,
                    registry,
                    diagnostics,
                );
            }
            let typed = typed_fields(value);
            match type_sd.filter(|_| !backbone) {
                Some(type_sd) => {
                    walk(
                        fields,
                        typed,
                        type_sd,
                        &type_sd.name,
                        context,
                        path,
                        diagnostics,
                    );
                    let node = (fields, value, node.span);
                    check_type_profiles(
                        node,
                        elem,
                        ty,
                        path,
                        registry,
                        context.environment,
                        start,
                        diagnostics,
                    );
                }
                None => walk(fields, typed, sd, &elem.path, context, path, diagnostics),
            }
        }
        _ if ty == "BackboneElement" || ty == "Element" || complex_type(ty, registry).is_some() => {
            diagnostics.push(
                Diagnostic::error(
                    "TYPE_001",
                    format!("Expected {ty} object at '{path}'"),
                    path.to_owned(),
                )
                .with_location(node.span),
            );
        }
        _ => {}
    }
}

/// Validate the children of one object against the elements under `def_path`.
fn walk(
    fields: &IndexMap<Arc<str>, Node>,
    typed: Option<&IndexMap<Arc<str>, Collection>>,
    sd: &StructureDefinition,
    def_path: &str,
    context: &Walk<'_, '_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_children_cardinality(fields, sd, def_path, path, diagnostics);

    let prefix = format!("{def_path}.");

    for (name, node) in fields {
        let name = name.as_ref();
        let child_path = format!("{path}.{name}");
        // `_name` carries the id/extensions of primitive `name`, so it must name a primitive.
        if let Some(bare) = name.strip_prefix('_') {
            let primitive = resolve(sd, &format!("{prefix}{bare}"), bare)
                .and_then(|(elem, ty)| Some((elem, ty.filter(|t| is_primitive(t))?)));
            if let Some((elem, ty)) = primitive {
                // A primitive with a value is checked with its `_name` when the value is reached.
                if !fields.contains_key(bare) {
                    let bare_path = format!("{path}.{bare}");
                    // Only the resource's own `id` is lifted.
                    let lifted = (bare == "id" && path == context.root_path)
                        .then(|| context.root_id.clone())
                        .flatten();
                    let value = lifted.map(|value| Node {
                        value,
                        span: node.span,
                    });
                    check_primitive(
                        value.as_ref(),
                        Some(node),
                        elem,
                        &ty,
                        &bare_path,
                        context,
                        diagnostics,
                    );
                }
                walk_primitive_extension(node, context, &child_path, diagnostics);
                continue;
            }
        }
        let Some((elem, ty)) = resolve(sd, &format!("{prefix}{name}"), name) else {
            diagnostics.push(
                Diagnostic::error(
                    "STRUCTURE_001",
                    format!("Unknown element '{child_path}'"),
                    child_path,
                )
                .with_location(node.span),
            );
            continue;
        };
        if context.json {
            check_array_shape(node, elem, &child_path, diagnostics);
        }
        let Some((elem, sd, ty)) = typed_element(context.registry, sd, elem, ty) else {
            continue;
        };
        let typed = typed.and_then(|typed| typed.get(name)).map(Vec::as_slice);
        if holds_resource(&ty, context.registry) {
            // references.html: only `DomainResource.contained` holds contained resources.
            let contained = name == "contained" && !def_path.contains('.');
            // A contained resource's `%rootResource` is the outermost container, like its ids.
            let container = contained.then_some((&context.contained_ids, context.root_resource));
            let queue = &mut context.embedded.borrow_mut();
            embedded::queue(
                node,
                typed,
                container,
                context.registry,
                &child_path,
                queue,
                diagnostics,
            );
            continue;
        }

        // Element.id is a `string`; R5 snapshots label many as `id` in the fhir-type extension.
        let check_as = if name == "id" { "string" } else { ty.as_ref() };
        let extension = is_primitive(&ty)
            .then(|| fields.get(format!("_{name}").as_str()))
            .flatten();
        check_type(
            &node.value,
            extension.map(|n| &n.value),
            check_as,
            &child_path,
            node.span,
            context.registry,
            diagnostics,
        );
        terminology::check(node, elem, &ty, &child_path, context.registry, diagnostics);
        if is_primitive(&ty) {
            check_primitive(
                Some(node),
                extension,
                elem,
                &ty,
                &child_path,
                context,
                diagnostics,
            );
        }
        descend(
            node,
            typed,
            elem,
            &ty,
            sd,
            context,
            &child_path,
            diagnostics,
        );
    }
}

/// Evaluate the invariants of primitive element `elem` (type `ty`, instance `path`) on each
/// value, paired by index with its `_name` element (R5 json.html: the two arrays align).
fn check_primitive(
    value: Option<&Node>,
    extension: Option<&Node>,
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    context: &Walk<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    fn items(node: Option<&Node>) -> Vec<&Node> {
        match node.map(|n| &n.value) {
            Some(Value::Array(items)) => items.iter().collect(),
            _ => node.into_iter().collect(),
        }
    }
    let repeats = [value, extension]
        .into_iter()
        .flatten()
        .any(|n| matches!(n.value, Value::Array(_)));
    let (values, extensions) = (items(value), items(extension));
    for i in 0..values.len().max(extensions.len()) {
        let (value, extension) = (values.get(i).copied(), extensions.get(i).copied());
        let Some(span) = value.or(extension).map(|n| n.span) else {
            continue;
        };
        let typed = node_value(value, extension, ty, ty, context.registry);
        let item_path = match repeats {
            true => format!("{path}[{i}]"),
            false => path.to_owned(),
        };
        check_invariants(&typed, elem, ty, &item_path, span, context, diagnostics);
    }
}

/// The FHIRPath value of one node (and its `_name` element) whose element has definition
/// path `definition` and type `ty`.
fn node_value(
    value: Option<&Node>,
    extension: Option<&Node>,
    definition: &str,
    ty: &str,
    registry: &Registry,
) -> Collection {
    fhir_fhirpath::node_to_typed_value(
        value.map(|n| &n.value),
        extension.map(|n| &n.value),
        Some(&(Arc::from(definition), Arc::from(ty))),
        &|parent, name| child_element(registry, parent, name),
    )
}

/// Evaluate on one node (instance `path`) the constraints of its element and of its type's
/// root element, each key once: a snapshot element repeats `ele-1` but not `qty-3`.
fn check_invariants(
    value: &[FhirPathValue],
    elem: &ElementDefinition,
    ty: &str,
    path: &str,
    span: Span,
    context: &Walk<'_, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let registry = context.registry;
    // A JSON null or a value-less, extension-less item is no node to constrain.
    if value.is_empty() {
        return;
    }
    let type_constraints = registry
        .structure_definition(&format!("http://hl7.org/fhir/StructureDefinition/{ty}"))
        .and_then(|type_sd| type_sd.element(ty))
        .map(|root| root.constraints.as_slice())
        .unwrap_or_default();
    let inherited = type_constraints
        .iter()
        .filter(|c| !elem.constraints.iter().any(|own| own.key == c.key));
    for constraint in elem.constraints.iter().chain(inherited) {
        invariants::check(
            constraint,
            value,
            &|base, suffix| registry.is_choice_variant(base, suffix),
            context.environment,
            path,
            Some(span),
            diagnostics,
        );
    }
}

/// Check a `_name` primitive extension as an `Element` (an array of them, nulls allowed,
/// for a repeating primitive), so its `id` and `extension` are validated.
fn walk_primitive_extension(
    node: &Node,
    context: &Walk<'_, '_>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(element_sd) = complex_type("Element", context.registry) else {
        return;
    };
    match &node.value {
        // Small subtrees (`id`, `extension`): their children convert where they are checked.
        Value::Object(fields) => walk(
            fields,
            None,
            element_sd,
            "Element",
            context,
            path,
            diagnostics,
        ),
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let item_path = format!("{path}[{i}]");
                match &item.value {
                    Value::Object(fields) => walk(
                        fields,
                        None,
                        element_sd,
                        "Element",
                        context,
                        &item_path,
                        diagnostics,
                    ),
                    Value::Null => {}
                    _ => push_not_element(item, &item_path, diagnostics),
                }
            }
        }
        _ => push_not_element(node, path, diagnostics),
    }
}

fn push_not_element(node: &Node, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(
        Diagnostic::error(
            "TYPE_001",
            format!("Expected Element object at '{path}'"),
            path.to_owned(),
        )
        .with_location(node.span),
    );
}

/// Emit `STRUCTURE_002` when a JSON value's shape contradicts the base definition:
/// a repeating element (`max != "1"`) must be an array, a single one must not be.
///
/// shortcut: `_name` companion arrays are not shape-checked; upgrade with the `_name` rules.
/// shortcut: uses `elem.max`, which equals the base max in every core SD walked here; switch
/// to the base max if `walk` ever runs on constrained (profile) snapshots.
fn check_array_shape(
    node: &Node,
    elem: &ElementDefinition,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let repeating = elem.max.as_ref() != "1";
    let is_array = matches!(node.value, Value::Array(_));
    if is_array == repeating || matches!(node.value, Value::Null) {
        return;
    }
    let message = if repeating {
        format!(
            "Element '{path}' repeats (max={}) and must be a JSON array",
            elem.max
        )
    } else {
        format!("Element '{path}' has max=1 and must not be a JSON array")
    };
    diagnostics.push(
        Diagnostic::error("STRUCTURE_002", message, path.to_owned()).with_location(node.span),
    );
}

#[cfg(test)]
#[path = "nested_tests.rs"]
mod tests;
