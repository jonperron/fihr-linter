# Spec: fhir-bundle-loader

## Purpose

Chargement et parsing des bundles JSON FHIR R5 officiels vers les types du modèle.
## Requirements
### Requirement: Chargement d'un bundle FHIR JSON

Le système SHALL fournir une fonction `load_bundle(path, ...)` capable de lire et parser un fichier de bundle FHIR R5 au format JSON.

#### Scenario: Bundle valide avec StructureDefinitions

- **WHEN** `load_bundle` est appelé avec un chemin vers un fichier bundle FHIR JSON valide contenant des entrées `StructureDefinition`
- **THEN** les `StructureDefinition` SHALL être parsées et insérées dans la map fournie, indexées par leur URL canonique

#### Scenario: Bundle valide avec ValueSets et CodeSystems

- **WHEN** `load_bundle` est appelé avec un bundle contenant des `ValueSet` et `CodeSystem`
- **THEN** ils SHALL être parsés et insérés dans les maps correspondantes

---

### Requirement: Gestion des types de ressources inconnus

Le système SHALL ignorer silencieusement toute entrée de bundle dont le `resourceType` n'est pas `StructureDefinition`, `ValueSet` ou `CodeSystem`.

#### Scenario: Type inconnu ignoré

- **WHEN** un bundle contient une entrée `Observation` ou tout autre type non géré
- **THEN** cette entrée SHALL être ignorée sans erreur, et les autres entrées SHALL être traitées normalement

---

### Requirement: Robustesse face aux données manquantes

Le système SHALL ignorer les ressources dont les champs obligatoires sont absents plutôt que de lever une erreur fatale.

#### Scenario: StructureDefinition sans URL

- **WHEN** une `StructureDefinition` dans un bundle n'a pas de champ `url`
- **THEN** cette entrée SHALL être ignorée silencieusement

#### Scenario: ElementDefinition sans path

- **WHEN** un élément dans le snapshot d'une `StructureDefinition` n'a pas de champ `path`
- **THEN** cet élément SHALL être ignoré, et les autres éléments SHALL être traités normalement

---

### Requirement: Erreurs typées

Le système SHALL retourner des erreurs typées et informatives en cas d'échec de chargement.

#### Scenario: Fichier inexistant retourne Error Io

- **WHEN** `load_bundle` est appelé avec un chemin vers un fichier inexistant
- **THEN** il SHALL retourner `Error::Io`

#### Scenario: JSON invalide retourne Error Json

- **WHEN** le fichier existe mais contient du JSON invalide
- **THEN** `load_bundle` SHALL retourner `Error::Json` incluant le nom du fichier

#### Scenario: Bundle sans tableau entry retourne Error MalformedBundle

- **WHEN** le JSON est valide mais ne contient pas de champ `entry` de type tableau
- **THEN** `load_bundle` SHALL retourner `Error::MalformedBundle` incluant le nom du fichier et un message explicatif

---

### Requirement: Fichiers de définitions FHIR R5 supportés

Le système SHALL supporter le chargement des bundles FHIR R5 officiels suivants : `profiles-resources.json`, `profiles-types.json`, `valuesets.json`.

#### Scenario: Chargement des profils officiels

- **WHEN** les trois fichiers FHIR R5 officiels sont chargés
- **THEN** le résultat SHALL contenir des StructureDefinitions, ValueSets et CodeSystems issus des données officielles HL7

