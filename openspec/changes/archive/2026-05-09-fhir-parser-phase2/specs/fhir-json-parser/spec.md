## ADDED Requirements

### Requirement: Type Resource

Le système SHALL exposer un type `Resource` représentant une ressource FHIR parsée depuis JSON.

#### Scenario: Champs obligatoires

- **WHEN** une ressource FHIR JSON est parsée avec succès
- **THEN** le `Resource` retourné SHALL contenir `resource_type: Arc<str>` (valeur du champ `"resourceType"`), `id: Option<Arc<str>>` (valeur du champ `"id"` si présent), et `fields: IndexMap<Arc<str>, Node>` (tous les champs restants)

---

### Requirement: Type Node

Le système SHALL exposer un type `Node` associant une valeur FHIR à une position source.

#### Scenario: Composition d'un Node

- **WHEN** un champ JSON est parsé
- **THEN** le `Node` résultant SHALL contenir `value: Value` (la valeur) et `span: Span` (la position dans la source)

---

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les types JSON possibles dans une ressource FHIR.

#### Scenario: Variantes couvertes

- **WHEN** un nœud JSON est parsé
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Decimal(f64)`, `Str(Arc<str>)`, `Array(Vec<Node>)`, `Object(IndexMap<Arc<str>, Node>)`

---

### Requirement: Type Span

Le système SHALL exposer un type `Span` décrivant la position d'un nœud dans la source texte.

#### Scenario: Champs d'un Span

- **WHEN** un nœud est localisé dans la source
- **THEN** le `Span` SHALL contenir `line: u32` (ligne 1-indexée), `col: u32` (colonne 1-indexée), `offset: u32` (offset byte depuis le début du fichier)

---

### Requirement: Parse JSON

Le système SHALL exposer une fonction `parse_json(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne JSON en `Resource`.

#### Scenario: Ressource valide

- **WHEN** `parse_json` reçoit une ressource FHIR JSON valide avec un champ `"resourceType"`
- **THEN** elle SHALL retourner `Ok(Resource)` avec `resource_type` égal à la valeur du champ `"resourceType"`

#### Scenario: JSON malformé

- **WHEN** `parse_json` reçoit une chaîne JSON syntaxiquement invalide
- **THEN** elle SHALL retourner `Err(ParseError)` contenant un message et une position (ligne, colonne)

#### Scenario: resourceType manquant

- **WHEN** `parse_json` reçoit un objet JSON valide sans champ `"resourceType"`
- **THEN** elle SHALL retourner `Err(ParseError::MissingResourceType)`

---

### Requirement: Positions sources précises

Le système SHALL associer une position source à chaque `Node` lors du parse JSON.

#### Scenario: Ligne correcte pour un champ

- **WHEN** un champ est présent à la ligne N de la source JSON
- **THEN** le `Span` du `Node` correspondant SHALL avoir `line == N`
