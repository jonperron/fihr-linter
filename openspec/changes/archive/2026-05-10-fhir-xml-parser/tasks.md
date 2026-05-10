## 1. Types

- [x] Ajouter la variante `XmlError { message: String, line: u32, col: u32 }` à `ParseError` dans `crates/fhir-parser/src/types.rs`

## 2. Implémentation XML

- [x] Créer `crates/fhir-parser/src/xml.rs` avec `parse_xml(source: &str) -> Result<Resource, ParseError>`
- [x] Extraire le type de ressource depuis le nom local de l'élément racine
- [x] Parser les champs primitifs depuis l'attribut `value` des éléments XML
- [x] Parser les champs complexes (éléments avec enfants) en `Value::Object`
- [x] Regrouper les éléments frères de même nom en `Value::Array`
- [x] Capturer le contenu XHTML `<div>` comme `Value::Str`
- [x] Représenter les primitives-avec-extensions comme `Value::Object { "value": ..., extensions... }`
- [x] Extraire `<id>` vers `Resource.id` et retourner `ParseError::XmlError` si non-primitif
- [x] Associer un `Span` à chaque `Node` produit

## 3. Exports

- [x] Exporter `parse_xml` et le module `xml` depuis `crates/fhir-parser/src/lib.rs`

## 4. Tests

- [x] Tests unitaires dans `xml.rs` couvrant : ressource valide, champs primitifs, objets complexes, tableaux, XHTML, primitives-avec-extensions, EOF inattendu, document sans racine, id non-primitif
