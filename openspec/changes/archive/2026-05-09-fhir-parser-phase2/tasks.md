## 1. Dépendances et structure

- [x] 1.1 Ajouter `serde_json` et `indexmap` comme dépendances dans `crates/fhir-parser/Cargo.toml`
- [x] 1.2 Définir les types `Span`, `Value`, `Node`, `Resource`, `ParseError` dans `crates/fhir-parser/src/lib.rs`

## 2. Implémentation du parseur JSON

- [x] 2.1 Implémenter `parse_json(source: &str) -> Result<Resource, ParseError>`
- [x] 2.2 Implémenter la reconstruction des spans (ligne/colonne) depuis les offsets `serde_json`
- [x] 2.3 Gérer le cas `resourceType` manquant → `ParseError::MissingResourceType`
- [x] 2.4 Gérer les erreurs de JSON malformé avec position (ligne, colonne)

## 3. Tests unitaires

- [x] 3.1 Test `parse_patient_example_json` : parse une ressource Patient minimale, vérifie `resource_type`
- [x] 3.2 Test `parse_returns_all_top_level_fields` : vérifie que les champs de premier niveau sont tous présents
- [x] 3.3 Test `span_points_to_correct_line_for_field` : vérifie que le `Span` d'un champ indique la bonne ligne
- [x] 3.4 Test `malformed_json_returns_parse_error` : JSON invalide → `Err(ParseError)`
- [x] 3.5 Test `missing_resource_type_returns_error` : objet JSON sans `"resourceType"` → `Err(ParseError::MissingResourceType)`
