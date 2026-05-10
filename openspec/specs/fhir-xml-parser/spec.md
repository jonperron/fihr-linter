# fhir-xml-parser Specification

## Purpose
TBD - created by archiving change fhir-xml-parser. Update Purpose after archive.
## Requirements
### Requirement: Parse XML

Le système SHALL exposer une fonction `parse_xml(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne XML FHIR en `Resource`.

#### Scenario: Ressource valide avec champs primitifs

- **WHEN** `parse_xml` reçoit une ressource FHIR XML valide (ex. `<Patient><status value="active"/></Patient>`)
- **THEN** elle SHALL retourner `Ok(Resource)` avec `resource_type == "Patient"` et le champ `status` dont la valeur est `Value::Str("active")`

#### Scenario: XML malformé (EOF inattendu)

- **WHEN** `parse_xml` reçoit une chaîne XML où un élément n'est pas fermé
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec un message décrivant l'erreur et la position (ligne, colonne)

#### Scenario: Document sans élément racine

- **WHEN** `parse_xml` reçoit une chaîne vide ou sans élément racine
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec le message `"document contains no root element"`

---

### Requirement: Type de ressource depuis l'élément racine

Le système SHALL extraire le type de ressource depuis le nom local de l'élément racine XML.

#### Scenario: Élément racine standard

- **WHEN** l'élément racine est `<Patient>` (ou tout autre nom d'élément FHIR)
- **THEN** `Resource.resource_type` SHALL être égal au nom local de l'élément racine

---

### Requirement: Champs primitifs XML

Le système SHALL extraire les champs primitifs depuis l'attribut `value` des éléments enfants.

#### Scenario: Attribut value présent

- **WHEN** un élément enfant porte un attribut `value` (ex. `<status value="active"/>`)
- **THEN** le champ correspondant SHALL avoir la valeur `Value::Str("active")`

#### Scenario: Élément auto-fermant sans attribut value

- **WHEN** un élément enfant est auto-fermant sans attribut `value` (ex. `<deceased/>`)
- **THEN** le champ correspondant SHALL avoir la valeur `Value::Object({})` (objet vide)

---

### Requirement: Champs complexes XML

Le système SHALL représenter les éléments XML ayant des enfants comme `Value::Object`.

#### Scenario: Élément avec enfants

- **WHEN** un élément XML contient des éléments enfants (ex. `<name><family value="Smith"/></name>`)
- **THEN** le champ correspondant SHALL être `Value::Object` contenant les champs enfants

---

### Requirement: Éléments répétés (arrays)

Le système SHALL regrouper les éléments frères de même nom en `Value::Array`.

#### Scenario: Éléments frères identiques

- **WHEN** plusieurs éléments frères portent le même nom local (ex. plusieurs `<name>`)
- **THEN** le champ correspondant SHALL être `Value::Array` contenant un `Node` par élément

---

### Requirement: Contenu XHTML

Le système SHALL capturer le contenu textuel de l'élément `<div>` XHTML comme `Value::Str`.

#### Scenario: Élément div XHTML

- **WHEN** un élément `<div>` (namespace XHTML) est rencontré
- **THEN** le champ `div` SHALL être `Value::Str` contenant le texte brut du contenu, et les éléments enfants de `<div>` SHALL être ignorés structurellement

---

### Requirement: Primitive avec extensions

Le système SHALL représenter un élément portant à la fois un attribut `value` et des éléments enfants comme `Value::Object` avec une clé `"value"` en tête.

#### Scenario: Primitive avec extension enfant

- **WHEN** un élément XML porte un attribut `value` ET contient des éléments enfants (ex. `<birthDate value="1990-01-01"><extension url="..."/></birthDate>`)
- **THEN** le champ SHALL être `Value::Object` dont la première entrée est `"value" -> Value::Str("1990-01-01")`, suivie des enfants

---

### Requirement: Extraction du champ id

Le système SHALL extraire l'élément `<id>` de la ressource XML vers `Resource.id`.

#### Scenario: Champ id présent et primitif

- **WHEN** la ressource XML contient `<id value="patient-1"/>`
- **THEN** `Resource.id` SHALL être `Some("patient-1")` et `id` ne SHALL pas apparaître dans `Resource.fields`

#### Scenario: Champ id non-primitif

- **WHEN** la ressource XML contient un élément `<id>` dont la valeur n'est pas un `Value::Str`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)` avec un message indiquant que `id` doit être une valeur primitive de type chaîne

---

### Requirement: Positions sources XML

Le système SHALL associer une position source (`Span`) à chaque `Node` produit par le parser XML.

#### Scenario: Span d'un champ XML

- **WHEN** un élément XML est parsé
- **THEN** le `Span` du `Node` correspondant SHALL avoir `line` et `col` correspondant à la position de l'élément dans la source

