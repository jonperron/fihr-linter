## Why

Le crate `fhir-parser` ne couvrait que le format JSON. FHIR R5 supporte également le format XML ; le linter doit pouvoir valider les ressources XML pour offrir une couverture complète.

## What Changes

- Ajout de `crates/fhir-parser/src/xml.rs` exposant `parse_xml(source: &str) -> Result<Resource, ParseError>`.
- Ajout de la variante `XmlError { message, line, col }` à `ParseError` (type partagé JSON/XML).
- Export public de `parse_xml` et du module `xml` depuis `crates/fhir-parser/src/lib.rs`.

## Capabilities

### New Capabilities
- `fhir-xml-parser`: Parse d'une ressource FHIR R5 depuis XML vers le type `Resource`, avec gestion des primitives, objets complexes, tableaux, XHTML et primitives-avec-extensions.

### Modified Capabilities
- `fhir-json-parser`: La variante `XmlError { message, line, col }` est ajoutée à `ParseError` (type partagé avec le parser XML).

## Impact

- `crates/fhir-parser` : nouveau fichier `xml.rs`, modification de `types.rs` et `lib.rs`.
- Dépendance ajoutée : `quick-xml`.
- API publique étendue : `parse_xml` et la variante `ParseError::XmlError` deviennent stables.
