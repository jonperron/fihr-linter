## Why

Le crate `fhir-definitions` fournit le socle de données nécessaire à tous les composants du linter FHIR : il charge et indexe les définitions FHIR R5 officielles (StructureDefinitions, ValueSets, CodeSystems) depuis les artefacts JSON bundlés, et expose un registry thread-safe consommable par le validateur et le linter.

## What Changes

- Création du crate `fhir-definitions` (nouveau crate Rust dans le workspace)
- Nouveau modèle de données FHIR R5 (`StructureDefinition`, `ElementDefinition`, `ValueSet`, `CodeSystem`)
- Nouveau chargeur de bundles JSON FHIR (`load_bundle`) avec gestion d'erreurs typées
- Nouveau `Registry` thread-safe pour la résolution par URL canonique

## Capabilities

### New Capabilities
- `fhir-model`: Types Rust représentant les artefacts FHIR R5 (StructureDefinition, ElementDefinition, ValueSet, CodeSystem)
- `fhir-bundle-loader`: Chargement et parsing des bundles JSON FHIR R5 officiels
- `fhir-definitions-registry`: Registry indexé par URL canonique, thread-safe, pour la résolution de définitions FHIR

### Modified Capabilities

## Impact

- Nouveau crate `fhir-definitions` ajouté au workspace Cargo
- Dépendances : `serde`, `serde_json`, `thiserror`
- Fichiers JSON FHIR R5 dans `definitions/r5/` utilisés à l'exécution
- Tous les crates futurs (`fhir-validator`, `fhir-linter`, etc.) dépendront de ce crate
