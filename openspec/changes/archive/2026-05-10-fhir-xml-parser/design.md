## Context

Le crate `fhir-parser` expose déjà `parse_json` pour le format JSON. La Phase 3 ajoute `parse_xml` pour le format XML en réutilisant les mêmes types de sortie (`Resource`, `Node`, `Value`, `Span`) et en étendant `ParseError` avec une variante `XmlError`.

## Goals

- Fournir `parse_xml(source: &str) -> Result<Resource, ParseError>` avec la même signature de retour que `parse_json`.
- Couvrir les conventions XML FHIR R5 : primitives via attribut `value`, éléments complexes, répétitions, XHTML `<div>`, primitives-avec-extensions.
- Préserver les positions sources (`Span`) pour chaque `Node`.

## Non-Goals

- Support des namespaces XML (ignorés via `local_name()`).
- Validation FHIR (responsabilité de `fhir-validator`).
- Support de XML autre que UTF-8.

## Decisions

| Décision | Choix | Raison |
|----------|-------|--------|
| Bibliothèque XML | `quick-xml` | Zéro-copie, streaming, déjà dans l'écosystème Rust FHIR |
| Namespaces | Ignorés (nom local uniquement) | Cohérence avec les ressources FHIR qui n'utilisent qu'un seul namespace |
| XHTML | Texte brut via `read_to_end` | Le linter n'a pas besoin de traverser le XHTML structurellement |
| Primitive+extension | `Value::Object` avec clé `"value"` en tête | Alignement avec la représentation JSON FHIR étendue |
| `ParseError::XmlError` | Variante partagée dans `types.rs` | Évite la duplication, type unifié pour tous les formats |

## Risks

- Les ressources FHIR XML très volumineuses avec beaucoup d'imbrication pourraient consommer de la mémoire ; acceptable pour la Phase 3 (pas d'optimisation streaming requise).
- Le texte brut XHTML peut contenir des entités HTML non décodées ; acceptable car le linter n'interprète pas le contenu narratif.
