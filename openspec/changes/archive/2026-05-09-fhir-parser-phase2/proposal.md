## Why

Le crate `fhir-parser` est actuellement vide. Le validateur et le linter ne peuvent pas fonctionner sans un parseur capable de lire les ressources FHIR JSON et de préserver les positions sources pour des messages d'erreur précis.

## What Changes

- Ajout du type `Resource { resource_type, id, fields }` représentant une ressource FHIR parsée.
- Ajout du type `Node` portant une valeur (`Value`) et un `Span` (position source).
- Ajout du type `Span { file, line, col, offset }` pour localiser chaque nœud dans la source.
- Ajout de la fonction `parse_json(source: &str) -> Result<Resource, ParseError>`.
- Gestion des erreurs de JSON malformé avec localisation précise.

## Capabilities

### New Capabilities

- `fhir-json-parser`: Parse les ressources FHIR R5 au format JSON en un AST typé `Resource` avec positions sources (`Span`) sur chaque nœud.

### Modified Capabilities

## Impact

- Crate `fhir-parser` : implémentation complète.
- Dépendances ajoutées : `serde_json`, `indexmap`.
- Crates `fhir-validator` et `fhir-linter` consommeront ce type `Resource` dans les phases suivantes.
