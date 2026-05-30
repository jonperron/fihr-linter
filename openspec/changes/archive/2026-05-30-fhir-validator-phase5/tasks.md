# Tasks: fhir-validator-phase5

## 1. Extension du modèle fhir-definitions

- [x] Ajouter le type `Constraint` à `crates/fhir-definitions/src/model.rs`
- [x] Ajouter le type `Binding` à `crates/fhir-definitions/src/model.rs`
- [x] Ajouter les champs `constraints: Vec<Constraint>` et `binding: Option<Binding>` à `ElementDefinition`
- [x] Mettre à jour `crates/fhir-definitions/src/loader.rs` pour parser les contraintes et bindings depuis les éléments snapshot
- [x] Exporter `Binding` et `Constraint` depuis `crates/fhir-definitions/src/lib.rs`

## 2. Types Diagnostic et Severity

- [x] Créer `crates/fhir-validator/src/diagnostic.rs` avec `Severity` (Error, Warning, Information, Hint)
- [x] Implémenter `Diagnostic` avec les champs severity, code, message, path, location, rule
- [x] Implémenter les constructeurs `Diagnostic::error()` et `Diagnostic::warning()`
- [x] Implémenter les builders `.with_location()` et `.with_rule()`

## 3. Couche structurelle

- [x] Créer `crates/fhir-validator/src/structural.rs`
- [x] Implémenter la détection du type de ressource inconnu (STRUCTURE_000) dans `lib.rs`
- [x] Implémenter la détection des éléments inconnus (STRUCTURE_001) avec court-circuit sur extensions et champs `_`
- [x] Gérer les champs polymorphiques `value[x]` via la liste `TYPE_SUFFIXES`

## 4. Couche cardinalité

- [x] Créer `crates/fhir-validator/src/cardinality.rs`
- [x] Implémenter CARDINALITY_001 (élément requis absent, min > 0)
- [x] Implémenter CARDINALITY_002 (dépassement du maximum, max fini)
- [x] Ignorer les placeholders `[x]` polymorphiques

## 5. Couche type

- [x] Créer `crates/fhir-validator/src/type_check.rs`
- [x] Implémenter TYPE_001 (type JSON incorrect pour boolean, integer, decimal, string et dérivés)
- [x] Implémenter TYPE_002 (valeur entière hors contrainte pour unsignedInt et positiveInt)
- [x] Implémenter TYPE_003 (format invalide pour date, dateTime, instant, time)
- [x] Implémenter TYPE_004 (whitespace dans un code)
- [x] Implémenter TYPE_005 (format invalide pour id)
- [x] Gérer la récursion dans les tableaux JSON

## 6. Couche terminologie (stub)

- [x] Créer `crates/fhir-validator/src/terminology.rs` avec une fonction stub sans-op

## 7. Couche invariants

- [x] Créer `crates/fhir-validator/src/invariants.rs`
- [x] Implémenter l'évaluation des expressions FHIRPath depuis l'élément racine du snapshot
- [x] Émettre INVARIANT_<key> pour chaque contrainte non satisfaite
- [x] Ignorer silencieusement les erreurs d'évaluation FHIRPath

## 8. API publique et orchestration

- [x] Créer `crates/fhir-validator/src/lib.rs` avec la fonction publique `validate()`
- [x] Orchestrer les couches dans l'ordre : structural → cardinality → type_check → terminology → invariants
- [x] Court-circuiter sur STRUCTURE_000 (type de ressource inconnu)

## 9. Tests

- [x] Écrire les tests unitaires dans chaque module (35 tests au total)
- [x] Créer `crates/fhir-validator/tests/validator_tests.rs` avec 9 tests d'intégration contre les définitions R5 bundlées
- [x] Vérifier que tous les tests passent (`cargo test`)
