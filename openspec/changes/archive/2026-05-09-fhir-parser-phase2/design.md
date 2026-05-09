## Context

Le crate `fhir-parser` doit transformer des ressources FHIR JSON en un AST Rust typé. Ce parseur est le point d'entrée de toute la chaîne de validation ; il doit donc être rapide, correct et fournir des positions sources précises.

L'existant : `crates/fhir-parser/src/lib.rs` contient uniquement un commentaire de module. Aucune structure, aucune dépendance.

## Goals / Non-Goals

**Goals:**
- Exposer les types `Resource`, `Node`, `Value`, `Span`, `ParseError` dans la racine du crate.
- Implémenter `parse_json(source: &str) -> Result<Resource, ParseError>` via `serde_json`.
- Préserver la position source (ligne/colonne/offset) pour chaque `Node`.
- Gérer les erreurs de JSON malformé avec un message et une position.

**Non-Goals:**
- Parseur XML (phase 3).
- Validation FHIR (phases 5+).
- Support des profiles ou extensions spécifiques.

## Decisions

### Représentation de la valeur : `serde_json::Value` réutilisée ou type propre ?

**Décision** : type `Value` propre avec des variantes miroir de `serde_json::Value` + champ `Span` dans `Node`.

**Rationale** : séparer le type de valeur de la bibliothèque tierce permet de l'étendre (e.g., ajouter `Decimal` FHIR) et évite une dépendance publique sur `serde_json`.

### Map de champs : `HashMap` vs `IndexMap` ?

**Décision** : `IndexMap<Arc<str>, Node>` (crate `indexmap`).

**Rationale** : FHIR JSON préserve l'ordre des champs dans les arrays et les objets ; `IndexMap` garantit l'ordre d'insertion, ce qui est utile pour les messages d'erreur reproductibles.

### Extraction des positions sources avec `serde_json`

`serde_json` ne fournit pas de spans nativement. L'approche retenue est une passe de scan indépendante : après le parse `serde_json::Value`, on re-parcourt la source `&str` avec `serde_json::StreamDeserializer` en mode `RawValue` pour mapper offset → (ligne, colonne).

Alternative rejetée : parser manuellement le JSON pour capturer les spans — trop coûteux à maintenir.

## Risks / Trade-offs

- **Positions approchées** : la méthode de reconstruction de spans via scan peut être légèrement décalée pour les objets imbriqués profonds. Mitigation : tests unitaires sur chaque type de nœud.
- **Overhead mémoire** : chaque `Node` porte un `Span` (4 × u32 = 16 octets). Acceptable pour des ressources FHIR de taille raisonnable.
