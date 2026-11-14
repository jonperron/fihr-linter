# fhir-xml-parser Specification

## Purpose
TBD - created by archiving change fhir-xml-parser. Update Purpose after archive.
## Requirements
### Requirement: Parse XML

Le système SHALL exposer une fonction `parse_xml(source: &str) -> Result<Resource, ParseError>` qui convertit une chaîne XML FHIR en `Resource`.

Un caractère littéral inférieur à U+0020 autre que tabulation, LF et CR SHALL rendre le document non bien formé (`Err(ParseError::XmlError)`), où qu'il se trouve, en XML 1.0 comme en XML 1.1. Dans un document XML 1.0 (version déclarée ou implicite), un tel caractère obtenu par référence de caractère (ex. `&#1;`) dans une valeur capturée ou dans le texte brut du div narratif (ex. `&#3;`, `&#x1;`) SHALL également rendre le document non bien formé ; en XML 1.1 ces références SHALL être acceptées (le validateur émet alors `TYPE_006` pour les valeurs). Limite connue : les références de caractère dans les attributs XHTML du div et de l'élément racine ne sont pas vérifiées.

#### Scenario: Ressource valide avec champs primitifs

- **WHEN** `parse_xml` reçoit une ressource FHIR XML valide (ex. `<Patient><status value="active"/></Patient>`)
- **THEN** elle SHALL retourner `Ok(Resource)` avec `resource_type == "Patient"` et le champ `status` dont la valeur est `Value::Str("active")`

#### Scenario: XML malformé (EOF inattendu)

- **WHEN** `parse_xml` reçoit une chaîne XML où un élément n'est pas fermé
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec un message décrivant l'erreur et la position (ligne, colonne)

#### Scenario: Document sans élément racine

- **WHEN** `parse_xml` reçoit une chaîne vide ou sans élément racine
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` avec le message `"document contains no root element"`

#### Scenario: Caractère de contrôle littéral

- **WHEN** `parse_xml` reçoit un document (XML 1.0 ou 1.1) contenant un caractère U+0001 littéral, dans une valeur, un commentaire ou ailleurs
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` positionnée sur ce caractère

#### Scenario: Référence de caractère de contrôle en XML 1.0

- **WHEN** `parse_xml` reçoit un document sans déclaration ou déclaré `version="1.0"` dont une valeur capturée contient `&#1;`
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Référence de caractère de contrôle dans le div en XML 1.0

- **WHEN** `parse_xml` reçoit un document XML 1.0 dont le texte du div narratif contient `&#3;` ou `&#x1;`
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)`, alors que le même div sous une déclaration `<?xml version="1.1"?>` SHALL être accepté

#### Scenario: Référence de caractère de contrôle en XML 1.1

- **WHEN** `parse_xml` reçoit un document `<?xml version="1.1"?>` dont une valeur contient `&#1;`
- **THEN** elle SHALL retourner `Ok(Resource)` et la valeur SHALL contenir U+0001

#### Scenario: Fixture officielle unicode-problem.xml

- **WHEN** la fixture `unicode-problem.xml` est parsée
- **THEN** le parse SHALL échouer (erreur fatale)

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

Le système SHALL capturer l'élément `<div>` XHTML comme `Value::Str` contenant sa tranche de source brute, du `<div` ouvrant au `</div>` fermant inclus (ou l'élément `<div .../>` auto-fermant), sans décodage.

#### Scenario: Élément div XHTML

- **WHEN** un élément `<div>` (namespace XHTML) est rencontré
- **THEN** le champ `div` SHALL être `Value::Str` contenant le balisage source `<div ...>...</div>` tel qu'écrit, et les éléments enfants de `<div>` SHALL être ignorés structurellement

#### Scenario: Div auto-fermant

- **WHEN** le div est écrit `<div xmlns="http://www.w3.org/1999/xhtml"/>`
- **THEN** le champ `div` SHALL être `Value::Str` contenant exactement cette tranche de source

### Requirement: Primitive avec extensions

Le système SHALL représenter un élément portant à la fois un attribut `value` et des éléments enfants comme `Value::Object` avec une clé `"value"` en tête.

#### Scenario: Primitive avec extension enfant

- **WHEN** un élément XML porte un attribut `value` ET contient des éléments enfants (ex. `<birthDate value="1990-01-01"><extension url="..."/></birthDate>`)
- **THEN** le champ SHALL être `Value::Object` dont la première entrée est `"value" -> Value::Str("1990-01-01")`, suivie des enfants

---

### Requirement: Extraction du champ id

Le système SHALL extraire l'attribut `value` de l'élément racine `<id>` de la ressource XML vers `Resource.id`. Lorsque `<id>` porte aussi une `<extension>` ou un attribut `id`, le système SHALL conserver le reste dans `Resource.fields` sous la clé `id`, comme `Value::Object` sans clé `value`, à la même position et avec le span de l'élément `id`, afin que le reshape XML du validateur le convertisse en `_id` comme le fait JSON. Limite connue : `Resource` n'a pas de span pour l'id remonté, donc un id dont les espaces de bordure ont été retirés ne déclenche pas encore TYPE_007.

#### Scenario: Champ id présent et primitif

- **WHEN** la ressource XML contient `<id value="patient-1"/>`
- **THEN** `Resource.id` SHALL être `Some("patient-1")` et `id` ne SHALL pas apparaître dans `Resource.fields`

#### Scenario: Champ id avec extension ou attribut id

- **WHEN** la ressource XML contient `<id value="p1">` avec un élément enfant `<extension>` ou un attribut `id`
- **THEN** `Resource.id` SHALL être `Some("p1")` et `Resource.fields` SHALL conserver `id` comme `Value::Object` sans clé `value`, à la même position, avec le span de l'élément `id`

#### Scenario: Champ id sans attribut value

- **WHEN** la ressource XML contient `<id id="e1"/>`
- **THEN** `Resource.id` SHALL être `None` et l'objet `id` SHALL être conservé dans `Resource.fields`

#### Scenario: Élément enfant value non remonté

- **WHEN** l'élément `<id>` contient un élément enfant `<value>` (et non l'attribut `value`)
- **THEN** cet enfant SHALL rester dans l'objet `id` sans être remonté dans `Resource.id`, afin que le validateur signale STRUCTURE_001

#### Scenario: Champ id vide ou répété

- **WHEN** la ressource XML contient un élément `<id/>` vide ou plusieurs éléments `<id>`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)` avec le message `'id' element must be a primitive string value`

---

### Requirement: Profondeur maximale

Le parser XML SHALL rejeter un document dont les éléments sont imbriqués sur plus de 128 niveaux, y compris à l'intérieur du div narratif.

#### Scenario: Imbrication trop profonde

- **WHEN** `parse_xml` reçoit des éléments imbriqués sur plus de 128 niveaux
- **THEN** il SHALL retourner `Err(ParseError::XmlError)` sans débordement de pile

#### Scenario: Imbrication trop profonde dans le div

- **WHEN** le div narratif contient des éléments XHTML imbriqués au-delà de 128 niveaux
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

### Requirement: Positions sources XML

Le système SHALL associer une position source (`Span`) à chaque `Node` produit par le parser XML.

#### Scenario: Span d'un champ XML

- **WHEN** un élément XML est parsé
- **THEN** le `Span` du `Node` correspondant SHALL avoir `line` et `col` correspondant à la position de l'élément dans la source

### Requirement: Conformité XML FHIR

`parse_xml` SHALL retourner `Err(ParseError::XmlError)` pour tout document que la représentation XML FHIR (xml.html) interdit :

- une DTD (`<!DOCTYPE ...>`) ;
- une déclaration XML dont l'`encoding` n'est pas UTF-8 (comparaison insensible à la casse) ;
- du texte non blanc, une référence d'entité ou une section CDATA entre éléments FHIR (les commentaires et instructions de traitement SHALL être ignorés) ;
- tout attribut de la racine de la ressource autre qu'une déclaration de namespace ou un attribut lié au namespace XMLSchema-instance, le message SHALL nommer tous les attributs fautifs ;
- un attribut et un élément enfant de même nom sur un même élément ;
- un élément hors du namespace `http://hl7.org/fhir` (un préfixe lié à ce namespace SHALL être accepté) ;
- le div narratif, ou tout élément qu'il contient, hors du namespace XHTML `http://www.w3.org/1999/xhtml` ;
- un attribut dans un namespace sur un élément FHIR, hors namespace XMLSchema-instance ;
- plus de 8 déclarations de namespace sur un même élément.

Le namespace XMLSchema-instance n'est pas une erreur de parse : il est enregistré dans `Resource::schema_instance` (voir « Namespace schema instance XML »).

#### Scenario: DTD refusée

- **WHEN** `parse_xml` reçoit un document contenant `<!DOCTYPE Patient>`
- **THEN** il SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Encodage non UTF-8

- **WHEN** la déclaration XML porte `encoding="ISO-8859-1"`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors que `encoding="utf-8"` SHALL être accepté

#### Scenario: Texte entre éléments

- **WHEN** du texte non blanc, une référence d'entité ou une section CDATA apparaît entre deux éléments FHIR
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors qu'un commentaire ou une instruction de traitement au même endroit SHALL être ignoré

#### Scenario: Attribut sur la racine

- **WHEN** l'élément racine porte `id="a"` et `lang="en"` en plus de `xmlns`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)` dont le message nomme `id` et `lang`

#### Scenario: Attribut et élément de même nom

- **WHEN** un élément porte un attribut `value` et un élément enfant `<value>`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Namespace FHIR

- **WHEN** un élément FHIR n'est pas dans `http://hl7.org/fhir`
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`, alors que `<f:Patient xmlns:f="http://hl7.org/fhir">` SHALL être accepté

#### Scenario: Namespace XHTML du div

- **WHEN** le div narratif ou un élément qu'il contient n'est pas dans le namespace XHTML
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Attribut dans un namespace

- **WHEN** un élément FHIR porte un attribut préfixé lié à un namespace autre que XMLSchema-instance (ex. `x:value`)
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

#### Scenario: Namespace XMLSchema-instance

- **WHEN** un élément déclare le namespace XMLSchema-instance ou porte `xsi:schemaLocation` / `xsi:type`
- **THEN** `parse_xml` SHALL retourner `Ok(Resource)` et l'élément SHALL figurer dans `schema_instance`

#### Scenario: Trop de déclarations de namespace

- **WHEN** un élément porte 9 déclarations de namespace
- **THEN** `parse_xml` SHALL retourner `Err(ParseError::XmlError)`

### Requirement: Épuration des valeurs d'attribut

`parse_xml` SHALL retirer des valeurs d'attribut les blancs XML (espace, tabulation, CR, LF) en tête et en fin. Chaque attribut ainsi modifié SHALL être enregistré dans `Resource::trimmed_attributes` sous la forme d'un `TrimmedAttribute` (`Resource::trimmed_attributes` est un `Vec<TrimmedAttribute>`) : `TrimmedAttribute::Element(offset, name)` (position en octets de l'élément, nom de l'attribut) pour un attribut d'élément, et `TrimmedAttribute::ResourceId(span)` pour la valeur épurée du `<id>` racine, remontée dans `Resource::id`, portant le span de l'élément `<id>`. Exception : une valeur composée uniquement de blancs SHALL être conservée telle quelle (ni réduite à `""`, ni enregistrée dans `trimmed_attributes`, donc sans `TYPE_007`). Pour une ressource issue du JSON, `trimmed_attributes` SHALL être vide.

#### Scenario: Valeur épurée

- **WHEN** `parse_xml` reçoit `<active value=" true "/>`
- **THEN** `active` SHALL valoir `Value::Str("true")` et `trimmed_attributes` SHALL contenir la position de `<active>` avec le nom `value`

#### Scenario: id racine épuré

- **WHEN** `parse_xml` reçoit un `Patient` avec `<id value=" p1 "/>`
- **THEN** `Resource::id` SHALL valoir `p1` et `trimmed_attributes` SHALL contenir `TrimmedAttribute::ResourceId` avec le span de l'élément `<id>`

#### Scenario: Valeur composée uniquement de blancs

- **WHEN** `parse_xml` reçoit `<valueString value="   "/>`
- **THEN** la valeur SHALL rester `"   "` et `trimmed_attributes` SHALL être vide

#### Scenario: Valeur sans blanc

- **WHEN** aucune valeur d'attribut n'a de blanc en tête ou en fin
- **THEN** `trimmed_attributes` SHALL être vide

#### Scenario: Ressource JSON

- **WHEN** une ressource est produite par `parse_json`
- **THEN** `trimmed_attributes` SHALL être vide

### Requirement: Namespace schema instance XML

`parse_xml` SHALL NOT échouer sur le namespace XMLSchema-instance. Chaque élément qui déclare un namespace dont la valeur normalisée est `http://www.w3.org/2001/XMLSchema-instance`, ou qui porte un attribut lié à ce namespace (ex. `xsi:schemaLocation`, `xsi:type`), SHALL être enregistré une seule fois, sous la forme du `Span` de l'élément, dans `Resource::schema_instance: Vec<Span>`. Ces attributs ne portent aucune donnée et SHALL NOT produire d'erreur d'attribut de la racine ni d'attribut dans un namespace. Pour une ressource issue du JSON, `schema_instance` SHALL être vide. Le validateur signale chaque élément enregistré par `STRUCTURE_003`.

#### Scenario: Déclaration et schemaLocation sur la racine

- **WHEN** `parse_xml` reçoit `<Patient xmlns="http://hl7.org/fhir" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="...">`
- **THEN** il SHALL retourner `Ok(Resource)` et `schema_instance` SHALL contenir exactement un `Span`, celui de l'élément racine

#### Scenario: Forme non normalisée

- **WHEN** un élément déclare le namespace XMLSchema-instance sous une forme non normalisée de la même valeur
- **THEN** l'élément SHALL être enregistré dans `schema_instance`

#### Scenario: Ressource JSON

- **WHEN** une ressource est produite par `parse_json`
- **THEN** `schema_instance` SHALL être vide

