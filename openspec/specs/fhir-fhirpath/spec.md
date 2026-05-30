# Spec: fhir-fhirpath

## Purpose

Évaluateur FHIRPath 3.0 (STU) en Rust, utilisé pour valider les invariants `constraint.expression` des StructureDefinitions FHIR R5.
Implémente la spécification HL7 FHIRPath 3.0 (https://build.fhir.org/ig/HL7/FHIRPath/en/).

## Requirements

### Requirement: API publique evaluate

Le système SHALL exposer `evaluate(expression: &str, context: &[Value]) -> Result<Collection, EvalError>` pour parser et évaluer une expression FHIRPath en une seule étape.

#### Scenario: Expression valide

- **WHEN** `evaluate` reçoit une expression FHIRPath syntaxiquement valide et un contexte non vide
- **THEN** il SHALL retourner `Ok(Collection)` contenant le résultat de l'évaluation

#### Scenario: Expression syntaxiquement invalide

- **WHEN** `evaluate` reçoit une expression malformée
- **THEN** il SHALL retourner `Err(EvalError::Parse(ParseError::Syntax(...)))`

#### Scenario: Contexte vide

- **WHEN** `evaluate` reçoit un contexte vide (`&[]`)
- **THEN** il SHALL retourner `Ok(vec![])` pour les expressions de navigation

---

### Requirement: API publique parse

Le système SHALL exposer `parse(expression: &str) -> Result<ast::Expr, ParseError>` pour parser une expression FHIRPath sans l'évaluer.

#### Scenario: Expression valide retourne un AST

- **WHEN** `parse` reçoit une expression FHIRPath syntaxiquement valide
- **THEN** il SHALL retourner `Ok(Expr)` représentant l'AST complet de l'expression

#### Scenario: Expression invalide retourne ParseError

- **WHEN** `parse` reçoit une expression syntaxiquement invalide
- **THEN** il SHALL retourner `Err(ParseError::Syntax(...))` avec un message décrivant l'erreur

---

### Requirement: Grammaire PEG FHIRPath 3.0

Le système SHALL implémenter une grammaire PEG via `pest` couvrant l'intégralité de la syntaxe FHIRPath 3.0.

#### Scenario: Littéraux pris en charge

- **WHEN** l'expression contient un littéral (booléen, entier, Long `45L`, décimal, chaîne entre guillemets simples, date `@YYYY-MM-DD`, dateTime complet `@YYYY-MM-DDThh:mm:ss`, dateTime partiel `@YYYY-MM-DDT` / `@YYYY-MMT` / `@YYYYT`, heure `@Thh:mm:ss`, quantité `1.5 'mg'`)
- **THEN** le parser SHALL produire le nœud AST correspondant sans erreur

#### Scenario: Opérateurs binaires

- **WHEN** l'expression contient un opérateur (`+`, `-`, `*`, `/`, `div`, `mod`, `&`, `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, `and`, `or`, `xor`, `implies`, `in`, `contains`, `is`, `as`, `|`)
- **THEN** le parser SHALL produire le nœud AST avec la bonne associativité et priorité

#### Scenario: Appels de fonctions

- **WHEN** l'expression contient un appel de fonction ou méthode (ex. `name.exists()`, `where(active = true)`)
- **THEN** le parser SHALL produire `Expr::Method` ou `Expr::FuncCall` avec les arguments corrects

---

### Requirement: AST Expr

Le système SHALL exposer un enum `Expr` couvrant toutes les constructions FHIRPath 3.0.

#### Scenario: Variantes de l'AST

- **WHEN** un AST est construit
- **THEN** `Expr` SHALL couvrir : littéraux (`Null`, `Bool`, `Integer`, `Long`, `Decimal`, `String`, `Temporal`, `Quantity`), navigation (`DollarThis`, `DollarIndex`, `Ident`, `Dot`, `Index`), appels (`FuncCall`, `Method`), arithmétique (`Add`, `Sub`, `Mul`, `Div`, `DivInt`, `Mod`, `Concat`, `Neg`), comparaisons (`Eq`, `Neq`, `Lt`, `Lte`, `Gt`, `Gte`, `Equiv`, `NotEquiv`), booléens (`And`, `Or`, `Xor`, `Implies`), types (`Is`, `As`), ensembles (`Union`, `In`, `Contains`)

---

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les valeurs FHIRPath à l'exécution.

#### Scenario: Variantes couvertes

- **WHEN** un résultat d'évaluation est produit
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Long(i64)`, `Decimal(f64)`, `String(Arc<str>)`, `Date(Arc<str>)`, `DateTime(Arc<str>)`, `Time(Arc<str>)`, `Quantity(f64, Arc<str>)`, `Object(Arc<IndexMap<Arc<str>, Collection>>)`

#### Scenario: Conversion depuis un Resource parser

- **WHEN** `resource_to_value(resource: &Resource) -> Value` est appelé
- **THEN** il SHALL retourner `Value::Object` représentant la ressource FHIR, utilisable comme contexte pour `evaluate`

#### Scenario: Conversion depuis un Node parser

- **WHEN** `from_parser_value(node: &fhir_parser::Value) -> Value` est appelé
- **THEN** il SHALL mapper chaque variante `fhir_parser::Value` vers la `Value` FHIRPath correspondante

---

### Requirement: Type Collection

Le système SHALL exposer `type Collection = Vec<Value>` comme type de retour de toute évaluation FHIRPath.

#### Scenario: Collection vide pour absence de résultat

- **WHEN** une expression FHIRPath ne sélectionne aucune valeur
- **THEN** `evaluate` SHALL retourner `Ok(vec![])` (collection vide)

---

### Requirement: Fonctions d'existence

Le système SHALL implémenter `empty()`, `exists()`, `exists(criteria)`, `all(criteria)`, `allTrue()`, `anyTrue()`, `allFalse()`, `anyFalse()`.

#### Scenario: empty sur collection vide

- **WHEN** `empty()` est évalué sur une collection vide
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: exists avec critère

- **WHEN** `exists(x | x > 0)` est évalué sur une collection d'entiers
- **THEN** le résultat SHALL être `[Bool(true)]` si au moins un élément satisfait le critère

---

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`.

#### Scenario: where filtre les éléments

- **WHEN** `where(active = true)` est évalué sur une collection d'objets
- **THEN** le résultat SHALL contenir uniquement les éléments pour lesquels le critère est vrai

#### Scenario: ofType filtre par type FHIRPath

- **WHEN** `ofType('string')` est évalué sur une collection mixte
- **THEN** le résultat SHALL contenir uniquement les valeurs de type `String`

---

### Requirement: Fonctions de sous-ensemble

Le système SHALL implémenter `first()`, `last()`, `tail()`, `skip(n)`, `take(n)`, `single()`, `count()`.

#### Scenario: first retourne le premier élément

- **WHEN** `first()` est évalué sur une collection non vide
- **THEN** le résultat SHALL être une collection contenant uniquement le premier élément

#### Scenario: single retourne une erreur si plusieurs éléments

- **WHEN** `single()` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

---

### Requirement: Fonctions de chaînes

Le système SHALL implémenter `toString()`, `length()`, `startsWith(s)`, `endsWith(s)`, `contains(s)`, `upper()`, `lower()`, `trim()`, `substring(start)`, `substring(start, length)`, `replace(pattern, substitution)`, `matches(regex, [flags])`, `matchesFull(regex, [flags])`, `replaceMatches(regex, substitution, [flags])`, `indexOf(s)`, `lastIndexOf(s)`, `split(separator)`, `join(separator)`, `encode(format)`, `decode(format)`, `escape(target)`, `unescape(target)`.

Le paramètre optionnel `flags` accepte `i` (insensible à la casse) et `m` (multiligne).
`escape`/`unescape` supportent les cibles `'html'` et `'json'`.

#### Scenario: length sur une chaîne

- **WHEN** `'hello'.length()` est évalué
- **THEN** le résultat SHALL être `[Integer(5)]`

#### Scenario: matches avec regex

- **WHEN** `'abc123'.matches('[0-9]+')` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: matches avec flag insensible à la casse

- **WHEN** `'Hello'.matches('hello', 'i')` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: matchesFull exige correspondance complète

- **WHEN** `'hello world'.matchesFull('hello')` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: lastIndexOf retourne la dernière occurrence

- **WHEN** `'abcabc'.lastIndexOf('b')` est évalué
- **THEN** le résultat SHALL être `[Integer(4)]`

#### Scenario: escape HTML

- **WHEN** `'<b>'.escape('html')` est évalué
- **THEN** le résultat SHALL être `[String('&lt;b&gt;')]`

#### Scenario: substring longueur négative ou zéro

- **WHEN** `'hello'.substring(0, -1)` ou `'hello'.substring(0, 0)` est évalué
- **THEN** le résultat SHALL être `[String('')]` (chaîne vide, pas collection vide)

---

### Requirement: Fonctions mathématiques

Le système SHALL implémenter `abs()`, `ceiling()`, `floor()`, `round()`, `round(precision)`, `sqrt()`, `exp()`, `ln()`, `log(base)`, `power(exponent)`, `truncate()`.

#### Scenario: abs sur valeur négative

- **WHEN** `(-3).abs()` est évalué
- **THEN** le résultat SHALL être `[Integer(3)]`

#### Scenario: sqrt sur entier positif

- **WHEN** `4.sqrt()` est évalué
- **THEN** le résultat SHALL être `[Decimal(2.0)]`

---

### Requirement: Fonctions de conversion

Le système SHALL implémenter `toInteger()`, `toLong()`, `toDecimal()`, `toBoolean()`, `convertsToInteger()`, `convertsToLong()`, `convertsToDecimal()`, `convertsToBoolean()`, `toDate()`, `toDateTime()`, `toTime()`, `toQuantity()`, `convertsToDate()`, `convertsToDateTime()`, `convertsToTime()`, `convertsToQuantity()`, `toString()`.

`toLong()` retourne une valeur `Long` (type distinct d'`Integer`).

#### Scenario: toInteger depuis une chaîne

- **WHEN** `'42'.toInteger()` est évalué
- **THEN** le résultat SHALL être `[Integer(42)]`

#### Scenario: toLong depuis un Integer

- **WHEN** `42.toLong()` est évalué
- **THEN** le résultat SHALL être `[Long(42)]`

#### Scenario: convertsToInteger sur chaîne non numérique

- **WHEN** `'abc'.convertsToInteger()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

---

### Requirement: Fonctions de collections

Le système SHALL implémenter `distinct()`, `isDistinct()`, `subsetOf(other)`, `supersetOf(other)`, `intersect(other)`, `exclude(other)`, `combine(other, [preserveOrder])`, `coalesce(values...)`, ainsi que l'opérateur `|` (union).

`combine()` diffère de `|` : il conserve les doublons.
`coalesce()` retourne le premier résultat non vide parmi ses arguments.

#### Scenario: distinct élimine les doublons

- **WHEN** `distinct()` est évalué sur une collection avec doublons
- **THEN** le résultat SHALL contenir chaque valeur unique une seule fois, dans l'ordre d'apparition

#### Scenario: intersect retourne les éléments communs

- **WHEN** `intersect(other)` est évalué
- **THEN** le résultat SHALL contenir uniquement les éléments présents dans les deux collections

#### Scenario: combine conserve les doublons

- **WHEN** `(1 | 2).combine(2 | 3)` est évalué
- **THEN** le résultat SHALL être `[1, 2, 2, 3]` (doublons conservés)

#### Scenario: coalesce retourne le premier non vide

- **WHEN** `coalesce({}, 'second', 'third')` est évalué
- **THEN** le résultat SHALL être `[String('second')]`

---

### Requirement: Fonctions d'agrégation

Le système SHALL implémenter `aggregate(aggregator, init)`, `sum()`, `min()`, `max()`, `avg()`, `sort()`, `sort(keySelector)`.

#### Scenario: sum sur une collection d'entiers

- **WHEN** `sum()` est évalué sur `[Integer(1), Integer(2), Integer(3)]`
- **THEN** le résultat SHALL être `[Integer(6)]`

#### Scenario: max sur une collection de décimaux

- **WHEN** `max()` est évalué sur une collection de valeurs numériques
- **THEN** le résultat SHALL retourner la valeur maximale

#### Scenario: avg calcule la moyenne

- **WHEN** `(1 | 3 | 5).avg()` est évalué
- **THEN** le résultat SHALL être `[Decimal(3.0)]`

#### Scenario: sort tri naturel

- **WHEN** `(3 | 1 | 2).sort()` est évalué
- **THEN** le résultat SHALL être `[1, 2, 3]` (ordre croissant)

---

### Requirement: Fonctions de navigation

Le système SHALL implémenter `children()` et `descendants()`.

#### Scenario: children retourne les champs directs d'un objet

- **WHEN** `children()` est évalué sur un `Value::Object`
- **THEN** le résultat SHALL contenir toutes les valeurs des champs de premier niveau de l'objet

#### Scenario: descendants retourne tous les nœuds imbriqués

- **WHEN** `descendants()` est évalué sur un objet avec des objets imbriqués
- **THEN** le résultat SHALL contenir récursivement tous les nœuds enfants et leurs descendants

---

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `now()`, `today()`, `timeOfDay()`, `hasValue()`, `getValue()`, `htmlChecks()`, `comparable(other)`, `type()`, `lowBoundary()`, `highBoundary()`, `precision()`.

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`

---

### Requirement: Erreurs d'évaluation

Le système SHALL exposer `EvalError` couvrant toutes les erreurs d'exécution possibles.

#### Scenario: Variantes d'EvalError

- **WHEN** une erreur se produit à l'évaluation
- **THEN** `EvalError` SHALL avoir les variantes : `Type(String)` (erreur de type), `UndefinedFunction(String)` (fonction inconnue), `Arity { name, expected, got }` (mauvais nombre d'arguments), `DivisionByZero`, `IndexOutOfBounds(i64)`, `Parse(ParseError)` (erreur de parse propagée)

---

### Requirement: Limitations connues

Le système SHALL documenter les limitations suivantes de l'implémentation actuelle.

#### Scenario: $index non supporté

- **WHEN** une expression utilise `$index` dans un contexte d'itération
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction("$index"))`

#### Scenario: aggregate sans $total

- **WHEN** `aggregate()` est utilisé avec une référence à `$total`
- **THEN** l'accumulateur `$total` ne sera pas accessible (comportement non défini dans l'implémentation actuelle)

#### Scenario: Fonctions terminologiques retournent une collection vide

- **WHEN** `resolve()`, `conformsTo()` ou `memberOf()` sont évalués
- **THEN** le résultat SHALL être une collection vide (aucun serveur terminologique connecté)
