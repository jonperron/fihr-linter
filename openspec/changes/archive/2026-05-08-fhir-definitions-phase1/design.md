## Context

Le workspace `fhir-linter` nécessite un accès aux définitions FHIR R5 officielles pour valider des ressources FHIR. Le crate `fhir-definitions` est le premier crate du workspace ; tous les autres (`fhir-validator`, `fhir-linter`, bindings) en dépendent. Le jeu de données source est constitué de bundles JSON FHIR R5 bundlés dans `definitions/r5/` (téléchargés et vérifiés par `scripts/download-definitions.sh`).

## Goals / Non-Goals

**Goals**
- Charger l'ensemble des StructureDefinitions, ValueSets et CodeSystems FHIR R5 en mémoire au démarrage
- Exposer une API de résolution par URL canonique, read-only, sans verrou à l'usage
- Modèle de données minimal : seulement les champs nécessaires au validateur

**Non-Goals**
- Validation de conformité des définitions elles-mêmes
- Support des formats XML ou Turtle
- Chargement à chaud ou reload des définitions
- Chargement de profils utilisateur (IGs) — prévu dans une phase ultérieure

## Decisions

**1. Modèle de données allégé (pas de désérialisation Serde complète)**
Décision : utiliser `serde_json::Value` pour le parsing intermédiaire et construire des structs légères à la main.
Rationale : les bundles FHIR R5 sont complexes (~60 champs par StructureDefinition). Implémenter un `Deserialize` complet serait coûteux en maintenance. On ne matérialise que les champs utiles au validateur.
Alternative écartée : `serde` `Deserialize` complet → trop de champs inutiles, code généré volumineux.

**2. `Arc<str>` pour les champs chaîne**
Décision : tous les champs de type chaîne utilisent `Arc<str>`.
Rationale : les URLs et noms FHIR sont souvent partagés entre StructureDefinition et ses éléments. `Arc<str>` permet le clone à coût zéro et le partage entre threads sans copie.

**3. Registry immutable après construction (`Registry` sans `RwLock`)**
Décision : le `Registry` est construit une fois, puis partagé via `Arc<Registry>` en lecture seule. Pas de `RwLock` interne.
Rationale : les définitions FHIR R5 ne changent pas au runtime. Un `RwLock` ajouterait de la latence pour aucun bénéfice.
Alternative écartée : `Arc<RwLock<Registry>>` → overhead inutile pour un accès exclusivement en lecture.

**4. Chargement séquentiel au démarrage**
Décision : `from_definitions_dir` charge les trois fichiers séquentiellement.
Rationale : les fichiers sont lus une seule fois au démarrage. La parallélisation n'apporte pas de gain significatif pour un coût de complexité élevé.

## Risks / Trade-offs

- [Risk] Le modèle allégé ne contient pas tous les champs FHIR → si un futur crate a besoin d'un champ absent, il faudra étendre le modèle. Mitigation : le modèle est facilement extensible ; ajouter des champs est non-breaking.
- [Risk] La mémoire consommée par les trois bundles FHIR R5 peut être significative (~50-100 MB). Mitigation : acceptable pour un outil de validation ; les `Arc<str>` limitent la duplication.

## Open Questions

- Faut-il charger `profiles-others.json` dans une phase ultérieure (extensions, logical models) ?
- Les IGs utilisateur seront-ils chargés dans ce crate ou dans un crate dédié ?
