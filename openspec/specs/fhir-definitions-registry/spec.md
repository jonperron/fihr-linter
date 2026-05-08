# Spec: fhir-definitions-registry

## Purpose

Registry FHIR R5 indexé par URL canonique, thread-safe, pour la résolution de définitions.
## Requirements
### Requirement: Construction depuis un répertoire de définitions

Le système SHALL fournir `Registry::from_definitions_dir(dir)` pour construire un registry en chargeant les bundles FHIR R5 officiels depuis un répertoire.

#### Scenario: Chargement réussi depuis le répertoire definitions r5

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin vers un répertoire contenant les fichiers FHIR R5 (`profiles-resources.json`, `profiles-types.json`, `valuesets.json`)
- **THEN** le registry SHALL être construit avec succès et contenir des StructureDefinitions, ValueSets et CodeSystems

#### Scenario: Répertoire inexistant retourne une erreur

- **WHEN** `Registry::from_definitions_dir` est appelé avec un chemin inexistant
- **THEN** il SHALL retourner une erreur `Error::Io`

---

### Requirement: Résolution de StructureDefinition par URL canonique

Le système SHALL permettre de résoudre une `StructureDefinition` par son URL canonique exacte.

#### Scenario: URL connue retourne Some StructureDefinition

- **WHEN** `registry.structure_definition(url)` est appelé avec l'URL canonique d'une définition chargée
- **THEN** il SHALL retourner `Some(&StructureDefinition)`

#### Scenario: URL inconnue retourne None

- **WHEN** `registry.structure_definition(url)` est appelé avec une URL non présente dans le registry
- **THEN** il SHALL retourner `None`

#### Scenario: Résolution sensible à la casse

- **WHEN** une URL est cherchée avec une casse différente de l'URL canonique
- **THEN** `structure_definition` SHALL retourner `None`

---

### Requirement: Résolution de ValueSet par URL canonique

Le système SHALL permettre de résoudre un `ValueSet` par son URL canonique exacte.

#### Scenario: URL connue retourne Some ValueSet

- **WHEN** `registry.value_set(url)` est appelé avec l'URL canonique d'un ValueSet chargé
- **THEN** il SHALL retourner `Some(&ValueSet)` avec les champs `url` et `name` correctement renseignés

#### Scenario: URL inconnue retourne None pour ValueSet

- **WHEN** `registry.value_set(url)` est appelé avec une URL non présente
- **THEN** il SHALL retourner `None`

---

### Requirement: Résolution de CodeSystem par URL canonique

Le système SHALL permettre de résoudre un `CodeSystem` par son URL canonique exacte.

#### Scenario: URL connue retourne Some CodeSystem

- **WHEN** `registry.code_system(url)` est appelé avec l'URL canonique d'un CodeSystem chargé
- **THEN** il SHALL retourner `Some(&CodeSystem)` avec les champs `url` et `name` correctement renseignés

#### Scenario: URL inconnue retourne None pour CodeSystem

- **WHEN** `registry.code_system(url)` est appelé avec une URL non présente
- **THEN** il SHALL retourner `None`

---

### Requirement: Compteurs de ressources indexées

Le système SHALL exposer des méthodes retournant le nombre d'entrées indexées.

#### Scenario: Compteurs positifs après chargement

- **WHEN** le registry est chargé depuis les définitions FHIR R5 officielles
- **THEN** `structure_definition_count()`, `value_set_count()` et `code_system_count()` SHALL retourner des valeurs positives

---

### Requirement: Thread-safety via Arc

Le `Registry` SHALL être partageable entre threads via `Arc<Registry>` sans nécessiter de synchronisation externe après construction.

#### Scenario: Accès concurrent en lecture

- **WHEN** le registry est partagé via `Arc` entre plusieurs threads
- **THEN** chaque thread SHALL pouvoir appeler les méthodes de résolution de manière sûre et concurrente

---

### Requirement: Données snapshot de StructureDefinition

Le système SHALL charger les éléments du snapshot d'une `StructureDefinition` avec leurs cardinalités et types.

#### Scenario: Snapshot contient les éléments avec path et types

- **WHEN** une `StructureDefinition` est chargée depuis un bundle FHIR R5
- **THEN** son champ `snapshot` SHALL contenir des `ElementDefinition` avec `path`, `min`, `max`, et `types` renseignés selon les données JSON source

#### Scenario: base_definition renseignée pour les ressources dérivées

- **WHEN** une `StructureDefinition` a un champ `baseDefinition` dans le JSON source
- **THEN** `base_definition` SHALL contenir l'URL de la définition parente

