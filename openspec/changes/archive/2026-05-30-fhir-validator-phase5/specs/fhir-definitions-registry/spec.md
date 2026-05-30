## ADDED Requirements

### Requirement: Type Constraint sur ElementDefinition

Le système SHALL fournir un type `Constraint` public avec les champs : `key: Arc<str>`, `severity: Arc<str>` (valeur `"error"` ou `"warning"`), `human: Arc<str>`, `expression: Option<Arc<str>>`. Chaque `ElementDefinition` SHALL exposer un champ `constraints: Vec<Constraint>`.

#### Scenario: Contraintes chargées depuis le snapshot

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5 et contient des éléments avec des contraintes (`constraint` array dans le JSON)
- **THEN** `element.constraints` SHALL contenir les `Constraint` correspondants avec `key`, `severity`, `human` et `expression` renseignés

#### Scenario: Contrainte sans expression

- **WHEN** un élément de contrainte JSON ne possède pas de champ `expression`
- **THEN** `Constraint.expression` SHALL être `None`

#### Scenario: Élément sans contraintes produit un Vec vide

- **WHEN** un élément du snapshot ne déclare pas de contraintes
- **THEN** `element.constraints` SHALL être un `Vec` vide

---

### Requirement: Type Binding sur ElementDefinition

Le système SHALL fournir un type `Binding` public avec les champs : `strength: Arc<str>`, `value_set: Option<Arc<str>>`. Chaque `ElementDefinition` SHALL exposer un champ `binding: Option<Binding>`.

#### Scenario: Binding chargé depuis le snapshot

- **WHEN** un élément du snapshot déclare un binding (`binding` dans le JSON)
- **THEN** `element.binding` SHALL être `Some(Binding)` avec `strength` et `value_set` renseignés

#### Scenario: Binding sans valueSet

- **WHEN** un binding JSON ne possède pas de champ `valueSet`
- **THEN** `Binding.value_set` SHALL être `None`

#### Scenario: Élément sans binding

- **WHEN** un élément du snapshot ne déclare pas de binding
- **THEN** `element.binding` SHALL être `None`

---

### Requirement: Export public de Constraint et Binding

Le crate `fhir-definitions` SHALL exporter `Constraint` et `Binding` dans son API publique de premier niveau.

#### Scenario: Importation directe depuis fhir-definitions

- **WHEN** un crate dépendant importe `use fhir_definitions::{Binding, Constraint}`
- **THEN** la compilation SHALL réussir sans erreur
