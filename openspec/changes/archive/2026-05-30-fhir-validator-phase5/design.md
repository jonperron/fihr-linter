## Context

Le pipeline de validation FHIR R5 nécessite plusieurs couches indépendantes, chacune inspectant un aspect distinct de la ressource : conformité structurelle, cardinalités, types primitifs, terminologie et invariants FHIRPath. Le modèle de `fhir-definitions` doit être étendu pour transporter les métadonnées nécessaires à ces couches (contraintes FHIRPath, bindings terminologiques).

## Goals

- Fournir un point d'entrée unique `validate()` qui orchestre toutes les couches dans l'ordre
- Chaque couche est indépendante et mutable (diagnostics accumulés dans un `Vec` partagé)
- Les types `Diagnostic` et `Severity` sont le seul type de sortie à travers tout le workspace
- Le modèle `fhir-definitions` ne dépend d'aucun autre crate de validation

## Non-Goals

- Validation de profilage (profils dérivés) — hors scope Phase 5
- Expansion des ValueSets et validation terminologique complète — stub uniquement
- Validation des références FHIR inter-ressources — hors scope Phase 5
- Support XML (la validation opère sur le modèle AST indépendamment du format)

## Decisions

### Architecture en couches
Chaque couche est un module Rust privé (`structural`, `cardinality`, `type_check`, `terminology`, `invariants`) avec une fonction `validate(resource, sd, [registry,] diagnostics)`. L'orchestrateur dans `lib.rs` les appelle séquentiellement. Cette architecture facilite l'ajout ou la suppression de couches sans modifier l'API publique.

### Court-circuit sur STRUCTURE_000
Si le type de ressource est inconnu du registry, la validation s'arrête immédiatement. Les couches suivantes ne peuvent pas fonctionner sans StructureDefinition, et empiler d'autres erreurs serait trompeur.

### Profondeur 1 uniquement pour structurel et cardinalité
Les couches structurelle et cardinalité n'inspectent que les éléments de profondeur 1 (enfants directs) du snapshot. La validation récursive des sous-éléments est planifiée pour une phase ultérieure.

### Polymorphisme value[x]
Les champs polymorphiques sont gérés par une liste exhaustive de suffixes de type connus (`TYPE_SUFFIXES`). Un champ nommé `valueBoolean` est accepté si le snapshot déclare `value[x]`. Cela évite de faux positifs STRUCTURE_001.

### Diagnostic avec location optionnelle
Le champ `location: Option<Span>` permet d'attacher la position source (ligne/colonne) quand le parser l'a préservée, sans rendre la location obligatoire (ex. : CARDINALITY_001 sur un champ absent n'a pas de position).

### Invariants — erreurs d'évaluation silencieuses
Les erreurs d'évaluation FHIRPath sur les invariants sont ignorées silencieusement. De nombreuses expressions FHIR R5 utilisent des fonctions non encore implémentées dans `fhir-fhirpath`. Ignorer ces erreurs évite un bruit excessif pendant le développement incrémental.

### Modèle fhir-definitions étendu
`Constraint` et `Binding` sont ajoutés directement à `ElementDefinition`. Le loader parse les tableaux `constraint` et l'objet `binding` de chaque élément snapshot. Ces champs sont optionnels (`constraints` par défaut vide, `binding` est `Option`).

## Risks

- **Invariants incomplets** : `fhir-fhirpath` ne couvre pas encore toutes les fonctions FHIRPath utilisées dans les expressions du snapshot R5. Les expressions non supportées sont silencieusement ignorées, ce qui peut masquer des violations réelles.
- **Validation profondeur 1 seulement** : La Phase 5 ne valide pas les sous-éléments (ex. : `Patient.name.given`). Des ressources partiellement invalides peuvent passer sans erreur sur les champs imbriqués.
- **Terminologie stub** : Les bindings terminologiques sont parsés mais pas vérifiés. Des valeurs de code invalides ne seront pas détectées jusqu'à l'implémentation de l'expansion ValueSet.
