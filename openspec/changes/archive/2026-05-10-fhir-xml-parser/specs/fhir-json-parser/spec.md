## ADDED Requirements

### Requirement: Erreurs de parse XML

Le système SHALL exposer la variante `ParseError::XmlError { message: String, line: u32, col: u32 }` pour signaler toute erreur rencontrée lors du parse XML.

#### Scenario: XML invalide

- **WHEN** `parse_xml` rencontre du XML syntaxiquement invalide ou un document sans élément racine
- **THEN** elle SHALL retourner `Err(ParseError::XmlError)` contenant un `message` descriptif ainsi que les coordonnées `line` et `col` (1-indexées) pointant vers l'erreur dans la source
