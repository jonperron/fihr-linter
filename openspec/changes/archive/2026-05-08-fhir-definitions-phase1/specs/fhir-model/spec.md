# Delta Spec: fhir-model

## ADDED Requirements

### Requirement: StructureDefinition type

Le système SHALL exposer un type `StructureDefinition` représentant une définition de profil FHIR R5.

#### Scenario: Champs obligatoires présents

- **WHEN** une `StructureDefinition` est construite
- **THEN** elle SHALL contenir les champs : `url` (URL canonique), `name` (nom logique), `kind` (catégorie : `primitive-type`, `complex-type`, `resource`, `logical`), `is_abstract` (booléen), `base_definition` (URL optionnelle), `snapshot` (liste d'`ElementDefinition`)

---

### Requirement: ElementDefinition type

Le système SHALL exposer un type `ElementDefinition` représentant un élément dans le snapshot d'une `StructureDefinition`.

#### Scenario: Champs d'une ElementDefinition

- **WHEN** une `ElementDefinition` est construite
- **THEN** elle SHALL contenir : `path` (chemin FHIRPath), `min` (cardinalité minimale, entier non négatif), `max` (cardinalité maximale : `"*"` ou entier représenté en string), `types` (liste des codes de type FHIR)

---

### Requirement: ValueSet type

Le système SHALL exposer un type `ValueSet` représentant un ensemble de valeurs FHIR R5.

#### Scenario: Champs d'un ValueSet

- **WHEN** un `ValueSet` est construit
- **THEN** il SHALL contenir : `url` (URL canonique), `name` (nom logique)

---

### Requirement: CodeSystem type

Le système SHALL exposer un type `CodeSystem` représentant un système de codes FHIR R5.

#### Scenario: Champs d'un CodeSystem

- **WHEN** un `CodeSystem` est construit
- **THEN** il SHALL contenir : `url` (URL canonique), `name` (nom logique)

---

### Requirement: Partage efficace via Arc

Les champs de type chaîne de caractères SHALL être représentés par `Arc<str>` pour permettre le clonage à coût zéro et le partage entre threads.

#### Scenario: Clone sans allocation

- **WHEN** un type du modèle est cloné
- **THEN** le clone SHALL partager la mémoire des champs `Arc<str>` sans nouvelle allocation de chaîne
