# Tasks: fhir-definitions-phase1

## 1. Modèle de données FHIR R5

- [x] Définir le type `StructureDefinition` avec les champs : `url`, `name`, `kind`, `is_abstract`, `base_definition`, `snapshot`
- [x] Définir le type `ElementDefinition` avec les champs : `path`, `min`, `max`, `types`
- [x] Définir le type `ValueSet` avec les champs : `url`, `name`
- [x] Définir le type `CodeSystem` avec les champs : `url`, `name`
- [x] Utiliser `Arc<str>` pour tous les champs de type chaîne

## 2. Chargeur de bundles JSON

- [x] Implémenter `load_bundle(path, sds, vss, css)` pour parser un bundle FHIR JSON
- [x] Parser les entrées `StructureDefinition` (url, name, kind, abstract, baseDefinition, snapshot)
- [x] Parser les éléments du snapshot (path, min, max, types[].code)
- [x] Parser les entrées `ValueSet` (url, name)
- [x] Parser les entrées `CodeSystem` (url, name)
- [x] Ignorer silencieusement les types de ressources non reconnus
- [x] Ignorer les ressources avec des champs obligatoires manquants (url, path)
- [x] Retourner `Error::Io` pour les fichiers manquants
- [x] Retourner `Error::Json` pour un JSON invalide (avec nom de fichier)
- [x] Retourner `Error::MalformedBundle` si le tableau `entry` est absent

## 3. Registry

- [x] Implémenter `Registry::from_definitions_dir(dir)` chargeant les 3 fichiers FHIR R5
- [x] Implémenter `structure_definition(url)` → `Option<&StructureDefinition>`
- [x] Implémenter `value_set(url)` → `Option<&ValueSet>`
- [x] Implémenter `code_system(url)` → `Option<&CodeSystem>`
- [x] Implémenter `structure_definition_count()`, `value_set_count()`, `code_system_count()`
- [x] S'assurer que le `Registry` est `Send + Sync` (partage via `Arc<Registry>`)

## 4. Tests

- [x] Tests unitaires `loader`: Io, Json, MalformedBundle, skip unknown types, skip missing url, skip missing path, parse SD, parse VS+CS
- [x] Tests unitaires `registry`: lookup case-sensitive
- [x] Tests d'intégration `registry`: load Patient SD, base_definition, multiple SDs, ValueSet by url, None for unknown, CodeSystems, snapshot elements, is_abstract, value_set_count, from_nonexistent_dir
