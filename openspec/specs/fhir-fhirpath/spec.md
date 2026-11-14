# Spec: fhir-fhirpath

## Purpose

Évaluateur FHIRPath en Rust, utilisé pour valider les invariants `constraint.expression` des StructureDefinitions FHIR R5.
Implémente un sous-ensemble de FHIRPath release 2 (2.0.0, http://hl7.org/fhirpath/N1/), version utilisée par FHIR R5, plus quelques ajouts ultérieurs ; les fonctions non implémentées retournent `EvalError::UndefinedFunction`.
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

#### Scenario: Expression déjà parsée

- **WHEN** `evaluate_parsed(&Expr, context)` reçoit un `Expr` produit par `parse`
- **THEN** il SHALL évaluer sans reparser, avec le même résultat que `evaluate` ; `fhir-definitions` met en cache le résultat de `parse` par `Constraint` (au plus un parse par contrainte)

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

### Requirement: Grammaire PEG FHIRPath release 2

Le système SHALL implémenter une grammaire PEG via `pest` couvrant la syntaxe de FHIRPath release 2 (2.0.0), version utilisée par FHIR R5, plus quelques ajouts ultérieurs (par ex. le littéral Long).

#### Scenario: Littéraux pris en charge

- **WHEN** l'expression contient un littéral (booléen, entier, Long `45L`, décimal, chaîne entre guillemets simples, date `@YYYY-MM-DD`, dateTime complet `@YYYY-MM-DDThh:mm:ss`, dateTime partiel `@YYYY-MM-DDT` / `@YYYY-MMT` / `@YYYYT`, heure `@Thh:mm:ss`, quantité `1.5 'mg'` ou avec mot-clé calendaire `4 days`)
- **THEN** le parser SHALL produire le nœud AST correspondant sans erreur

#### Scenario: Date et DateTime calendaires

- **WHEN** un littéral de date ou de dateTime n'est pas une date calendaire (`@2023-02-29`, `@2023-13`, `@0000`)
- **THEN** `parse` SHALL retourner `Err(ParseError::Syntax(...))` ; l'année SHALL être `0001` à `9999`, le mois `01` à `12` et le jour valide pour son mois (années bissextiles comprises), et `@2024-02-29` SHALL être accepté

#### Scenario: Littéral Time non borné

- **WHEN** un littéral `@Thh:mm:ss` est analysé
- **THEN** ses valeurs ne SHALL pas être contrôlées en plage (comportement inchangé)

#### Scenario: Mot-clé calendaire dans une quantité

- **WHEN** `4 days` ou `1 day` est analysé
- **THEN** le mot-clé calendaire (singulier ou pluriel) SHALL être accepté et conservé au singulier (`'day'`) ; un mot inconnu (`1 fortnight`) ou suivi de texte (`5 daysx`) SHALL être une erreur

#### Scenario: $total dans la grammaire

- **WHEN** l'expression contient `$total`
- **THEN** le parser SHALL l'accepter comme variable (voir « Fonctions d'agrégation »)

#### Scenario: Opérateurs binaires

- **WHEN** l'expression contient un opérateur (`+`, `-`, `*`, `/`, `div`, `mod`, `&`, `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, `and`, `or`, `xor`, `implies`, `in`, `contains`, `is`, `as`, `|`)
- **THEN** le parser SHALL produire le nœud AST avec la bonne associativité et priorité

#### Scenario: Priorité de is/as et de l'union

- **WHEN** l'expression combine `|` avec `is` ou `as` (ex. `a | b is X`)
- **THEN** `is`/`as` SHALL lier plus fort que `|` (`a | (b is X)`)

#### Scenario: Limites de taille et d'imbrication

- **WHEN** l'expression dépasse 1 KiB ou plus de 16 niveaux d'imbrication de `(` / `[` (hors chaînes simples ou doubles, backticks et commentaires)
- **THEN** `parse` SHALL retourner `Err(ParseError::Syntax(...))` sans dépasser la pile ; toute expression acceptée au maximum de ces limites (chaînes `a.a.a…`, `1+1+…`, parenthèses imbriquées) SHALL pouvoir être parsée, évaluée et libérée dans un thread de 2 Mo en debug

#### Scenario: Identifiants délimités par backticks

- **WHEN** l'expression contient un DELIMITEDIDENTIFIER (ex. ``text.`div` ``) dans un chemin, un nom de fonction ou un spécificateur de type
- **THEN** il SHALL être évalué exactement comme l'identifiant simple correspondant

#### Scenario: Séquences d'échappement

- **WHEN** une chaîne littérale ou un identifiant délimité contient une séquence d'échappement
- **THEN** le parser SHALL accepter l'ensemble ESC de la spécification : `` \` ``, `\'`, `\"`, `\\`, `\/`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`

#### Scenario: Chaîne entre guillemets doubles (extension hors N1)

- **WHEN** l'expression contient un littéral `"..."` (ex. `type.code.contains(":")`, contrainte R5 eld-11)
- **THEN** le parser SHALL l'accepter comme chaîne, avec les mêmes séquences d'échappement que `'...'` ; il s'agit d'une extension délibérée de la grammaire FHIRPath N1 (chaînes simples uniquement), les identifiants à backticks sont inchangés et les unités de quantité restent entre guillemets simples uniquement

#### Scenario: Identifiants débutant par true/false

- **WHEN** l'expression contient un identifiant tel que `trueValue`
- **THEN** il SHALL être analysé comme identifiant et non comme littéral booléen

#### Scenario: Appels de fonctions

- **WHEN** l'expression contient un appel de fonction ou méthode (ex. `name.exists()`, `where(active = true)`)
- **THEN** le parser SHALL produire `Expr::Method` ou `Expr::FuncCall` avec les arguments corrects

---

### Requirement: AST Expr

Le système SHALL exposer un enum `Expr` couvrant toutes les constructions de la grammaire prise en charge.

#### Scenario: Variantes de l'AST

- **WHEN** un AST est construit
- **THEN** `Expr` SHALL couvrir : littéraux (`Null`, `Bool`, `Integer`, `Long`, `Decimal`, `String`, `Temporal`, `Quantity`), navigation (`DollarThis`, `DollarIndex`, `External`, `Ident`, `Dot`, `Index`), appels (`FuncCall`, `Method`), arithmétique (`Add`, `Sub`, `Mul`, `Div`, `DivInt`, `Mod`, `Concat`, `Neg`), comparaisons (`Eq`, `Neq`, `Lt`, `Lte`, `Gt`, `Gte`, `Equiv`, `NotEquiv`), booléens (`And`, `Or`, `Xor`, `Implies`), types (`Is`, `As`), ensembles (`Union`, `In`, `Contains`)

---

### Requirement: Type Value

Le système SHALL exposer un enum `Value` représentant les valeurs FHIRPath à l'exécution.

#### Scenario: Variantes couvertes

- **WHEN** un résultat d'évaluation est produit
- **THEN** `Value` SHALL avoir les variantes : `Null`, `Bool(bool)`, `Integer(i64)`, `Long(i64)`, `Decimal(f64)`, `String(Arc<str>)`, `Date(Arc<str>)`, `DateTime(Arc<str>)`, `Time(Arc<str>)`, `Quantity(f64, Arc<str>)`, `Object(Arc<IndexMap<Arc<str>, Collection>>)`, `Primitive(Box<Value>, Arc<IndexMap<Arc<str>, Collection>>, Option<Arc<str>>)` (valeur, enfants `id`/`extension`, code de type FHIR)

#### Scenario: Conversion depuis un Resource parser

- **WHEN** `resource_to_value(resource: &Resource) -> Value` est appelé
- **THEN** il SHALL retourner `Value::Object` représentant la ressource FHIR, utilisable comme contexte pour `evaluate`, sans typage des primitives (équivalent à `resource_to_typed_value` avec un `child_element` qui retourne toujours `None`)

#### Scenario: Conversion depuis un Node parser

- **WHEN** `from_parser_value(node: &fhir_parser::Value) -> Vec<Value>` est appelé
- **THEN** il SHALL mapper chaque variante `fhir_parser::Value` vers la `Value` FHIRPath correspondante, sans typage des primitives

#### Scenario: null JSON exclu

- **WHEN** un tableau JSON contient `null`, ou un champ ne contient que `null`
- **THEN** les `null` ne SHALL pas être des éléments de la collection (sauf s'ils sont appariés à un `_x` objet, qui donne une primitive sans valeur) et un champ ne contenant que `null` SHALL être absent de l'objet

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

#### Scenario: $index dans exists et all

- **WHEN** `('a' | 'b').exists($index = 1)` ou `('a' | 'b').all($index < 2)` est évalué
- **THEN** `$index` SHALL valoir la position (Integer, base 0) de l'élément courant et les résultats SHALL être `[Bool(true)]`

#### Scenario: Critère non booléen

- **WHEN** le critère de `where`, `exists` ou `all` produit pour un élément autre chose qu'un booléen seul ou une collection vide (ex. `(1 | 2).exists('a')`)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: allTrue sur des éléments non booléens

- **WHEN** `allTrue()`, `anyTrue()`, `allFalse()` ou `anyFalse()` est évalué sur une collection contenant un élément non booléen
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

### Requirement: Fonctions de filtrage

Le système SHALL implémenter `where(criteria)`, `select(projection)`, `repeat(projection)`, `ofType(type)`, `is(type)`, `as(type)`.

L'argument de `ofType`, `is` et `as` est un identifiant de nom de type. La correspondance de type (partagée par `ofType`, les fonctions `is`/`as` et les opérateurs `is`/`as`) SHALL accepter les types primitifs FHIRPath ainsi qu'un objet ressource dont `resourceType` est exactement égal au nom de type. Elle ne SHALL pas gérer les noms qualifiés (`FHIR.X`, `System.X`). Lorsque `Environment::is_subtype` est fourni et que la valeur porte un type FHIR (`Value::Primitive` avec un code de type), `ofType`, `is` et `as` SHALL répondre par l'ancêtrie des types FHIR pour un nom de type FHIR (`code` est une `string`, `canonical` et `url` sont des `uri` ; `string` n'est pas un `code`, `uri` n'est pas un `url`). Les noms System (`String`, `Integer`, ...) et les valeurs sans type FHIR (littéraux) SHALL conserver les règles System. Avec le crochet, un nom non qualifié qui n'est ni un type System ni un type FHIR (ex. `string1`) SHALL retourner `Err(EvalError::Type(...))` (`Any` et les noms qualifiés `System.X`/`FHIR.X` conservent la réponse précédente). Sans ce crochet (défaut), tester une `String` contre un sous-type FHIR de `string` SHALL échouer avec `EvalError::Type`.

L'opérateur `is` (`X is Type`) SHALL avoir exactement la même sémantique que la fonction `is(Type)` (N1 §6.3.1) : collection vide en entrée -> collection vide, un seul élément -> `[Bool(true)]` ou `[Bool(false)]`, plus d'un élément -> `Err(EvalError::Type(...))`.

L'opérateur `as` (`X as Type`) SHALL avoir la même sémantique que la fonction `as(Type)` : collection vide en entrée -> collection vide, un seul élément -> l'élément s'il correspond, sinon une collection vide, plus d'un élément -> `Err(EvalError::Type(...))`.

Le critère de `where` SHALL produire pour chaque élément un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))`.

#### Scenario: where filtre les éléments

- **WHEN** `where(active = true)` est évalué sur une collection d'objets
- **THEN** le résultat SHALL contenir uniquement les éléments pour lesquels le critère est vrai

#### Scenario: $index dans where, select et repeat

- **WHEN** `('a' | 'b' | 'c').where($index > 0)` ou `('a' | 'b' | 'c').select($index)` est évalué
- **THEN** `$index` SHALL valoir la position (Integer, base 0) de l'élément dans la collection itérée, soit `['b', 'c']` puis `[0, 1, 2]` ; dans `repeat`, il SHALL valoir la position dans la ronde courante (la spécification N1 est muette)

#### Scenario: $index de l'itération la plus interne

- **WHEN** `(1 | 2).select((10 | 20 | 30).where($index = 2))` est évalué
- **THEN** `$index` SHALL désigner la position dans l'itération la plus interne, soit `[30, 30]`

#### Scenario: ofType filtre par type FHIRPath

- **WHEN** `ofType(String)` est évalué sur une collection mixte
- **THEN** le résultat SHALL contenir uniquement les valeurs de type `String`

#### Scenario: ofType filtre par resourceType

- **WHEN** `ofType(Patient)` est évalué sur une collection de ressources
- **THEN** le résultat SHALL contenir uniquement les objets dont `resourceType` vaut exactement `Patient`

#### Scenario: is et as sur un seul élément

- **WHEN** `is(Patient)` ou `as(Patient)` est évalué sur une collection d'un seul élément
- **THEN** `is` SHALL retourner `[Bool(true)]` ou `[Bool(false)]`, et `as` SHALL retourner l'élément s'il correspond, sinon une collection vide

#### Scenario: Héritage des types FHIR

- **WHEN** `Questionnaire.status.is(string)`, `Questionnaire.status is code` ou `Questionnaire.status.as(code)` est évalué avec le crochet `is_subtype` du Registry
- **THEN** le résultat SHALL être `[Bool(true)]`, `[Bool(true)]` puis la valeur `draft`

#### Scenario: Un supertype n'est pas son sous-type

- **WHEN** `Questionnaire.title.is(code)`, `Questionnaire.title as code` ou `Questionnaire.url.is(url)` est évalué avec le crochet
- **THEN** le résultat SHALL être `[Bool(false)]`, une collection vide, puis `[Bool(false)]`

#### Scenario: ofType par type FHIR

- **WHEN** `Questionnaire.descendants().ofType(canonical)` puis `ofType(uri)` sont évalués avec le crochet
- **THEN** le premier SHALL ne retourner que les canonical, le second les `uri` et leurs sous-types (`url`, `canonical`, l'`url` d'extension)

#### Scenario: Type FHIR de l'id de ressource et des primitives sans valeur

- **WHEN** `Questionnaire.id.is(id)` ou `Questionnaire.version.is(string)` (où `version` n'a que `_version`) est évalué avec le crochet
- **THEN** le résultat SHALL être `[Bool(true)]` (l'`id` est typé par `child_element(resourceType, "id")`), et `version.is(code)` SHALL être `[Bool(false)]`

#### Scenario: Primitives booléennes, entières et décimales typées

- **WHEN** un champ `boolean`, `integer` ou `decimal` est converti avec `child_element`
- **THEN** la valeur SHALL être une `Primitive` portant son type FHIR (`integer`), et l'arithmétique et les comparaisons SHALL la traiter comme sa valeur nue

#### Scenario: Primitive XML avec extensions

- **WHEN** `Questionnaire.status.is(code)` est évalué avec le crochet sur un `status` XML portant une extension
- **THEN** le résultat SHALL être `[Bool(true)]` (la `Primitive` porte son type FHIR)

#### Scenario: Noms System et littéraux

- **WHEN** `Questionnaire.status.is(String)`, `Questionnaire.status.ofType(integer)` ou `'draft'.is(code)` est évalué avec le crochet
- **THEN** les deux premiers SHALL suivre les règles System (`[Bool(true)]`, collection vide) et `'draft'.is(code)`, littéral sans type FHIR, SHALL retourner `Err(EvalError::Type(...))` (`'draft'.is(String)` vaut `[Bool(true)]`)

#### Scenario: Sans crochet is_subtype

- **WHEN** `Questionnaire.status.is(code)` ou `ofType(canonical)` est évalué sans `is_subtype`
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (le validateur, qui ne fournit pas encore le crochet, émet `INVARIANT_SKIPPED`)

#### Scenario: is et as sur une collection vide

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection vide
- **THEN** le résultat SHALL être une collection vide

#### Scenario: is et as sur plusieurs éléments

- **WHEN** `is(type)` ou `as(type)` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (test officiel `testFHIRPathAsFunction21`)

#### Scenario: l'opérateur is suit la sémantique de is()

- **WHEN** `X is Type` est évalué avec `X` vide, d'un seul élément, ou de plusieurs éléments
- **THEN** le résultat SHALL être respectivement une collection vide, `[Bool(true)]`/`[Bool(false)]`, ou `Err(EvalError::Type(...))`, comme pour `X.is(Type)`

#### Scenario: l'opérateur as suit la sémantique de as()

- **WHEN** `(1 | 2) as Integer` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`, comme pour `(1 | 2).as(Integer)`

### Requirement: Fonctions de sous-ensemble

Le système SHALL implémenter `first()`, `last()`, `tail()`, `skip(n)`, `take(n)`, `single()`, `count()`.

#### Scenario: first retourne le premier élément

- **WHEN** `first()` est évalué sur une collection non vide
- **THEN** le résultat SHALL être une collection contenant uniquement le premier élément

#### Scenario: single retourne une erreur si plusieurs éléments

- **WHEN** `single()` est évalué sur une collection de plus d'un élément
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: single sur collection vide

- **WHEN** `{}.single()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: skip et take avec un compte vide

- **WHEN** `(0 | 1 | 2).skip({})` ou `(0 | 1 | 2).take({})` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: skip et take avec un compte invalide

- **WHEN** `skip` ou `take` reçoit plusieurs comptes (`skip(1 | 2)`) ou un compte non `Integer` (`skip('1')`)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Indexeur hors limites ou négatif

- **WHEN** `coll[i]` est évalué avec `i` négatif ou `i` >= la taille de la collection
- **THEN** le résultat SHALL être une collection vide (FHIRPath N1 §5.5), sans erreur

#### Scenario: repeat ne retourne que les éléments projetés

- **WHEN** `repeat(expr)` est évalué
- **THEN** le résultat SHALL contenir uniquement les éléments produits par la projection (jamais les éléments d'entrée) et SHALL s'arrêter quand plus aucun nouvel élément n'apparaît

#### Scenario: Concaténation de chaînes avec +

- **WHEN** `'a' + 'b'` est évalué
- **THEN** le résultat SHALL être `[String("ab")]`

#### Scenario: Égalité FHIRPath dans les opérateurs ensemblistes

- **WHEN** `union`, `distinct`, `in`, `contains`, `intersect`, `exclude`, `subsetOf`, `supersetOf` ou `repeat` comparent des valeurs
- **THEN** la comparaison SHALL utiliser l'égalité FHIRPath (`1 = 1.0` vaut vrai)

---

### Requirement: Fonctions de chaînes

Le système SHALL implémenter `toString()`, `length()`, `startsWith(s)`, `endsWith(s)`, `contains(s)`, `upper()`, `lower()`, `trim()`, `substring(start)`, `substring(start, length)`, `replace(pattern, substitution)`, `matches(regex, [flags])`, `matchesFull(regex, [flags])`, `replaceMatches(regex, substitution, [flags])`, `indexOf(s)`, `lastIndexOf(s)`, `split(separator)`, `join(separator)`, `encode(format)`, `decode(format)`, `escape(target)`, `unescape(target)`.

Le paramètre optionnel `flags` accepte `i` (insensible à la casse) et `m` (multiligne).
`escape`/`unescape` supportent les cibles `'html'` et `'json'`.

Pour `startsWith`, `endsWith`, `contains`, `indexOf`, `lastIndexOf`, `matches`, `matchesFull`, `replace`, `replaceMatches` et `substring`, une entrée vide ou un argument vide (motif, sous-chaîne, substitution, début) SHALL donner une collection vide ; un argument non `String` (ou non `Integer` pour `substring`) SHALL rester une erreur. Pour `substring`, un début hors de `0..length-1` SHALL donner une collection vide, une longueur vide SHALL se comporter comme une longueur absente et une longueur `<= 0` SHALL donner `''`. `join` sur une entrée vide SHALL donner une collection vide.

Les fonctions de chaînes de N1 §5.6 (`indexOf`, `lastIndexOf`, `substring`, `startsWith`, `endsWith`, `contains`, `upper`, `lower`, `replace`, `matches`, `matchesFull`, `replaceMatches`, `length`, `trim`, `split`, `encode`, `decode`, `escape`, `unescape`) SHALL appliquer l'évaluation singleton à leur entrée : plusieurs éléments ou un élément qui n'est pas une `String` (y compris `Date`, `DateTime` et `Time`) SHALL donner `Err(EvalError::Type(...))`, et aucune ne SHALL lire seulement le premier élément. `join` SHALL conserver son entrée en collection. Les arguments `Integer` (`index`, `round`, `substring`) SHALL rejeter plusieurs éléments. `toString()` suit les règles de « Fonctions de conversion ».

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

#### Scenario: matchesFull reste ancré avec le flag m

- **WHEN** `'a\nb'.matchesFull('a', 'm')` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` (ancrage sur tout le texte, pas par ligne)

#### Scenario: decode rejette les entrées invalides

- **WHEN** `decode('base64')` ou `decode('urlbase64')` reçoit des caractères hors alphabet (hors espaces ASCII ignorés) ou un contenu non UTF-8
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: lastIndexOf retourne la dernière occurrence

- **WHEN** `'abcabc'.lastIndexOf('b')` est évalué
- **THEN** le résultat SHALL être `[Integer(4)]`

#### Scenario: escape HTML

- **WHEN** `'<b>'.escape('html')` est évalué
- **THEN** le résultat SHALL être `[String('&lt;b&gt;')]`

#### Scenario: substring longueur négative ou zéro

- **WHEN** `'hello'.substring(0, -1)` ou `'hello'.substring(0, 0)` est évalué
- **THEN** le résultat SHALL être `[String('')]` (chaîne vide, pas collection vide)

#### Scenario: Entrée ou argument vide d'une fonction de chaîne

- **WHEN** `{}.startsWith('a')`, `'abc'.startsWith({})`, `'abc'.replace({}, 'x')` ou `'abc'.replace('a', {})` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Argument non chaîne d'une fonction de chaîne

- **WHEN** `'abc'.startsWith(1)` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: substring avec début hors limites

- **WHEN** `'hello'.substring(5)` ou `'hello'.substring(-1)` est évalué
- **THEN** le résultat SHALL être une collection vide (pas `''`)

#### Scenario: substring avec longueur vide

- **WHEN** `'hello'.substring(1, {})` est évalué
- **THEN** le résultat SHALL être `[String('ello')]`

#### Scenario: Entrée à plusieurs éléments ou non chaîne

- **WHEN** `('a' | 'b').upper()`, `1.length()`, `@2020-01-01.startsWith('2')` ou `('a' | 'b').trim()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Argument Integer à plusieurs éléments

- **WHEN** `'hello'.substring(1 | 2)` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: join conserve une entrée collection

- **WHEN** `('a' | 'b').join(',')` est évalué
- **THEN** le résultat SHALL être `[String('a,b')]`

#### Scenario: join sur une entrée vide

- **WHEN** `{}.join(',')` est évalué
- **THEN** le résultat SHALL être une collection vide

---

### Requirement: Fonctions mathématiques

Le système SHALL implémenter `abs()`, `ceiling()`, `floor()`, `round()`, `round(precision)`, `sqrt()`, `exp()`, `ln()`, `log(base)`, `power(exponent)`, `truncate()`.

#### Scenario: abs sur valeur négative

- **WHEN** `(-3).abs()` est évalué
- **THEN** le résultat SHALL être `[Integer(3)]`

#### Scenario: Dépassement sur i64::MIN

- **WHEN** `abs()` ou la négation unaire est appliqué à `i64::MIN` (Integer ou Long)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` sans paniquer

#### Scenario: Arithmétique entière exacte

- **WHEN** `+`, `-`, `*` ou `mod` est appliqué à deux `Integer`/`Long` (Integer/Long mixte donne `Long`)
- **THEN** le calcul SHALL être fait en `i64` (exact au-delà de 2^53), et un dépassement SHALL retourner `Err(EvalError::Type(...))`; `f64` n'est utilisé que si un `Decimal` est impliqué

#### Scenario: sqrt sur entier positif

- **WHEN** `4.sqrt()` est évalué
- **THEN** le résultat SHALL être `[Decimal(2.0)]`

---

### Requirement: Fonctions de conversion

Le système SHALL implémenter `toInteger()`, `toLong()`, `toDecimal()`, `toBoolean()`, `convertsToInteger()`, `convertsToLong()`, `convertsToDecimal()`, `convertsToBoolean()`, `toDate()`, `toDateTime()`, `toTime()`, `toQuantity()`, `convertsToDate()`, `convertsToDateTime()`, `convertsToTime()`, `convertsToQuantity()`, `toString()`.

`toLong()` retourne une valeur `Long` (type distinct d'`Integer`).

Pour `toX()` et `convertsToX()` avec X parmi `Boolean`, `Integer`, `Decimal`, `Date`, `DateTime`, `Time` : une entrée vide (ou un seul `Null` de substitution) SHALL donner une collection vide ; plus d'un élément SHALL être une erreur ; un élément non convertible SHALL donner une collection vide (`toX`) ou `false` (`convertsToX`), jamais une erreur. Sources convertibles :

- `toBoolean` : `Boolean` ; `Integer` `1`/`0` ; `Decimal` `1.0`/`0.0` ; `String` `true`/`t`/`yes`/`y`/`1`/`1.0` et `false`/`f`/`no`/`n`/`0`/`0.0`, sans tenir compte de la casse.
- `toInteger` : `Integer`, `Boolean`, `String` `(+|-)?\d+` ; un `Decimal` SHALL donner une collection vide.
- `toDecimal` : `Decimal`, `Integer`, `Boolean`, `String` `(+|-)?\d+(\.\d+)?` uniquement (`NaN`, `inf`, `1e5` donnent une collection vide).
- `toDate` : `Date`, la partie date d'une `DateTime`, `String` `YYYY[-MM[-DD]]`.
- `toDateTime` : `DateTime`, `Date`, `String` `YYYY[-MM[-DD]][Thh[:mm[:ss[.fff]]][Z|(+|-)hh:mm]]` avec un décalage d'au plus `14:00`.
- `toTime` : `Time`, `String` `hh[:mm[:ss[.fff]]]`.

Une date en `String` SHALL être une date calendaire réelle (année `0001` à `9999`, jour valide pour son mois, années bissextiles comprises). `toLong`/`convertsToLong` (absents de N1, FHIRPath 3.0 STU) suivent le modèle de `toInteger` : entrée singleton, sources `Integer`, `Long`, `Boolean` et `String` `(\+|-)?\d+` dans les bornes `i64`, sinon collection vide (`toLong`) ou `false` (`convertsToLong`). Une chaîne de chiffres au-delà de `f64` ne SHALL pas être convertie en infini par `toDecimal` ni `toQuantity`. De même, `toQuantity` ne SHALL pas convertir un `Decimal` non fini (littéral décimal au-delà de `f64`) : il donne une collection vide.

`toQuantity([unit])` et `convertsToQuantity([unit])` suivent N1 §5.5.7 : `Integer` et `Decimal` donnent une quantité d'unité `'1'`, une `Quantity` est inchangée, un `Boolean` donne `1.0` ou `0.0` d'unité `'1'`, une `String` conforme à `(\+|-)?\d+(\.\d+)?\s*('unité'|mot-clé)?` est convertie (une unité sans guillemets SHALL être un mot-clé calendaire). Plusieurs éléments SHALL être une erreur, une entrée vide donne une collection vide, tout autre élément donne une collection vide (`toQuantity`) ou `false` (`convertsToQuantity`). Un argument `unit` ne conserve que l'unité propre de l'entrée (aucune conversion UCUM) : une autre unité donne une collection vide, un argument vide donne une collection vide et un argument non `String` est une erreur.

Les unités calendaires des littéraux de quantité (`year`, `month`, `week`, `day`, `hour`, `minute`, `second`, `millisecond`, au singulier ou au pluriel) SHALL être stockées au singulier. `=` ne SHALL apparier une unité calendaire à son unité UCUM que pour `second`/`'s'` et `millisecond`/`'ms'` ; `~` SHALL les apparier toutes (`year`~`'a'`, `month`~`'mo'`, `week`~`'wk'`, `day`~`'d'`, `hour`~`'h'`, `minute`~`'min'`). Les comparaisons d'ordre SHALL accepter les mêmes paires que `=`.

Pour `=`, `!=`, `~`, `!~`, `<`, `<=`, `>`, `>=`, un objet FHIR Quantity (enfants typés : `value` de type `decimal`, clés limitées à value/unit/system/code/id/extension, texte décimal XML fini) SHALL être traité comme une quantité littérale (R5 §2.1.9.1). Son unité est le `code` si `system` vaut `http://unitsofmeasure.org`, sinon le texte `unit` en l'absence de `code`, sinon aucune. Un `code` avec un `system` absent ou non UCUM, ou un `comparator`, n'est pas reconnu : la comparaison structurelle d'objets s'applique. Des unités non comparables (unités différentes, aucune conversion UCUM n'étant implémentée, ou unité d'un seul côté) SHALL donner une collection vide pour `=`, `!=` et les opérateurs d'ordre, et `false` pour `~`. `sort`/`min`/`max` sont inchangés.

`toString()` et `convertsToString()` SHALL exiger une entrée singleton (plusieurs éléments : erreur). `String`, `Boolean`, `Integer`, `Long`, `Decimal`, `Date`, `DateTime`, `Time` et `Quantity` sont convertibles, tout autre élément non (collection vide pour `toString`, `false` pour `convertsToString`). Un `Decimal` SHALL toujours s'afficher avec une partie fractionnaire (`1.0` donne `'1.0'` ; `f64` ne conserve pas la précision du littéral, donc `1.50` donne `'1.5'`) et une `Quantity` SHALL s'afficher `valeur 'unité'`, unités calendaires comprises (`4 days` donne `"4 'day'"`). `toString` d'un `Decimal` non fini (littéral décimal au-delà de `f64`) SHALL donner une collection vide. `convertsToString` sur une entrée vide SHALL donner une collection vide.

#### Scenario: toInteger depuis une chaîne

- **WHEN** `'42'.toInteger()` est évalué
- **THEN** le résultat SHALL être `[Integer(42)]`

#### Scenario: toLong depuis un Integer

- **WHEN** `42.toLong()` est évalué
- **THEN** le résultat SHALL être `[Long(42)]`

#### Scenario: convertsToInteger sur chaîne non numérique

- **WHEN** `'abc'.convertsToInteger()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: Conversion sur une entrée vide

- **WHEN** `{}.toInteger()` ou `{}.convertsToBoolean()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Conversion sur plusieurs éléments

- **WHEN** `(1 | 2).toInteger()` ou `(1 | 2).convertsToInteger()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: toBoolean depuis une chaîne

- **WHEN** `'Yes'.toBoolean()` ou `'0.0'.toBoolean()` est évalué
- **THEN** le résultat SHALL être respectivement `[Bool(true)]` et `[Bool(false)]`

#### Scenario: toInteger depuis un Decimal

- **WHEN** `1.5.toInteger()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: toDecimal refuse la notation scientifique

- **WHEN** `'1e5'.toDecimal()` ou `'NaN'.toDecimal()` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: toDate depuis une DateTime

- **WHEN** `@2020-01-02T10:00:00.toDate()` est évalué
- **THEN** le résultat SHALL être `[Date('2020-01-02')]`

#### Scenario: toQuantity depuis les types numériques et Boolean

- **WHEN** `1.toQuantity()`, `1.5.toQuantity()`, `true.toQuantity()` puis `4 'mg'.toQuantity()` sont évalués
- **THEN** les résultats SHALL être `1 '1'`, `1.5 '1'`, `1.0 '1'` et `4 'mg'` (Quantity inchangée)

#### Scenario: toQuantity depuis une chaîne

- **WHEN** `'4 days'.toQuantity()`, `'1.0'.toQuantity()` puis `'1 foo'.toQuantity()` sont évalués
- **THEN** les deux premiers SHALL donner `4 'day'` et `1.0 '1'`, le dernier une collection vide (unité sans guillemets non calendaire)

#### Scenario: toQuantity avec argument unité

- **WHEN** `4 'mg'.toQuantity('mg')`, `4 'mg'.toQuantity('g')`, `4 'mg'.toQuantity({})` puis `4 'mg'.toQuantity(1)` sont évalués
- **THEN** les résultats SHALL être `4 'mg'`, une collection vide, une collection vide, puis `Err(EvalError::Type(...))`

#### Scenario: toQuantity sur plusieurs éléments

- **WHEN** `(1 | 2).toQuantity()` ou `(1 | 2).convertsToQuantity()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Chaîne de chiffres au-delà de f64

- **WHEN** une chaîne de plus de 310 chiffres est passée à `toDecimal()` ou `toQuantity()`
- **THEN** le résultat SHALL être une collection vide, et `convertsToQuantity()` SHALL valoir `[Bool(false)]`

#### Scenario: toLong depuis les sources convertibles

- **WHEN** `'12'.toLong()`, `true.toLong()`, `1.5.toLong()` puis `{}.convertsToLong()` sont évalués
- **THEN** les résultats SHALL être `[Long(12)]`, `[Long(1)]`, une collection vide puis une collection vide ; `(1 | 2).toLong()` SHALL retourner une erreur

#### Scenario: toString d'un Decimal

- **WHEN** `1.0.toString()` puis `1.50.toString()` sont évalués
- **THEN** les résultats SHALL être `[String('1.0')]` puis `[String('1.5')]`

#### Scenario: Decimal non fini

- **WHEN** `toQuantity()` ou `toString()` est évalué sur un littéral décimal au-delà de `f64` (au-dessus d'environ 1,8e308, par exemple 400 chiffres entiers)
- **THEN** le résultat SHALL être une collection vide

#### Scenario: toString d'une Quantity

- **WHEN** `4 days.toString()` est évalué
- **THEN** le résultat SHALL être `[String("4 'day'")]`

#### Scenario: toString sur plusieurs éléments

- **WHEN** `(1 | 2).toString()` est évalué
- **THEN** l'évaluateur SHALL retourner une erreur ; `{}.toString()` et `{}.convertsToString()` SHALL donner une collection vide

#### Scenario: Quantités calendaires et UCUM

- **WHEN** `1 second = 1 's'`, `1 year = 1 'a'`, `1 year ~ 1 'a'` puis `1 week ~ 1 'd'` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]`, vide, `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Quantity FHIR contre littéral

- **WHEN** `Observation.valueQuantity` a `value` 185, `system` UCUM et `code` `[lb_av]`, puis `Observation.valueQuantity = 185 '[lb_av]'` et `Observation.valueQuantity = 185 'lbs'` sont évalués
- **THEN** le premier résultat SHALL être `[Bool(true)]` et le second une collection vide

#### Scenario: Unité contre absence d'unité

- **WHEN** une Quantity FHIR `{"value": 1}` sans unité est comparée à une Quantity FHIR `{"value": 1, "unit": "tablets"}`, dans les deux sens, avec `=` puis `~`
- **THEN** `=` SHALL donner une collection vide et `~` SHALL donner `[Bool(false)]` ; deux Quantity FHIR sans unité et de même valeur SHALL être égales

#### Scenario: Date calendaire invalide

- **WHEN** `'2023-02-29'.convertsToDate()` ou `'2020-01-01T10:00+14:30'.convertsToDateTime()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`, et `'2024-02-29'.convertsToDate()` SHALL valoir `[Bool(true)]`

---

### Requirement: Fonctions de collections

Le système SHALL implémenter `distinct()`, `isDistinct()`, `subsetOf(other)`, `supersetOf(other)`, `intersect(other)`, `exclude(other)`, `combine(other, [preserveOrder])`, `coalesce(values...)`, ainsi que l'opérateur `|` (union).

`combine()` diffère de `|` : il conserve les doublons.
`coalesce()` retourne le premier résultat non vide parmi ses arguments.
L'union `|` et `intersect()` SHALL éliminer les doublons des deux opérandes, y compris ceux présents dans un seul opérande.

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

#### Scenario: union et intersect dédupliquent les deux opérandes

- **WHEN** `1 | 2.combine(2)` puis `(1 | 2).intersect(1.combine(1))` sont évalués
- **THEN** les résultats SHALL être `[1, 2]` puis `[1]`, chaque valeur n'apparaissant qu'une fois

### Requirement: Fonctions d'agrégation

Le système SHALL implémenter `aggregate(aggregator, init)`, `sum()`, `min()`, `max()`, `avg()`, `sort()`, `sort(keySelector)`.

#### Scenario: $total dans aggregate

- **WHEN** `(1 | 2 | 3).aggregate($total + $this, 0)` ou `(1 | 2).aggregate($total + $this)` est évalué
- **THEN** `$total` SHALL valoir `init` (ou une collection vide sans `init`) à la première étape, puis le résultat de chaque étape ; les résultats SHALL être `[Integer(6)]` puis une collection vide

#### Scenario: aggregate imbriqué

- **WHEN** un `aggregate()` est imbriqué dans l'agrégateur d'un autre
- **THEN** l'`aggregate()` interne SHALL avoir son propre `$total`

#### Scenario: $index dans aggregate

- **WHEN** `(5 | 6 | 7).aggregate($total + $index, 0)` est évalué
- **THEN** `$index` SHALL valoir la position de l'élément et le résultat SHALL être `[Integer(3)]`

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

#### Scenario: $index dans la clé de sort

- **WHEN** `(30 | 10 | 20).sort(-$index)` est évalué
- **THEN** `$index` SHALL valoir la position (base 0, ordre d'entrée) de l'élément trié et le résultat SHALL être `[20, 10, 30]` ; un `sort` imbriqué dans une autre itération ne SHALL pas voir le `$index` externe

---

### Requirement: Fonctions de navigation

Le système SHALL implémenter `children()` et `descendants()`. Les deux SHALL exclure le champ synthétique `resourceType` et SHALL inclure les enfants `id`/`extension` d'une `Value::Primitive`.

#### Scenario: children retourne les champs directs d'un objet

- **WHEN** `children()` est évalué sur un `Value::Object`
- **THEN** le résultat SHALL contenir toutes les valeurs des champs de premier niveau de l'objet, sauf `resourceType`

#### Scenario: descendants retourne tous les nœuds imbriqués

- **WHEN** `descendants()` est évalué sur un objet avec des objets imbriqués
- **THEN** le résultat SHALL contenir récursivement tous les nœuds enfants et leurs descendants
- **THEN** l'ordre SHALL être un parcours en profondeur préfixé (parent avant enfants, dans l'ordre des champs)

#### Scenario: Extensions d'une primitive parmi les descendants

- **WHEN** `descendants()` est évalué sur une ressource dont une primitive porte `_x.extension`
- **THEN** le résultat SHALL contenir ces extensions, et aucune valeur `resourceType`

### Requirement: Fonctions diverses

Le système SHALL implémenter `not()`, `iif(condition, trueResult, otherwiseResult)`, `trace(name)`, `hasValue()`, `getValue()`, `type()`. Les autres fonctions diverses de FHIRPath ne sont pas implémentées (voir « Limitations connues »).

La condition de `iif` SHALL être un booléen seul ou une collection vide, sinon `Err(EvalError::Type(...))` ; `iif` appelé sur une collection de plusieurs éléments SHALL retourner `Err(EvalError::Type(...))`. `not()` SHALL appliquer l'évaluation singleton dans un contexte booléen.

`hasValue()` SHALL retourner `[Bool(true)]` pour une seule primitive portant une valeur, une collection vide pour une seule valeur complexe, et `[Bool(false)]` pour une primitive sans valeur, plusieurs éléments ou aucun élément. Cela s'écarte du texte R5 (« otherwise empty ») parce que les contraintes R5 `exp-2`/`per-1` et la suite officielle attendent `false`.

#### Scenario: not inverse un booléen

- **WHEN** `true.not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: iif retourne la branche correcte

- **WHEN** `iif(true, 'yes', 'no')` est évalué
- **THEN** le résultat SHALL être `[String("yes")]`

#### Scenario: iif avec une condition invalide

- **WHEN** `iif(1 | 2 | 3, true, false)` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: hasValue sur une primitive

- **WHEN** `birthDate.hasValue()` est évalué sur un Patient avec `birthDate`, puis sans `birthDate`, puis avec `_birthDate` seul
- **THEN** les résultats SHALL être `[Bool(true)]`, `[Bool(false)]` puis `[Bool(false)]`

#### Scenario: hasValue sur une valeur complexe ou plusieurs éléments

- **WHEN** `name.hasValue()` est évalué sur un Patient avec un seul `name`, puis `name.given.hasValue()` avec deux prénoms
- **THEN** les résultats SHALL être une collection vide puis `[Bool(false)]`

### Requirement: Préfixe du nom de type de ressource

Le système SHALL permettre à une expression de commencer par le nom de type de la ressource de contexte.

#### Scenario: Nom de type correspondant

- **WHEN** `CodeSystem.content` est évalué sur une `CodeSystem` sans champ nommé `CodeSystem`
- **THEN** `evaluate` SHALL retourner la valeur de `content`

#### Scenario: Nom de type différent

- **WHEN** `Patient.id` est évalué sur une `CodeSystem`
- **THEN** `evaluate` SHALL retourner une collection vide

#### Scenario: Nom de type après le premier segment

- **WHEN** `Patient.Patient.id` est évalué sur une `Patient` sans champ nommé `Patient`
- **THEN** `evaluate` SHALL retourner une collection vide, le préfixe n'étant admis qu'en tête

#### Scenario: Champ réel prioritaire

- **WHEN** la ressource possède un champ dont le nom est égal au nom demandé
- **THEN** ce champ SHALL être retourné au lieu de la ressource elle-même

---

### Requirement: Erreurs d'évaluation

Le système SHALL exposer `EvalError` couvrant toutes les erreurs d'exécution possibles.

#### Scenario: Variantes d'EvalError

- **WHEN** une erreur se produit à l'évaluation
- **THEN** `EvalError` SHALL avoir les variantes : `Type(String)` (erreur de type), `UndefinedFunction(String)` (fonction inconnue), `Arity { name, expected, got }` (mauvais nombre d'arguments), `DivisionByZero` (conservée pour la compatibilité de l'enum public mais plus jamais produite, la division par zéro donnant une collection vide), `Parse(ParseError)` (erreur de parse propagée), `CollectionTooLarge(usize)` (collection intermédiaire au-delà de `MAX_COLLECTION_LEN`, message « collection larger than 100000 items »), `StringTooLong(usize)` (chaîne construite au-delà de `MAX_STRING_LEN`, le champ portant la limite et non la longueur réelle, message « string longer than 1048576 characters »), `StringBudgetExceeded(usize)` (budget total de caractères de chaînes d'une évaluation dépassé, le champ portant la limite, message « evaluation built more than 16777216 characters of strings »)

### Requirement: Plafond de taille des collections

Le système SHALL définir la constante `MAX_COLLECTION_LEN` = 100 000 (`crates/fhir-fhirpath/src/eval/helpers.rs`). Lorsqu'une collection intermédiaire dépasse cette limite, l'évaluation SHALL s'arrêter avec `Err(EvalError::CollectionTooLarge(limit))`. Le plafond SHALL être vérifié sur le résultat de chaque nœud évalué, ainsi que pour chaque item de focus dans `select`, `repeat`, `children()`, `descendants()`, la navigation de chemin et `extension(url)`, de sorte qu'une étape dépasse la limite d'au plus la sortie d'un item (ou l'autre côté d'un `combine`/union). Ce plafond borne la croissance exponentielle d'expressions courtes.

#### Scenario: Chaîne select exponentielle

- **WHEN** une expression courte enchaîne `select($this.combine($this))` jusqu'à dépasser 100 000 items (par ex. 2^20 items)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::CollectionTooLarge(100000))` sans construire la collection complète

#### Scenario: Forme aggregate

- **WHEN** `aggregate($total.combine($total), 1)` est évalué sur une entrée assez grande pour dépasser 100 000 items
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::CollectionTooLarge(100000))`

#### Scenario: Navigation sur de nombreuses copies

- **WHEN** une navigation de chemin ou `extension(url)` étend de nombreuses copies d'une ressource au-delà de 100 000 items
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::CollectionTooLarge(100000))`

#### Scenario: 2^16 items restent évalués

- **WHEN** une expression produit 2^16 (65 536) items, sous la limite
- **THEN** l'évaluateur SHALL retourner la collection normalement

#### Scenario: Entrée légitime volumineuse

- **WHEN** une entrée légitime dépasse 100 000 items (par ex. `descendants()` d'un très grand Bundle)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::CollectionTooLarge(100000))`, et le validateur SHALL la signaler par un `INVARIANT_SKIPPED`

### Requirement: Plafond de longueur des chaînes

Le système SHALL définir la constante `MAX_STRING_LEN` = 1 048 576 caractères (maximum d'une chaîne en R5 ; `crates/fhir-fhirpath/src/eval/helpers.rs`). Lorsqu'une chaîne construite par l'évaluateur dépasse cette limite, l'évaluation SHALL s'arrêter avec `Err(EvalError::StringTooLong(limit))`, où `limit` est la limite et non la longueur réelle. La longueur SHALL être comptée en caractères et non en octets ; une chaîne d'exactement 1 048 576 caractères SHALL être acceptée. Le plafond SHALL être vérifié pour la concaténation `+` et `&`, `join` et `replace` (taille calculée avant construction), `replaceMatches` (vérifié après chaque référence de groupe développée), `upper`, `lower`, `trim`, `escape`, `unescape`, `encode` et `decode`. Il SHALL NOT s'appliquer à `substring` et `split` (parties de l'entrée), à `toString` ni aux chaînes lues dans la ressource.

#### Scenario: Doublement d'une chaîne

- **WHEN** une expression double une chaîne (par ex. `aggregate($total + $total, 'a')`) jusqu'à dépasser 1 048 576 caractères
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::StringTooLong(1048576))` sans construire la chaîne complète

#### Scenario: Remplacement ou jointure trop grand

- **WHEN** `replace` ou `join` produirait une chaîne de plus de 1 048 576 caractères
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::StringTooLong(1048576))` avant de construire le résultat

#### Scenario: Expansion de groupes dans replaceMatches

- **WHEN** les références de groupe développées par `replaceMatches` font dépasser 1 048 576 caractères au résultat
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::StringTooLong(1048576))`

#### Scenario: Exactement 1 048 576 caractères

- **WHEN** une expression produit une chaîne d'exactement 1 048 576 caractères
- **THEN** l'évaluateur SHALL retourner la chaîne normalement

#### Scenario: Longueur comptée en caractères

- **WHEN** une chaîne de 1 048 576 caractères multi-octets (plus de 1 048 576 octets) est construite
- **THEN** l'évaluateur SHALL l'accepter

#### Scenario: Chaîne de la ressource non plafonnée

- **WHEN** une chaîne lue dans la ressource, ou une partie d'entrée produite par `substring`, `split` ou `toString`, dépasse 1 048 576 caractères
- **THEN** l'évaluateur SHALL NOT retourner `StringTooLong` pour cette valeur ; seule une fonction plafonnée qui la transforme (par ex. `upper`) SHALL le faire

#### Scenario: Chaîne trop longue signalée INVARIANT_SKIPPED

- **WHEN** l'évaluation d'une contrainte retourne `EvalError::StringTooLong`
- **THEN** le validateur SHALL la signaler par un `INVARIANT_SKIPPED`

### Requirement: Budget de caractères de chaînes par évaluation

Le système SHALL définir la constante `MAX_EVAL_STRING_CHARS` = 16 777 216 (`crates/fhir-fhirpath/src/eval/helpers.rs`) : le nombre total de caractères de valeurs `String` qu'une évaluation (un appel à `evaluate`/`eval`) peut construire. Le compteur SHALL être partagé par tous les évaluateurs imbriqués (`select`, `where`, `aggregate`, ...). Lorsque le total dépasse la limite, l'évaluation SHALL s'arrêter avec `Err(EvalError::StringBudgetExceeded(limit))`, où `limit` est la limite. Le budget SHALL être débité pour chaque chaîne construite par les fonctions et opérateurs de chaînes (`+` et `&`, `join`, `replace`, `replaceMatches`, `upper`, `lower`, `trim`, `escape`, `unescape`, `encode`, `decode`) ainsi que par `substring`, `split` (toutes les parties), `toString` et `toQuantity` (unité), ces derniers restant exemptés de `MAX_STRING_LEN` ; un texte que ces fonctions retournent sans le recopier (`toString` d'une `String`, `toQuantity` d'une `Quantity`) SHALL NOT être débité. Le budget SHALL NOT être débité pour les chaînes lues dans la ressource (la navigation les partage), les littéraux chaîne et quantité, les noms retournés par `type()` ni les valeurs Date/DateTime/Time (tailles fixes et petites ; les débiter par item ferait sauter de vraies invariants sur de grandes collections). Les chaînes libérées SHALL NOT être remboursées : le compte est une borne supérieure de ce que l'évaluation a alloué.

#### Scenario: Budget dépassé malgré les plafonds unitaires

- **WHEN** 32 chaînes de 1 048 576 caractères sont reconstruites par `upper()` dans une même évaluation
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::StringBudgetExceeded(16777216))`, bien que chaque chaîne soit au plus `MAX_STRING_LEN` et la collection au plus `MAX_COLLECTION_LEN`

#### Scenario: Sous le budget

- **WHEN** 8 chaînes de 1 048 576 caractères sont reconstruites par `upper()` dans une même évaluation
- **THEN** l'évaluateur SHALL retourner le résultat normalement

#### Scenario: Références à une chaîne existante non débitées

- **WHEN** 32 références à une même chaîne existante sont produites (par ex. par `combine`)
- **THEN** le budget SHALL NOT être débité et l'évaluateur SHALL retourner le résultat normalement

#### Scenario: Budget dépassé signalé INVARIANT_SKIPPED

- **WHEN** l'évaluation d'une contrainte retourne `EvalError::StringBudgetExceeded`
- **THEN** le validateur SHALL la signaler par un `INVARIANT_SKIPPED` (traitement générique d'`EvalError`)

### Requirement: Variables d'environnement

Le système SHALL analyser les constantes externes FHIRPath `%identifiant`, `%` suivi d'un identifiant délimité par accents graves, et `%'chaîne'` en un nœud `Expr::External` (nom sans le `%`), et les évaluer comme suit. Il SHALL exposer `pub struct Environment { resource: Option<&Value>, root_resource: Option<&Value>, is_subtype: Option<&IsSubtype> }` où `pub type IsSubtype = dyn Fn(&str, &str) -> bool` est construit par l'appelant à partir de `Registry::type_ancestors` (`is_subtype(nom, nom)` signifie « nom est un type FHIR ») et `evaluate_parsed_with_environment(expr, context, is_choice_variant, &Environment)`. `evaluate`, `evaluate_parsed` et `evaluate_parsed_with_choices` SHALL conserver leur signature et utiliser les valeurs par défaut.

- `%context` SHALL être la collection d'entrée d'origine, inchangée à l'intérieur de `where`, `select`, etc.
- `%ucum` = `http://unitsofmeasure.org`, `%sct` = `http://snomed.info/sct`, `%loinc` = `http://loinc.org`, `%vs-[nom]` = `http://hl7.org/fhir/ValueSet/[nom]`, `%ext-[nom]` = `http://hl7.org/fhir/StructureDefinition/[nom]` (un nom vide est une erreur).
- `%resource` et `%rootResource` (R5 fhirpath.html §2.1.9.1.5) proviennent d'`Environment`. Par défaut, `%resource` est l'entrée si c'est exactement un objet ressource (avec `resourceType`) et `%rootResource` vaut `%resource`.
- Tout autre `%nom` SHALL produire `Err(EvalError::Type(...))`.

#### Scenario: %context dans un filtre

- **WHEN** `%context` est évalué à l'intérieur de `where(...)`
- **THEN** il SHALL valoir la collection d'entrée d'origine

#### Scenario: Constantes de terminologie

- **WHEN** `%ucum`, `%sct`, `%loinc`, `%vs-x` ou `%ext-x` est évalué
- **THEN** le résultat SHALL être la chaîne d'URL correspondante ; `%ext-` sans nom SHALL produire une erreur

#### Scenario: %resource par défaut

- **WHEN** `%resource` est évalué sans `Environment` sur une entrée qui est exactement une ressource
- **THEN** il SHALL valoir cette ressource, et `%rootResource` aussi

#### Scenario: %resource indisponible

- **WHEN** `%resource` ou `%rootResource` est lu sans valeur définie et sans entrée ressource unique
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (le validateur le traduit en `INVARIANT_SKIPPED`)

#### Scenario: Variable inconnue

- **WHEN** `%inconnue` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Environment explicite

- **WHEN** `evaluate_parsed_with_environment` reçoit un `Environment` avec `resource` et `root_resource`
- **THEN** `%resource` et `%rootResource` SHALL valoir ces valeurs

### Requirement: defineVariable

Le système SHALL implémenter `defineVariable(name [, expr])`. Elle retourne sa collection d'entrée inchangée. `name` est une expression qui SHALL donner une seule `String`, sinon `Err(EvalError::Type(...))`. La valeur est `expr` évaluée sur toute la collection d'entrée, ou l'entrée elle-même sans `expr`. La variable se lit `%name` dans le reste de sa chaîne d'expression (étapes suivantes et leurs arguments, y compris les arguments itérants `where`, `select`, `aggregate`), mais pas à travers les branches d'un `|` ni hors de l'argument de fonction où elle est définie. Redéfinir une variable déjà dans la portée ou une variable système (`context`, `resource`, `rootResource`, `ucum`, `sct`, `loinc`, `vs-*`, `ext-*`) SHALL retourner `Err(EvalError::Type(...))`.

#### Scenario: Variable lue dans la suite de la chaîne

- **WHEN** `defineVariable('v', 1).select(%v + 1)` est évalué
- **THEN** le résultat SHALL être `[2]`

#### Scenario: Variable hors portée

- **WHEN** `%v` est lu après un `|` ou hors de l'argument où `defineVariable('v')` a été appelée
- **THEN** l'évaluateur SHALL retourner une erreur (variable d'environnement inconnue)

#### Scenario: Redéfinition

- **WHEN** `defineVariable('v').defineVariable('v')` ou `defineVariable('resource')` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

### Requirement: Limitations connues

Le système SHALL documenter les limitations suivantes de l'implémentation actuelle.

#### Scenario: $index hors d'une itération

- **WHEN** `$index` est évalué hors de `where`, `select`, `all`, `exists`, `repeat`, `aggregate` et de la clé de `sort` (ex. `$index`, `iif(true, $index, 0)`)
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))` (et non `UndefinedFunction`, de sorte que le validateur ne le traduise plus en `INVARIANT_SKIPPED`)

#### Scenario: $total hors d'aggregate

- **WHEN** `$total` est évalué hors de l'agrégateur d'`aggregate()`
- **THEN** l'évaluateur SHALL retourner une erreur

#### Scenario: Fonctions non implémentées retournent UndefinedFunction

- **WHEN** `resolve()`, `memberOf()`, `conformsTo()`, `today()`, `now()`, `timeOfDay()`, `lowBoundary()`, `highBoundary()`, `precision()`, `htmlChecks()` ou `comparable()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::UndefinedFunction(name))` où `name` est le nom de la fonction
- **AND** il SHALL NOT retourner de résultat deviné (collection vide, booléen), afin que le validateur signale la contrainte comme non vérifiée (`INVARIANT_SKIPPED`)

#### Scenario: Dédoublonnage hors du plafond de collection

- **WHEN** une expression déclenche le dédoublonnage O(n²) de `repeat`/`distinct`
- **THEN** `MAX_COLLECTION_LEN` SHALL NOT borner le coût CPU ; ce cas est une limitation connue

#### Scenario: Mémoire totale hors des plafonds

- **WHEN** une expression construit une collection de jusqu'à 100 000 chaînes distinctes de jusqu'à 1 048 576 caractères chacune (par ex. `select($this + big)`)
- **THEN** `MAX_COLLECTION_LEN` et `MAX_STRING_LEN` SHALL NOT borner la mémoire totale ; seul `MAX_EVAL_STRING_CHARS` la borne, pour les chaînes construites uniquement (les chaînes de la ressource et les littéraux ne sont pas débités)

### Requirement: Exécution de la suite de tests officielle FHIRPath

Le crate `fhir-fhirpath` SHALL fournir un test d'intégration exécutant `r5/fhirpath/tests-fhir-r5.xml` de `fhir-test-cases`. Chaque cas SHALL être évalué avec `evaluate_parsed_with_environment` sur son `inputfile` (passé aussi comme `%resource` ; un nom de fichier nu, sans partie répertoire, sous `r5/`, JSON ou XML analysé par `fhir-parser` puis converti par `resource_to_typed_value` avec `Registry::child_element` du Registry R5, et avec le crochet `is_subtype` issu du Registry (les éléments choix naviguent par le Registry comme dans le validateur)) ou sur un contexte vide ; un attribut `context` SHALL être évalué d'abord et son résultat sert de contexte. Un cas réussit quand les sorties correspondent en type et en valeur (ordre conservé sauf `ordered="false"` ; `predicate="true"` convertit le résultat en booléen), ou, pour une expression `invalid`, quand l'analyse ou l'évaluation échoue avec une erreur autre que `EvalError::UndefinedFunction` (une fonction absente ne prouve pas l'invalidité). Une panique, une entrée non chargeable, un `inputfile` comportant une partie répertoire ou une erreur de contexte compte comme un échec. Les cas dont le mode (du test ou du groupe) est `cda`, `tx` ou `custom` SHALL être ignorés et comptés. Le test SHALL afficher réussites/exécutés, ignorés, total et chaque cas en échec, et SHALL échouer si les nombres de cas exécutés et ignorés diffèrent exactement de (1226, 121) (dérive du chargeur ou du filtre d'ignorance) ou si moins de 969 cas réussissent (plancher de régression). Le répertoire de fixtures SHALL pouvoir être surchargé par `FHIR_FIXTURES_DIR` ; si le fichier de la suite est introuvable, le test SHALL être ignoré, sauf en CI (`CI` défini) où il SHALL échouer ; toute autre erreur de lecture de ce fichier SHALL faire échouer le test.

#### Scenario: Réussites sous le plancher

- **WHEN** moins de 948 cas de la suite réussissent
- **THEN** le test SHALL échouer

#### Scenario: Dérive du nombre de cas

- **WHEN** le nombre de cas exécutés diffère de 1226 ou le nombre de cas ignorés diffère de 121
- **THEN** le test SHALL échouer

#### Scenario: Expression invalide évaluée sans erreur

- **WHEN** un cas marqué `invalid` est analysé et évalué sans erreur
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Expression invalide utilisant une fonction non implémentée

- **WHEN** l'évaluation d'un cas marqué `invalid` échoue avec `EvalError::UndefinedFunction`
- **THEN** le cas SHALL compter comme un échec

#### Scenario: Fichier d'entrée avec une partie répertoire

- **WHEN** l'`inputfile` d'un cas n'est pas un nom de fichier nu (par exemple `../patient.json`)
- **THEN** le cas SHALL compter comme un échec sans lire le fichier

#### Scenario: Sous-module absent

- **WHEN** le fichier de la suite est introuvable
- **THEN** le test SHALL se terminer sans échec si `CI` n'est pas défini, et SHALL échouer si `CI` est défini

#### Scenario: Erreur de lecture de la suite

- **WHEN** la lecture du fichier de la suite échoue pour une raison autre que son absence
- **THEN** le test SHALL échouer

### Requirement: Navigation des éléments choix

Le système SHALL exposer `evaluate_parsed_with_choices(expr: &Expr, context: &[Value], is_choice_variant: &dyn Fn(&str, &str) -> bool) -> Result<Collection, EvalError>`. Lorsqu'un identifiant `name` est absent d'un objet, l'évaluation SHALL retourner les valeurs des champs `name` + suffixe capitalisé pour chaque suffixe tel que `is_choice_variant(name, suffix)` est vrai (FHIRPath R5 §2.1.9.1.1). Une clé explicite `name` SHALL être prioritaire et les clés commençant par `_` ne SHALL jamais correspondre. `evaluate` et `evaluate_parsed` SHALL conserver leur comportement sans navigation des choix.

#### Scenario: Le nom de base trouve la variante

- **WHEN** `value` est évalué avec `evaluate_parsed_with_choices` sur `{"valueQuantity": {...}}` et que `is_choice_variant("value", "Quantity")` est vrai
- **THEN** le résultat SHALL contenir la valeur de `valueQuantity`

#### Scenario: Clé explicite prioritaire

- **WHEN** l'objet contient à la fois `value` et `valueQuantity`
- **THEN** seul le champ `value` SHALL être retourné

#### Scenario: Suffixe non déclaré ignoré

- **WHEN** `modifier` est évalué sur un objet contenant `modifierExtension` et que `is_choice_variant("modifier", "Extension")` est faux
- **THEN** le résultat SHALL être vide

#### Scenario: Élément primitif étendu ignoré

- **WHEN** l'objet ne contient que `_valueString`
- **THEN** `value` SHALL retourner une collection vide

#### Scenario: evaluate sans navigation des choix

- **WHEN** `evaluate("value", ...)` est appelé sur `{"valueQuantity": {...}}`
- **THEN** le résultat SHALL être vide

### Requirement: Évaluation singleton

Les opérateurs et fonctions qui attendent une seule valeur SHALL appliquer l'évaluation singleton (N1 §4.5). Là où un booléen est attendu (`and`, `or`, `xor`, `implies`, `not()`), une collection d'un seul élément non booléen SHALL compter comme `true` et une collection de plusieurs éléments SHALL retourner `Err(EvalError::Type(...))`. L'arithmétique, la négation unaire, `&`, `<`, `<=`, `>`, `>=`, l'opérande gauche de `in` et l'opérande droit de `contains` SHALL retourner `Err(EvalError::Type(...))` sur plusieurs éléments. Un élément `Null` seul (valeur de remplacement) SHALL compter comme une collection vide.

#### Scenario: Élément non booléen dans un contexte booléen

- **WHEN** `42.not()` ou `false or 'foo'` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` puis `[Bool(true)]`, l'élément non booléen comptant comme `true`

#### Scenario: Plusieurs éléments dans un contexte booléen

- **WHEN** `(1 | 2).not()` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Plusieurs éléments en arithmétique ou comparaison

- **WHEN** `(1 | 2) + 1`, `-(1 | 2)`, `(1 | 2 | 3) & 'b'`, `(1 | 2) < 3`, `('a' | 'c' | 'd') in 'b'` ou `'b' contains ('a' | 'c')` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

#### Scenario: Valeur de remplacement Null

- **WHEN** un opérande est une collection contenant uniquement `Null`
- **THEN** il SHALL être traité comme une collection vide

---

### Requirement: Égalité et équivalence

L'opérateur `=` SHALL comparer deux collections élément par élément dans l'ordre ; des longueurs différentes donnent `[Bool(false)]` et une collection vide d'un côté donne une collection vide. Les objets SHALL être comparés propriété par propriété, récursivement. Un objet (valeur complexe) comparé à une primitive SHALL donner `[Bool(false)]` ; une primitive FHIR portant `id`/`extension` (`Value::Primitive`) SHALL être comparée par sa valeur.

L'opérateur `~` SHALL ne jamais retourner de collection vide : `{} ~ {}` vaut `true` et un seul côté vide vaut `false`. Il SHALL ignorer l'ordre des collections, comparer les chaînes sans tenir compte de la casse en traitant chaque caractère d'espacement comme une espace, sans fusionner les suites d'espaces, arrondir les décimaux à la précision de l'opérande le moins précis, et ignorer `id` dans les objets.

Les dates, date-heures et heures SHALL être comparées précision par précision (secondes et millisecondes comptent pour une seule précision), les fuseaux étant normalisés en UTC. Une précision différente avec des parties communes égales SHALL donner une collection vide, de même qu'un fuseau présent d'un seul côté, y compris une `Date` comparée à une `DateTime` avec fuseau. `sort()`, `min()` et `max()` SHALL continuer d'ordonner les date-heures par leur texte.

#### Scenario: Collections de longueurs différentes

- **WHEN** `(1 | 2) ~ (1 | 2 | 3)` ou `(1 | 1) = (1 | 2 | {})` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]`

#### Scenario: Collections dans un ordre différent

- **WHEN** `(1 | 2) = (2 | 1)` puis `(1 | 2 | 3) ~ (3 | 2 | 1)` sont évalués
- **THEN** les résultats SHALL être `[Bool(false)]` puis `[Bool(true)]`

#### Scenario: Équivalence avec collections vides

- **WHEN** `{} ~ {}` puis `1 ~ {}` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de chaînes

- **WHEN** `'a B\tc' ~ 'A b c'` puis `'a  b' ~ 'a b'` sont évalués
- **THEN** les résultats SHALL être `[Bool(true)]` puis `[Bool(false)]`

#### Scenario: Équivalence de décimaux

- **WHEN** `1.2 / 1.8 ~ 0.67` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]` (arrondi à la précision de `0.67`)

#### Scenario: Date-heures avec fuseaux différents

- **WHEN** `@2012-04-15T15:00:00+02:00 = @2012-04-15T16:00:00+03:00` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Secondes et millisecondes

- **WHEN** `@2012-04-15T15:30:31 = @2012-04-15T15:30:31.0` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]`

#### Scenario: Précisions différentes

- **WHEN** `@2012-01 = @2012` ou `@2012-04-15 = @2012-04-15T10:00:00` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Fuseau d'un seul côté

- **WHEN** `@2012-04-15T15:00:00Z != @2012-04-15T10:00:00` ou `@2012-04-16 = @2012-04-15T23:00:00-05:00` est évalué
- **THEN** le résultat SHALL être une collection vide

#### Scenario: Valeur complexe comparée à une primitive

- **WHEN** `name.first() = 'Peter'` puis `name.first() != 'Peter'` sont évalués sur un Patient dont `name` est un HumanName
- **THEN** les résultats SHALL être `[Bool(false)]` puis `[Bool(true)]`

### Requirement: Opérateurs booléens

`and`, `or` et `implies` SHALL suivre la logique à trois valeurs de FHIRPath. Quand un côté suffit à décider du résultat (`false and x`, `true or x`, `false implies x`, `x implies true`), une erreur produite par l'autre côté SHALL être ignorée ; le côté gauche SHALL court-circuiter l'évaluation du côté droit.

#### Scenario: Le côté gauche décide

- **WHEN** `false and (1 | 2).not()` est évalué
- **THEN** le résultat SHALL être `[Bool(false)]` sans erreur

#### Scenario: Le côté droit décide

- **WHEN** `(1 | 2).not() or true` ou `(1 | 2).not() implies true` est évalué
- **THEN** le résultat SHALL être `[Bool(true)]` sans erreur

#### Scenario: Aucun côté ne décide

- **WHEN** `(1 | 2).not() and true` ou `(1 | 2).not() xor true` est évalué
- **THEN** l'évaluateur SHALL retourner `Err(EvalError::Type(...))`

---

### Requirement: Division par zéro

`/`, `div` et `mod` SHALL retourner une collection vide quand le diviseur vaut zéro (N1 §6.6), sans erreur.

#### Scenario: Division par zéro

- **WHEN** `1 / 0`, `5 div 0`, `5 mod 0` ou `5 mod 0.0` est évalué
- **THEN** le résultat SHALL être une collection vide

### Requirement: Typage des primitives par le type déclaré

Le système SHALL exposer `resource_to_typed_value(resource, child_element: &ChildElement) -> Value`, où `ChildElement` est `dyn Fn(&str, &str) -> Option<(Arc<str>, Arc<str>)>` (chemin de définition du parent, nom de l'enfant) vers (chemin de définition, code de type). Une chaîne dont le type déclaré est `date` SHALL devenir `Value::Date`, `dateTime` et `instant` `Value::DateTime`, `time` `Value::Time` (FHIRPath R5 §2.1.9.1). Une ressource imbriquée (contained, entrée de Bundle) SHALL être typée par son propre `resourceType`. Le système SHALL aussi exposer `node_to_typed_value(value, element, child, child_element)`, qui convertit un nœud et son élément JSON `_nom` avec le même typage que `resource_to_typed_value`. `resource_to_value` et `from_parser_value` SHALL rester non typées (les dates restent des `String`).

#### Scenario: Date typée par le Registry

- **WHEN** `birthDate` est évalué sur un Patient converti par `resource_to_typed_value` avec `Registry::child_element`
- **THEN** le résultat SHALL être `[Date(...)]` et `birthDate < @2000-01-01` SHALL être comparé comme une date

#### Scenario: Conversion non typée

- **WHEN** le même Patient est converti par `resource_to_value`
- **THEN** `birthDate` SHALL être une `String`

#### Scenario: Conversion d'un nœud

- **WHEN** un nœud et son `_nom` sont convertis par `node_to_typed_value`
- **THEN** le résultat SHALL être typé comme par `resource_to_typed_value`

### Requirement: Représentation des primitives FHIR

`Value` SHALL avoir une variante `Primitive(Box<Value>, Arc<IndexMap<Arc<str>, Collection>>, Option<Arc<str>>)` portant la valeur, les enfants `id`/`extension` et le code de type FHIR (minuscule) d'une primitive FHIR. Toute primitive de type chaîne dont l'élément a un type primitif FHIR connu, `id` de ressource compris, SHALL être construite en `Primitive(valeur, enfants vides, Some(type))` par la conversion typée ; les primitives avec `id`/`extension` portent aussi ce type. En JSON, `_x` SHALL être fusionné dans la primitive `x`, appariés par indice pour les tableaux ; `_x` seul SHALL donner une primitive sans valeur (`Value::Null`) ; un `_x` qui n'est pas un objet ne SHALL rien porter. En XML, un élément de type primitif portant des enfants SHALL devenir une `Primitive` (conversion typée). La représentation scindée ne SHALL jamais être visible : aucune clé `_x` n'est navigable.

Les opérateurs et les fonctions de chaînes, mathématiques et de conversion SHALL voir la valeur ; une primitive sans valeur SHALL être traitée comme une entrée vide. La navigation, `extension()`, `children()` et `descendants()` SHALL voir `id` et `extension`.

#### Scenario: Extension d'une primitive JSON

- **WHEN** `birthDate.extension.url` est évalué sur `{"birthDate": "1970-03-30", "_birthDate": {"extension": [{"url": "u"}]}}`
- **THEN** le résultat SHALL être `['u']` et `birthDate` SHALL toujours se comparer comme sa valeur

#### Scenario: Tableau apparié par indice

- **WHEN** `given[1].extension.exists()` est évalué sur `{"given": ["a", "b"], "_given": [null, {"extension": [...]}]}`
- **THEN** le résultat SHALL être `[Bool(true)]` et `given[0].extension.exists()` SHALL être `[Bool(false)]`

#### Scenario: Primitive sans valeur

- **WHEN** `status.length()` est évalué sur `{"_status": {"extension": [...]}}`
- **THEN** le résultat SHALL être une collection vide, et `status.extension.exists()` SHALL être `[Bool(true)]`

#### Scenario: _x non objet

- **WHEN** `status.exists()` est évalué sur `{"_status": 5}`
- **THEN** le résultat SHALL être `[Bool(false)]`

