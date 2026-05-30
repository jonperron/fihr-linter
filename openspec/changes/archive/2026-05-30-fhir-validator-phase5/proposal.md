## Why

Le projet dispose d'un parser FHIR R5, d'un registry de définitions et d'un évaluateur FHIRPath, mais n'avait pas encore de couche de validation capable de détecter les erreurs structurelles, de cardinalité, de type et d'invariants dans les ressources FHIR. La Phase 5 comble ce manque en introduisant un validateur de base opérationnel.

## What Changes

- Nouveau crate `fhir-validator` exposant une fonction publique `validate(resource, registry) -> Vec<Diagnostic>`
- Nouveau type `Diagnostic` avec les champs : severity, code, message, path, location optionnelle, rule optionnelle
- Nouveau type `Severity` (`Error`, `Warning`, `Information`, `Hint`)
- Couche structurelle : détection des types de ressources inconnus (STRUCTURE_000) et des éléments inconnus (STRUCTURE_001)
- Couche cardinalité : vérification des contraintes min/max (CARDINALITY_001, CARDINALITY_002) en ignorant les placeholders polymorphiques `[x]`
- Couche type : validation des formats primitifs pour `boolean`, `integer`, `unsignedInt`, `positiveInt`, `decimal`, `date`, `dateTime`, `instant`, `time`, `code`, `id`, et les types chaîne (TYPE_001 à TYPE_005)
- Couche terminologie : stub prévu pour expansion future des ValueSets
- Couche invariants : évaluation des expressions FHIRPath de contraintes (INVARIANT_<key>) depuis l'élément racine du snapshot
- Extension du modèle `fhir-definitions` : nouveaux types `Constraint` (key, severity, human, expression optionnelle) et `Binding` (strength, value_set optionnel) ajoutés à `ElementDefinition`

## Capabilities

### New Capabilities
- `fhir-validator`: Validateur FHIR R5 multi-couches produisant des `Diagnostic` structurés

### Modified Capabilities
- `fhir-definitions-registry`: Ajout des types `Constraint` et `Binding` au modèle `ElementDefinition`; le loader parse désormais les constraints et bindings depuis les éléments snapshot

## Impact

- `crates/fhir-validator/` : nouveau crate entièrement créé (6 modules + tests d'intégration)
- `crates/fhir-definitions/src/model.rs` : ajout de `Constraint` et `Binding`
- `crates/fhir-definitions/src/loader.rs` : parsing des contraintes et bindings
- `crates/fhir-definitions/src/lib.rs` : export public de `Binding` et `Constraint`
- Dépendances : `fhir-validator` dépend de `fhir-definitions`, `fhir-parser`, et `fhir-fhirpath`
- 44 tests au total (35 unitaires + 9 intégration), tous passants
