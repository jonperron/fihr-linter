# Changelog

## [1.2.0](https://github.com/jonperron/fhir-linter/compare/fhir-linter-v1.1.0...fhir-linter-v1.2.0) (2026-11-14)


### Features

* **validator:** evaluate closed, ordered and type slicing ([4617069](https://github.com/jonperron/fhir-linter/commit/4617069c501498ab6c75f49db95de24de29b396d))
* **validator:** evaluate closed, ordered and type slicing ([#104](https://github.com/jonperron/fhir-linter/issues/104)) ([0b0e202](https://github.com/jonperron/fhir-linter/commit/0b0e202f5e0db3e10f16771e8d9134a60d24a161))
* **validator:** evaluate invariants with fhir types and resources ([e92c16b](https://github.com/jonperron/fhir-linter/commit/e92c16b1fd2dbc2025c77bf9d3d60baddbb6faf7))
* **validator:** evaluate invariants with fhir types and resources ([#101](https://github.com/jonperron/fhir-linter/issues/101)) ([35e5c8c](https://github.com/jonperron/fhir-linter/commit/35e5c8cb9e8ffe02d0e40071ff2da1bb7408bfe2))


### Bug Fixes

* **fhirpath:** compare fhir quantities as quantities ([aa5f07f](https://github.com/jonperron/fhir-linter/commit/aa5f07f6772aa564bdcb09560d48d4d5487244eb))
* **fhirpath:** compare fhir quantities as quantities ([#103](https://github.com/jonperron/fhir-linter/issues/103)) ([07915e6](https://github.com/jonperron/fhir-linter/commit/07915e6fdfc1b7779f4bb321034e4ac7be52ff82))
* **fhirpath:** fall back to unit text for an extension-only quantity code ([95bd1d1](https://github.com/jonperron/fhir-linter/commit/95bd1d1cf8f3d05b0cc9cde012a22ff04b8d2672))
* **fhirpath:** implement definevariable and reject unknown type names ([215749c](https://github.com/jonperron/fhir-linter/commit/215749cc104b5835bf49d3cef783af4e1814b024))
* **fhirpath:** implement defineVariable and reject unknown type names ([#100](https://github.com/jonperron/fhir-linter/issues/100)) ([b95e98d](https://github.com/jonperron/fhir-linter/commit/b95e98d50a5ad995bb8e98f899cbcbac0d6d82f3))
* **fhirpath:** keep variables across indexers and check empty types ([5d53637](https://github.com/jonperron/fhir-linter/commit/5d536376528dbf92563d80a01568d0f0b6170ff7))
* **validator:** report xml element order and trimmed resource ids ([46d3355](https://github.com/jonperron/fhir-linter/commit/46d335511d6b5041b6046c9010d610acdaa97d65))
* **validator:** report xml element order and trimmed resource ids ([#105](https://github.com/jonperron/fhir-linter/issues/105)) ([e31b350](https://github.com/jonperron/fhir-linter/commit/e31b350fcd93bcf27774a59125cb8b861e95f6ca))
* **validator:** skip attribute elements in the xml order check ([658f066](https://github.com/jonperron/fhir-linter/commit/658f066bec2c5a55536c6b79131b2e97a3a2c3e1))

## [1.1.0](https://github.com/jonperron/fhir-linter/compare/fhir-linter-v1.0.0...fhir-linter-v1.1.0) (2026-11-14)


### Features

* **definitions:** load basedefinition and add type ancestry lookup ([ea34b0b](https://github.com/jonperron/fhir-linter/commit/ea34b0b6e7e72633c16e9dd94d8715ef0179c773))
* **definitions:** load basedefinition and add type ancestry lookup ([#96](https://github.com/jonperron/fhir-linter/issues/96)) ([7c3532a](https://github.com/jonperron/fhir-linter/commit/7c3532aac62256efbb85738dcd877f20f6803773))
* **definitions:** load the r5 regex of each primitive type ([fdf7883](https://github.com/jonperron/fhir-linter/commit/fdf7883d7187554100af43fcd45cb7c260de71fc))
* **definitions:** load the r5 regex of each primitive type ([#88](https://github.com/jonperron/fhir-linter/issues/88)) ([0dd66a6](https://github.com/jonperron/fhir-linter/commit/0dd66a6d7ccd5b18ddbe722ee1179192ac1eecea))
* **fhirpath:** add the %resource, %rootResource and %context variables ([baec90a](https://github.com/jonperron/fhir-linter/commit/baec90a190eee446897b9d97416229e4d243a02d))
* **fhirpath:** add the %resource, %rootResource and %context variables ([#94](https://github.com/jonperron/fhir-linter/issues/94)) ([dd79c8c](https://github.com/jonperron/fhir-linter/commit/dd79c8c64da894936ec8760d44584cac28593dfb))
* **fhirpath:** answer is/as/ofType through FHIR type inheritance ([a18a563](https://github.com/jonperron/fhir-linter/commit/a18a5636f6bed385b1142ad8fd47e38bef6deb24))
* **fhirpath:** answer is/as/ofType through FHIR type inheritance ([#98](https://github.com/jonperron/fhir-linter/issues/98)) ([ff4fc1e](https://github.com/jonperron/fhir-linter/commit/ff4fc1e34974ac94f1b5510e0bf3d0491d457965))
* **fhirpath:** refuse fhir string subtypes in is, as and oftype ([c784fc3](https://github.com/jonperron/fhir-linter/commit/c784fc390b3d5875b206483bd274bc08543b5fb1))


### Bug Fixes

* **cli:** cap input size and escape paths in tool errors ([3747c40](https://github.com/jonperron/fhir-linter/commit/3747c40c3649ba71e76031b50aa937c73d80549b))
* **cli:** cap input size and escape paths in tool errors ([#86](https://github.com/jonperron/fhir-linter/issues/86)) ([cc70b46](https://github.com/jonperron/fhir-linter/commit/cc70b4679ac35e66b4bd1cd591352a9ecbcf5aae))
* **definitions:** answer the review of the type ancestry lookup ([e6efcb9](https://github.com/jonperron/fhir-linter/commit/e6efcb962fb1653a849b2623c8687e0b433ba70e))
* **fhirpath:** answer the review of the environment variables ([bfc3298](https://github.com/jonperron/fhir-linter/commit/bfc329878e7b133b6ba953af607adb44f2a73868))
* **fhirpath:** cap intermediate collections at 100,000 items ([1cbc56d](https://github.com/jonperron/fhir-linter/commit/1cbc56de1913bae2c605001a8957514b759dc59e))
* **fhirpath:** cap string results at 1,048,576 characters ([33bbebb](https://github.com/jonperron/fhir-linter/commit/33bbebb3c3a1ee17e6adf70c6ba48e5993182a36))
* **fhirpath:** cap string results at 1,048,576 characters ([#87](https://github.com/jonperron/fhir-linter/issues/87)) ([001dc1f](https://github.com/jonperron/fhir-linter/commit/001dc1fb6003f20c061284637e6caf2cab5b81fc))
* **fhirpath:** cap the characters of strings one evaluation builds ([c9121f6](https://github.com/jonperron/fhir-linter/commit/c9121f6cc554e652a6a6b73881c238e4fd8fa0e4))
* **fhirpath:** cap the characters of strings one evaluation builds ([#91](https://github.com/jonperron/fhir-linter/issues/91)) ([ac83790](https://github.com/jonperron/fhir-linter/commit/ac83790605c607b308f86132bf7b0d2e9911e57e))
* **fhirpath:** carry the FHIR type on boolean and numeric primitives ([5814078](https://github.com/jonperron/fhir-linter/commit/5814078d9e8694ca7ce190e4c7a1cf9edb69dffb))
* **fhirpath:** charge only text a conversion newly builds ([d893a84](https://github.com/jonperron/fhir-linter/commit/d893a846ad122cf2c52c08ba8b2232a6fcc02969))
* **fhirpath:** check the collection cap in extension(url) ([b60f48c](https://github.com/jonperron/fhir-linter/commit/b60f48cc478b2ff8ccc7ba077dcdfe755b02dff7))
* **fhirpath:** check the collection cap while navigating ([3005ca3](https://github.com/jonperron/fhir-linter/commit/3005ca3b75a195fc4fcb282568e2c73b2f398edc))
* **fhirpath:** convert quantities, longs and decimal strings per n1 ([0a79466](https://github.com/jonperron/fhir-linter/commit/0a7946678308543dade88a5df7d2c50eb356d925))
* **fhirpath:** do not convert digit strings beyond f64 to infinity ([098ae58](https://github.com/jonperron/fhir-linter/commit/098ae58e152d08d69d7fd8c51b040fb310861042))
* **fhirpath:** give empty from skip and take with an empty count ([49a2ee2](https://github.com/jonperron/fhir-linter/commit/49a2ee20d18ead70bb213d15e02a649be3374d66))
* **fhirpath:** implement $index in iterating functions ([06c2627](https://github.com/jonperron/fhir-linter/commit/06c2627f67ea1d6c4cb6c6ab1171aea7918c7de7))
* **fhirpath:** index sort keys and keep infinite decimals unconverted ([16136d6](https://github.com/jonperron/fhir-linter/commit/16136d6fd463c7b5a8149f9f6dcb7346366516a5))
* **fhirpath:** n1 follow-ups (skip/take, string singletons, conversions, dates, $total, $index) ([#83](https://github.com/jonperron/fhir-linter/issues/83)) ([2facb9a](https://github.com/jonperron/fhir-linter/commit/2facb9a6d3288d81333d0579451c82a70e9a361b))
* **fhirpath:** reject date literals that are not calendar dates ([b77b2f5](https://github.com/jonperron/fhir-linter/commit/b77b2f53b16989e133d2bf9d238adc91ad34cd79))
* **fhirpath:** reject several or non-string items in string functions ([816bee0](https://github.com/jonperron/fhir-linter/commit/816bee0d3f020f4260d30633312fbfd50f7234fb))
* **fhirpath:** split replaceMatches replacements in linear time ([36769e9](https://github.com/jonperron/fhir-linter/commit/36769e937781736392ae6a7d747e3073fe00483f))
* **fhirpath:** support $total in aggregate() ([bc8af78](https://github.com/jonperron/fhir-linter/commit/bc8af78e583f5767fe6f0e75f7858480975b1cf4))
* **parser:** accept an xml resource id with an extension ([9e5277d](https://github.com/jonperron/fhir-linter/commit/9e5277d4f569705d77ffc435786b6ac770058129))
* **parser:** accept an xml resource id with an extension ([#89](https://github.com/jonperron/fhir-linter/issues/89)) ([5ca4bd4](https://github.com/jonperron/fhir-linter/commit/5ca4bd4c73a5c95c6dddf5837051ff52ec1b91c6))
* **validator:** accept null placeholders and warn on profile versions ([79d15c6](https://github.com/jonperron/fhir-linter/commit/79d15c65d99e96c83d90a13047b4b61c8f94085d))
* **validator:** accept null placeholders and warn on profile versions ([#95](https://github.com/jonperron/fhir-linter/issues/95)) ([10e404f](https://github.com/jonperron/fhir-linter/commit/10e404f4e224119b57f9e932e71a976d187dba57))
* **validator:** check primitive formats with the r5 regexes ([713a792](https://github.com/jonperron/fhir-linter/commit/713a7922e8c74cac972d58f997c0eb4bc883b24d))
* **validator:** check primitive formats with the r5 regexes ([#93](https://github.com/jonperron/fhir-linter/issues/93)) ([65b281b](https://github.com/jonperron/fhir-linter/commit/65b281b2ce243fee2bae1a80e76a80e8233ea459))
* **validator:** check reference target types and uncoded codings ([c1e3103](https://github.com/jonperron/fhir-linter/commit/c1e3103cd21f2e9ab24c0510a1bc914fb6c4b9f8))
* **validator:** check reference target types and uncoded codings ([#97](https://github.com/jonperron/fhir-linter/issues/97)) ([79dbf21](https://github.com/jonperron/fhir-linter/commit/79dbf21e0052babe61f644f6d6531ca45052c9a8))
* **validator:** check reference targets through the type ancestry ([d6e07ef](https://github.com/jonperron/fhir-linter/commit/d6e07efda36cc03fb20b417433e17c966682c387))
* **validator:** enforce fixed, pattern and type profiles ([a9ac2c0](https://github.com/jonperron/fhir-linter/commit/a9ac2c0aaaaabfbbb6ab304415c7a09dd83f0f0b))
* **validator:** enforce fixed, pattern and type profiles ([#99](https://github.com/jonperron/fhir-linter/issues/99)) ([1bbb8e3](https://github.com/jonperron/fhir-linter/commit/1bbb8e3eaa3382bbd792c4af18994f6d2762cd58))
* **validator:** limit the whitespace warning to regex-checked types ([b06061d](https://github.com/jonperron/fhir-linter/commit/b06061d7d838585ccecc186faddbe0fd4e312863))
* **validator:** pass the path argument in the string-cap test ([7529570](https://github.com/jonperron/fhir-linter/commit/75295707e24fe2c19c7273683a61de0a513147d0))
* **validator:** pass the path argument in the string-cap test ([#92](https://github.com/jonperron/fhir-linter/issues/92)) ([518574e](https://github.com/jonperron/fhir-linter/commit/518574e5f6897855d2ff4b5f9054259bddb651e9))
* **validator:** reject empty placeholders and ignore version fragments ([bb5ad98](https://github.com/jonperron/fhir-linter/commit/bb5ad98711d89eeb71837465419f605347b0d10f))
* **validator:** share the container ids of contained resources ([fcbcb23](https://github.com/jonperron/fhir-linter/commit/fcbcb2397d37d344905cf252a1c5d624a095c140))
* **validator:** walk contentReference, contained and Bundle resources ([2ddf65a](https://github.com/jonperron/fhir-linter/commit/2ddf65ac02d70ab19ea26c6c01fb74be7bcaf4c8))
* **validator:** walk contentReference, contained and Bundle resources ([#90](https://github.com/jonperron/fhir-linter/issues/90)) ([549cd87](https://github.com/jonperron/fhir-linter/commit/549cd8722482220a4bff4b7982a04abf7ed7347b))
* **validator:** warn on whitespace-only strings ([8388f8a](https://github.com/jonperron/fhir-linter/commit/8388f8aae0a1c58a14e5c6386b38ff5ba9db6a1a))

## [1.0.0](https://github.com/jonperron/fhir-linter/compare/fhir-linter-v0.5.2...fhir-linter-v1.0.0) (2026-11-12)


### ⚠ BREAKING CHANGES

* **linter:** diagnostics from the Python and Node bindings now report severity "information" where they reported "hint", and "fatal" is a new possible severity value. "hint" is still accepted in the lint configuration.

### Features

* **cli:** implement the fhir-lint command-line interface ([2e01e53](https://github.com/jonperron/fhir-linter/commit/2e01e53adb59e2156fa082b06281025e3fef58bd))
* **cli:** implement the fhir-lint command-line interface ([#58](https://github.com/jonperron/fhir-linter/issues/58)) ([fd4d7d2](https://github.com/jonperron/fhir-linter/commit/fd4d7d2fcacfa61cb7099a903bc5bd2928e98261))
* **definitions:** load additional bindings and value limits ([4d62589](https://github.com/jonperron/fhir-linter/commit/4d625898b694c31aae80599f7ab9745ca88ba0f5))
* **definitions:** load contentReference on snapshot elements ([46a4818](https://github.com/jonperron/fhir-linter/commit/46a481843cc0f1bf6961b6bc51c96a5349fcfb64))
* **definitions:** load contentReference, type profiles, fixed/pattern, slicing and value limits ([#82](https://github.com/jonperron/fhir-linter/issues/82)) ([4f0dbcc](https://github.com/jonperron/fhir-linter/commit/4f0dbcc2a046a3957f8d1221a04ce56ddfda9e12))
* **definitions:** load element slicing definitions ([85e07de](https://github.com/jonperron/fhir-linter/commit/85e07deb94d0b340c802a6e753ca764763cf2e23))
* **definitions:** load fixed[x] and pattern[x] values ([6b6e1dd](https://github.com/jonperron/fhir-linter/commit/6b6e1ddb2fffed1ffb9735911da85e7c751ce7e1))
* **definitions:** load type.profile per element type ([219c200](https://github.com/jonperron/fhir-linter/commit/219c200d36392388aced6881b8dca0e6509370f9))
* **definitions:** load type.targetProfile per element type ([dbe6aff](https://github.com/jonperron/fhir-linter/commit/dbe6aff4147dfa6b46668dfa8cb15c0685124d44))


### Bug Fixes

* **cli:** address review findings on inputs, config order and output ([5a3f71a](https://github.com/jonperron/fhir-linter/commit/5a3f71a5020c91e5243b702ebd6620fdb2825fa5))
* **cli:** use fatal severity in docs and cover it with a test ([1693dda](https://github.com/jonperron/fhir-linter/commit/1693dda4e43a01c7eeb9a9df6fd4eacd6d871c4a))
* **definitions:** detect notSelectable by its property URI ([ae89a70](https://github.com/jonperron/fhir-linter/commit/ae89a709021ad1b1fadcf4df4f84116082e54d04))
* **definitions:** honour include.version in ValueSet membership ([d0a6f04](https://github.com/jonperron/fhir-linter/commit/d0a6f04c5abff07d1ef1286c17c6e92142e003b5))
* **definitions:** registry audit items A6-A8, D6, D7, D10 ([#66](https://github.com/jonperron/fhir-linter/issues/66)) ([961c81c](https://github.com/jonperron/fhir-linter/commit/961c81c41881229f6b4f73c045b8be30373ad68a))
* **definitions:** reject snapshot elements without path or max ([1bdd68f](https://github.com/jonperron/fhir-linter/commit/1bdd68f95331d0ccfd0d63d10a3885a2d543a34c))
* **definitions:** treat notSelectable concepts as undecidable ([d8939a0](https://github.com/jonperron/fhir-linter/commit/d8939a0d463dde76f4507aa8dcdd4cd19dd2ffdb))
* **fhirpath:** accept double-quoted string literals ([f92c1ab](https://github.com/jonperron/fhir-linter/commit/f92c1ab29709b9474a18e91177da91d3aa8bb0c3))
* **fhirpath:** accept double-quoted string literals ([#81](https://github.com/jonperron/fhir-linter/issues/81)) ([7885622](https://github.com/jonperron/fhir-linter/commit/78856225a9c564912380384db6b71e6cf5c23bcf))
* **fhirpath:** accept leap seconds and clamp substring length ([3cbe1b9](https://github.com/jonperron/fhir-linter/commit/3cbe1b980992eb79c873df709635db774620a802))
* **fhirpath:** apply singleton evaluation and boolean criteria rules ([4c2367d](https://github.com/jonperron/fhir-linter/commit/4c2367d82bb23ebada3f1101347db2df4e6e1c13))
* **fhirpath:** check ordering operands before empty and drop lone nulls ([9ca7d43](https://github.com/jonperron/fhir-linter/commit/9ca7d43d46c5bf415767f2fc713d8e92a1cceb34))
* **fhirpath:** conversion functions follow fhirpath n1 ([67bfe53](https://github.com/jonperron/fhir-linter/commit/67bfe53d3fdd06bb9e717bbcd4a62c8f83528d81))
* **fhirpath:** dedupe both operands of union and intersect ([ce160fb](https://github.com/jonperron/fhir-linter/commit/ce160fba7c5a9196f09929171059a216c31dea49))
* **fhirpath:** fhir data model mapping (F10) ([#79](https://github.com/jonperron/fhir-linter/issues/79)) ([870d3ae](https://github.com/jonperron/fhir-linter/commit/870d3ae9868558b7c55db0ff3feeddb080ded776))
* **fhirpath:** follow the spec for = and ~ on collections and dates ([176418c](https://github.com/jonperron/fhir-linter/commit/176418cfb992669c777a6b0067b1be86f74c31d2))
* **fhirpath:** give empty when a zone makes a date comparison unknown ([ed83bb1](https://github.com/jonperron/fhir-linter/commit/ed83bb1cf0c99d7b2ccb7831522a2c2b002e976c))
* **fhirpath:** give hasValue() empty unless one primitive has a value ([e25c1c3](https://github.com/jonperron/fhir-linter/commit/e25c1c3521ad8db267f47430b54f3e85ff8b27d9))
* **fhirpath:** hide the split representation of fhir primitives ([c129ed9](https://github.com/jonperron/fhir-linter/commit/c129ed99522cc2d0cde2b526427e03e7da1048e8))
* **fhirpath:** keep resourceType and json nulls out of the item tree ([08de71a](https://github.com/jonperron/fhir-linter/commit/08de71a401e011ae8b600e7869963e9d63c497c0))
* **fhirpath:** keep sort order total and string ~ length-preserving ([316f0ae](https://github.com/jonperron/fhir-linter/commit/316f0ae6b276ecd9d1b820fc283b09e7451f7b1e))
* **fhirpath:** let the deciding side of and, or and implies win ([e633cdb](https://github.com/jonperron/fhir-linter/commit/e633cdb4a62a07469343207736124c24bda0a755))
* **fhirpath:** make the as operator follow as() on several items ([2f1b189](https://github.com/jonperron/fhir-linter/commit/2f1b189ce6028e5d6c02316341f95a533c1cb98d))
* **fhirpath:** n1 string functions and conversions (F8, F9) ([#78](https://github.com/jonperron/fhir-linter/issues/78)) ([7d8c10b](https://github.com/jonperron/fhir-linter/commit/7d8c10b51cc460afba6545410716425b9b0a3808))
* **fhirpath:** operator semantics (F3, F4, F6, F7, and/or errors, as operator) ([#77](https://github.com/jonperron/fhir-linter/issues/77)) ([b707452](https://github.com/jonperron/fhir-linter/commit/b70745299cb80161c9ff2672808913f68c3a18c4))
* **fhirpath:** return empty on division by zero ([0d9a61e](https://github.com/jonperron/fhir-linter/commit/0d9a61e6bb54cf19696cae1ffa2ad5e2022a50d3))
* **fhirpath:** return false from hasValue() when there is no value ([e196f6a](https://github.com/jonperron/fhir-linter/commit/e196f6a81f902d7c4c66c3eb4b9df1e1ba5ae49f))
* **fhirpath:** skip value-less primitives in aggregates and getValue ([387a86d](https://github.com/jonperron/fhir-linter/commit/387a86d35cd1aa632f4cd145de71f17d5c3a8d46))
* **fhirpath:** string functions return empty on empty input ([38efb8a](https://github.com/jonperron/fhir-linter/commit/38efb8a235facb7a274b5f3bea3d6b3d241b593a))
* **fhirpath:** treat a date against a zoned date-time as one-sided zone ([855866a](https://github.com/jonperron/fhir-linter/commit/855866a3585cd84dfd5055e00361f562b6d5ef7c))
* **fhirpath:** type fhir temporals by their declared element type ([9e4ec89](https://github.com/jonperron/fhir-linter/commit/9e4ec899a8d89e6bd074ae041a5197bb78354993))
* **linter:** use fhir issueseverity codes for diagnostic severities ([efcf6e5](https://github.com/jonperron/fhir-linter/commit/efcf6e5e40820103cc91bfd6b668c51d5e6e01ac))
* **linter:** use fhir issueseverity codes for diagnostic severities ([#74](https://github.com/jonperron/fhir-linter/issues/74)) ([28a4e14](https://github.com/jonperron/fhir-linter/commit/28a4e14f2e39485c040ccb865a004908fb2b4c0e))
* **parser:** bound xml namespace lookups and the div depth ([02ee7ca](https://github.com/jonperron/fhir-linter/commit/02ee7caf3a14ae8a6ffd609c39fd6085f8628789))
* **parser:** check the xml attribute or element form of each element ([63b48b1](https://github.com/jonperron/fhir-linter/commit/63b48b1975f050ca836c0916c3f6549c123ad124))
* **parser:** check xml namespaces with quick-xml's nsreader ([3c69c5f](https://github.com/jonperron/fhir-linter/commit/3c69c5f2ab26475d3b8a5607ad407cef3d45a827))
* **parser:** keep rejecting control references in the raw div ([f945b40](https://github.com/jonperron/fhir-linter/commit/f945b4078caab7158d60d477c93e077a3eab7ed8))
* **parser:** keep the xml narrative div as its raw markup ([ac117ea](https://github.com/jonperron/fhir-linter/commit/ac117ea891b64d59400795f31e4515dd3b6855f8))
* **parser:** reject a dtd anywhere and malformed xhtml attributes ([07a3a97](https://github.com/jonperron/fhir-linter/commit/07a3a97d1d3dc2b931e1764d51085070ebe076d2))
* **parser:** reject a DTD in fhir xml ([859b7c6](https://github.com/jonperron/fhir-linter/commit/859b7c6b05b9714aea2eac842321e42d52226aad))
* **parser:** reject a non-utf-8 xml encoding declaration ([028ef5e](https://github.com/jonperron/fhir-linter/commit/028ef5e6c56ba964630cfdd63aa0976f50c5ed07))
* **parser:** reject attributes on the xml resource root ([9e7f2b2](https://github.com/jonperron/fhir-linter/commit/9e7f2b28d5bee13888d4517eb9741c282ccc5e48))
* **parser:** reject text and cdata inside fhir xml elements ([8756f3f](https://github.com/jonperron/fhir-linter/commit/8756f3ff93b6333758e67cc3bbf446c1448a5e24))
* **parser:** report the xml schema instance namespace as structure_003 ([5c6ce6a](https://github.com/jonperron/fhir-linter/commit/5c6ce6a1f361a774f9d0c5052d1101142eefc1a3))
* **parser:** trim xml attribute values on read and warn ([58ec325](https://github.com/jonperron/fhir-linter/commit/58ec32507b0be83a426ee5b5d0dcbf82a882599e))
* **parser:** xml conformance checks from the r5 xml page ([#71](https://github.com/jonperron/fhir-linter/issues/71)) ([aaa841a](https://github.com/jonperron/fhir-linter/commit/aaa841a9af07e3066f3de2120a2c41620428260e))
* **validator:** pair root _id with the lifted id, convert once ([5d62ec5](https://github.com/jonperron/fhir-linter/commit/5d62ec593a01c523af9a745e921db18f33883be2))
* **validator:** report trimmed xml attribute values as type_007 ([5bd52dc](https://github.com/jonperron/fhir-linter/commit/5bd52dccbfe6014f6f9bebcde3bfee107ccd721f))
* **validator:** run invariants, terminology and references at every depth ([7c6d7e1](https://github.com/jonperron/fhir-linter/commit/7c6d7e127e68754e5f3cf6e2456b7937286f8b22))
* **validator:** run invariants, terminology and references at every depth ([#80](https://github.com/jonperron/fhir-linter/issues/80)) ([2762841](https://github.com/jonperron/fhir-linter/commit/276284192cb81adceee8a2cc8c5a288d5da1dcee))

## [0.5.2](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.5.1...fhir-linter-v0.5.2) (2026-11-11)


### Bug Fixes

* **ci:** restrict publishing to main and keep queued retries ([aee7dcb](https://github.com/jonperron/fihr-linter/commit/aee7dcb1c37c1222a8d5ce770927964306475d58))
* **fhirpath:** navigate choice elements by their base name ([2414ded](https://github.com/jonperron/fihr-linter/commit/2414ded6b310782c04a10daa757bcb2e8377a90c))
* **fhirpath:** return undefined function for unimplemented stubs ([650df9b](https://github.com/jonperron/fihr-linter/commit/650df9bc43abfc1a8945eb12b707e0bd1fbc9812))
* **fhirpath:** route the is operator through the is() function logic ([aaf64c3](https://github.com/jonperron/fihr-linter/commit/aaf64c3f6c3d590b7487571c321bf79dc1726a07))
* **fhirpath:** undefined stubs, choice navigation and is operator ([#65](https://github.com/jonperron/fihr-linter/issues/65)) ([b995921](https://github.com/jonperron/fihr-linter/commit/b9959211aa61b11c96e6d8af558acd9b311e11f8))
* **parser:** accept a leading utf-8 bom in json ([b197375](https://github.com/jonperron/fihr-linter/commit/b1973753ccf06af8222079331ba977963be51e86))
* **parser:** flag control characters in json and xml strings ([6b48955](https://github.com/jonperron/fihr-linter/commit/6b4895513d00d1141a49afcc136519a3da8b7187))
* **parser:** json -0, control characters, utf-8 bom and byte offsets ([#68](https://github.com/jonperron/fihr-linter/issues/68)) ([48560a5](https://github.com/jonperron/fihr-linter/commit/48560a5d027d46a0c3869cddf4942017b0ba8c05))
* **parser:** keep a literal json -0 as a decimal ([255923d](https://github.com/jonperron/fihr-linter/commit/255923d1163b3a1f029cf8432013ed221b2cdd6d))
* **parser:** reject literal xml control characters anywhere ([61b6dd6](https://github.com/jonperron/fihr-linter/commit/61b6dd6a69c5175109d25a83ffeac2645ad46a32))
* **release:** publish to registries from the top-level workflow ([cd9aac0](https://github.com/jonperron/fihr-linter/commit/cd9aac0675165f250c6884003e3eaa30c91c808e))
* **release:** publish to registries from the top-level workflow ([#69](https://github.com/jonperron/fihr-linter/issues/69)) ([d4cb432](https://github.com/jonperron/fihr-linter/commit/d4cb4329f863de8aff8c84931c552bec775253c2))
* **validator:** check extensible bindings as warnings ([195bcca](https://github.com/jonperron/fihr-linter/commit/195bccadf2b5b93493022e95d6c239b347528098))
* **validator:** clearer text-only terminology message ([361425d](https://github.com/jonperron/fihr-linter/commit/361425d438454ad69c63318ff5c4bcec6389c1fe))
* **validator:** compare reference type with a fhir url reference ([ea7d305](https://github.com/jonperron/fihr-linter/commit/ea7d305cf9cd50ab3eae6efe3da1d4c8442887dc))
* **validator:** empty arrays, code format and date-only zones ([ce141b8](https://github.com/jonperron/fihr-linter/commit/ce141b8379f06df9a5556104a73bfde5c675d2a1))
* **validator:** follow the r5 regexes for code and dateTime zones ([bdb10d4](https://github.com/jonperron/fihr-linter/commit/bdb10d4626c80ccfc654accc505eda93cd0250c4))
* **validator:** read a single xml coding in terminology checks ([02fd4e6](https://github.com/jonperron/fihr-linter/commit/02fd4e61a5d01aa9bb50df4ce8855f154e864b3a))
* **validator:** reject a bare hash reference at the root ([a80a9bc](https://github.com/jonperron/fihr-linter/commit/a80a9bc752f76f5596e6efd4122dc0d825445f25))
* **validator:** reject text-only codeable concept under required binding ([62accfa](https://github.com/jonperron/fihr-linter/commit/62accfa4a16eae1e3247a5b38f2cde6665855b6f))
* **validator:** terminology and reference audit fixes (V2, V4, V5, V7, V11) ([#70](https://github.com/jonperron/fihr-linter/issues/70)) ([1b8122e](https://github.com/jonperron/fihr-linter/commit/1b8122e765ccf95c2c7b979e16a5242243bb7c0d))
* **validator:** treat a coding without system as not a member ([87b88a4](https://github.com/jonperron/fihr-linter/commit/87b88a46f9748d2b66b04e409f2fe9bc49e6c9f3))
* **validator:** treat empty or all-null arrays as absent ([7b69847](https://github.com/jonperron/fihr-linter/commit/7b69847a4d7f4e3022405bd07c4fa81baa315862))
* **validator:** treat only a uri scheme as an absolute reference ([36eaf40](https://github.com/jonperron/fihr-linter/commit/36eaf400267a33c744f117053c4d7adaa255993f))
* **validator:** warn on a malformed relative reference ([51097eb](https://github.com/jonperron/fihr-linter/commit/51097eba12a7a4f034fb4e8a9dd7b65e00588c6a))
* **validator:** warn on control characters in the resource id ([610de3f](https://github.com/jonperron/fihr-linter/commit/610de3f8d0c447e793a13e309dead2c77492d158))

## [0.5.1](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.5.0...fhir-linter-v0.5.1) (2026-11-10)


### Bug Fixes

* **linter:** close narrative, identifier and profile rule gaps ([83fb381](https://github.com/jonperron/fihr-linter/commit/83fb381c310e4653f7d94f21403d33549048c617))
* **linter:** limit bundle missing-identifier to persistent types ([3992212](https://github.com/jonperron/fihr-linter/commit/3992212ad5dc6ba94adc4028339d54f95c3dacae))
* **linter:** locate identifier and profile findings at meta ([bdf8094](https://github.com/jonperron/fihr-linter/commit/bdf8094537f91794ff4c7f99523e26bb1c1220ec))
* **linter:** report narrative with status empty as absent ([3b8eac5](https://github.com/jonperron/fihr-linter/commit/3b8eac5bca0d959eea1deca52fe2e6f3e256b09e))
* **linter:** scan narrative xhtml properly and read xml primitives ([5e80ae3](https://github.com/jonperron/fihr-linter/commit/5e80ae37c2cb743c3a86b92524dce7dd67839b63))
* **linter:** treat a markup-only narrative div as empty ([a06b2bd](https://github.com/jonperron/fihr-linter/commit/a06b2bd887c5123961bbf5d0113929a837337026))
* **linter:** treat empty identifier and profile arrays as missing ([fb208f5](https://github.com/jonperron/fihr-linter/commit/fb208f55c680637b69074c12c2dc405ba461656c))
* **release:** publish npm to github packages and python to pypi ([c4fb6e6](https://github.com/jonperron/fihr-linter/commit/c4fb6e6afa3284b0780cd7a728ec16d3aff6443c))
* **release:** publish npm to github packages and python to pypi ([5fd6151](https://github.com/jonperron/fihr-linter/commit/5fd615176ddffc634bbcf83b426d5e97f6e3ec94))

## [0.5.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.4.1...fhir-linter-v0.5.0) (2026-11-09)


### Features

* **fhirpath:** parse backtick-delimited identifiers ([05baada](https://github.com/jonperron/fihr-linter/commit/05baada7c82f1ed4dfb60dcf47132ae8b301c1ea))
* **fhirpath:** parse backtick-delimited identifiers ([9610844](https://github.com/jonperron/fihr-linter/commit/9610844ffb53099152ef5c54bc8e92682153abec))
* **release:** attach cli, wheels and npm tarball to github releases ([bca1c64](https://github.com/jonperron/fihr-linter/commit/bca1c649470bf28d5cabba981c25aa3679cc580d))


### Bug Fixes

* **ci:** match crate Cargo.toml files in ci path filters ([8535218](https://github.com/jonperron/fihr-linter/commit/85352187698d16edf51fdeb393a53cf13e50b453))
* **ci:** match crate Cargo.toml files in ci path filters ([bfe82b9](https://github.com/jonperron/fihr-linter/commit/bfe82b9b84d35dc730cab3d28359908172ffa2c5))
* **ci:** pin release-please action to a commit sha ([7f044c4](https://github.com/jonperron/fihr-linter/commit/7f044c40c353bc31cc31941c6e1b472e9b600274))
* **ci:** pin release-please action to a commit sha ([04b74c6](https://github.com/jonperron/fihr-linter/commit/04b74c67d597900bd6b4d92a4e0932f3b0e5353f))
* **fhirpath:** add is() and as() function forms ([e9194d3](https://github.com/jonperron/fihr-linter/commit/e9194d3e2bf4d51b62a15c7cd651b7491bfec155))
* **fhirpath:** add is() and as() function forms ([e444051](https://github.com/jonperron/fihr-linter/commit/e444051850a91db4f0fd3756edf93c8fad1f86e6))
* **fhirpath:** allow expressions to start with the context type name ([5cb716f](https://github.com/jonperron/fihr-linter/commit/5cb716f9ef2cd4e1e51b7114f8cf93112ca27c68))
* **fhirpath:** allow expressions to start with the context type name ([174ecb0](https://github.com/jonperron/fihr-linter/commit/174ecb071a1a8e4bbb7db4e3043dfab913bafc9b))
* **fhirpath:** restrict type name prefix to the first path segment ([84c4e35](https://github.com/jonperron/fihr-linter/commit/84c4e35084aaf2c1f17c24bf220b9246992e0d33))
* **fhirpath:** return empty collection for out-of-range index ([d193b62](https://github.com/jonperron/fihr-linter/commit/d193b628d42c4b58311479d7e0e27acc8d2a7097))
* **fhirpath:** return empty collection for out-of-range index ([f0cd4cf](https://github.com/jonperron/fihr-linter/commit/f0cd4cfff3d4ce192eb6674f2b2be666366b638e))
* **release:** keep cargo.lock in sync with the release version ([04f525b](https://github.com/jonperron/fihr-linter/commit/04f525b019bc7cc8a34eae627e4ca1fbaa7cdade))
* **release:** leave the stub cli out of release assets ([9ce52a1](https://github.com/jonperron/fihr-linter/commit/9ce52a1edacfff3980b54c284be5c6a2bb9feb0e))
* **release:** pin the rust toolchain and manylinux image ([7a70e72](https://github.com/jonperron/fihr-linter/commit/7a70e72f6cf5440b608bfb271197ebe1a1ed310c))
* **release:** publish the release only after its assets are uploaded ([db50792](https://github.com/jonperron/fihr-linter/commit/db507927d5d940db0e664978eebd7a9739684960))
* **validator:** enforce json array shape of repeating elements ([e8a2bab](https://github.com/jonperron/fihr-linter/commit/e8a2babca98c45bf81c0a09fb4eb56b7b620e808))
* **validator:** enforce json array shape of repeating elements ([4c0c072](https://github.com/jonperron/fihr-linter/commit/4c0c0725a7aad4200b4e981e74d0897e0a037400))
* **validator:** name non-concrete types in structure_000 message ([376340c](https://github.com/jonperron/fihr-linter/commit/376340c3cfe79d1ef7dce23dbdb55cb0eda924ee))
* **validator:** reject abstract and non-resource resource types ([26321fa](https://github.com/jonperron/fihr-linter/commit/26321fa5ee12a627f41885a8861e782634835909))
* **validator:** reject abstract and non-resource resource types ([6043d21](https://github.com/jonperron/fihr-linter/commit/6043d21dab2b9d5540204beb2cda399ac2928bcc))
* **validator:** reject unknown _name keys and walk primitive extensions ([d25d6ee](https://github.com/jonperron/fihr-linter/commit/d25d6eeb8b0d0705dc9caff08a1f8a56e4a88672))
* **validator:** reject unknown _name keys and walk primitive extensions ([da504d8](https://github.com/jonperron/fihr-linter/commit/da504d8401b342386a5316c6d857d49b89ac756f))
* **validator:** skip choice slices in cardinality check ([be0e075](https://github.com/jonperron/fihr-linter/commit/be0e075559fd6ba44c4e8d4cd95b8feaafd01558))
* **validator:** skip slice elements in profile cardinality check ([365c3bb](https://github.com/jonperron/fihr-linter/commit/365c3bb1a9258f08f1977f2e34bf886531605880))
* **validator:** skip slice elements in profile cardinality check ([0442d2b](https://github.com/jonperron/fihr-linter/commit/0442d2b08b609037c621977e368bc084dbad2f7a))
* **validator:** type XML primitive extensions and keep empty primitives ([6607001](https://github.com/jonperron/fihr-linter/commit/6607001642869986b466624758a39a59efd4f2ac))
* **validator:** type XML primitives and split their extensions ([d0c56b4](https://github.com/jonperron/fihr-linter/commit/d0c56b49fd7222e8b91a7d371b8625ca67cf2ba2))
* **validator:** type XML primitives and split their extensions ([4ee84b2](https://github.com/jonperron/fihr-linter/commit/4ee84b228f39a96671d08066d93b31e788d9dc5e))

## [0.4.1](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.4.0...fhir-linter-v0.4.1) (2026-11-08)


### Bug Fixes

* **definitions:** load core profiles from profiles-others.json ([b83c499](https://github.com/jonperron/fihr-linter/commit/b83c499a0a22d62b4982c5ae38b6538b26fd2e3c))
* **definitions:** load core profiles from profiles-others.json ([366a8cc](https://github.com/jonperron/fihr-linter/commit/366a8ccd07e15da5fc42eb9aed143213f6bf3eab))
* **parser:** parse integers beyond i64 as decimals ([efba82d](https://github.com/jonperron/fihr-linter/commit/efba82d6b283a0c7f553fc1aa2dcd670b8c8470e))
* **parser:** parse integers beyond i64 as decimals ([cfb5729](https://github.com/jonperron/fihr-linter/commit/cfb57291a5b18c16e6589e07aa5e9545bae2316c))
* **validator:** address audit should-fix findings ([977d45d](https://github.com/jonperron/fihr-linter/commit/977d45deb95dc0c34acac9db3f8e477e06fa3791))
* **validator:** apply the primitive extension rule at every depth ([513016e](https://github.com/jonperron/fihr-linter/commit/513016ef2e8e702fda6f9c6f88089c0a143b7dc5))
* **validator:** check cardinality of choice elements ([4e61e8b](https://github.com/jonperron/fihr-linter/commit/4e61e8b07298a79fe32aa8fa16068179d75c2b19))
* **validator:** enforce strict temporal and 32-bit integer formats ([ec28548](https://github.com/jonperron/fihr-linter/commit/ec28548e3983fde0f10e71cf496bd770197a5d84))
* **validator:** report skipped invariants as information ([3071256](https://github.com/jonperron/fihr-linter/commit/30712567d1a71115516ec21233239415a9f0ea86))
* **validator:** require extension shape for primitive presence ([4df7c9f](https://github.com/jonperron/fihr-linter/commit/4df7c9f0674321c091836e09a6ed5a319013bd3c))
* **validator:** tighten primitive extension and choice handling ([816de22](https://github.com/jonperron/fihr-linter/commit/816de22439ad178ef9acc01eebd64e41369094eb))
* **validator:** type-check resource id and fhirpath system types ([38ed94e](https://github.com/jonperron/fihr-linter/commit/38ed94e869a0d703558f9b88a0e2ab3027c5ac86))
* **validator:** validate choice elements from their declared types ([ea96e77](https://github.com/jonperron/fihr-linter/commit/ea96e778ede742c0a5dcd17636cf21b079f90be0))
* **validator:** validate choice elements from their declared types ([dc82c37](https://github.com/jonperron/fihr-linter/commit/dc82c3718660799cae3a51fe9072b22fa818942a))
* **validator:** validate extension and modifierExtension elements ([d24e99c](https://github.com/jonperron/fihr-linter/commit/d24e99c77e1b34dc1292cda7b8e95dc16b33ee8f))
* **validator:** validate resource ids and required choice elements ([aeb054a](https://github.com/jonperron/fihr-linter/commit/aeb054aba3c164268ebfdc683eb1be59cc778a71))


### Performance Improvements

* **validator:** index snapshot elements by path ([d35a39b](https://github.com/jonperron/fihr-linter/commit/d35a39b98f7cf642990c92a10c567f347ee610c5))

## [0.4.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.3.0...fhir-linter-v0.4.0) (2026-11-03)


### Features

* **python:** expose validate and lint via pyo3 ([e3452f0](https://github.com/jonperron/fihr-linter/commit/e3452f048ead3068520538a3bfb1c2591e1d2f73))
* **python:** phase 8 python bindings ([8e6ae2a](https://github.com/jonperron/fihr-linter/commit/8e6ae2ac93d41bf387e09d05c6406857f11b27b4))
* **typescript:** add node addon bindings with validate and lint ([b1b4510](https://github.com/jonperron/fihr-linter/commit/b1b45104b9834b3651201c4e4674448cf9a27c58))
* **typescript:** node addon bindings (phase 9) ([03c1c9e](https://github.com/jonperron/fihr-linter/commit/03c1c9e2300b87af29f2824761a16ed2e00bc0d0))
* update readme ([6d6a579](https://github.com/jonperron/fihr-linter/commit/6d6a5791bc49df609894b3c66433f3ba800bd344))
* update readme ([dfa1ace](https://github.com/jonperron/fihr-linter/commit/dfa1acefb8bda9eeba5b99c724d4cdd8635c710b))


### Bug Fixes

* **deps:** bump quick-xml to 0.41 ([54b7a18](https://github.com/jonperron/fihr-linter/commit/54b7a182252b524cbe6cb5f0eb0a9608dfcd60a2))
* **python:** build abi3 wheels for all python &gt;= 3.10 ([fa23b40](https://github.com/jonperron/fihr-linter/commit/fa23b40f1b32c8df4055ca97e991c17fc779ac8a))
* **python:** share one registry per resolved definitions path ([013ee68](https://github.com/jonperron/fihr-linter/commit/013ee68595997e107d7becdde5be16cfc83bd9d0))
* **typescript:** address cross-review findings ([f2c14f5](https://github.com/jonperron/fihr-linter/commit/f2c14f5bfc7e7538c4ba418f8b4c102fd4cf0597))


### Performance Improvements

* **python:** release the GIL during validate and lint ([edb2d6c](https://github.com/jonperron/fihr-linter/commit/edb2d6c6d8fc8bb7b5435e72a2a1749a86a86964))

## [0.3.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.2.0...fhir-linter-v0.3.0) (2026-11-02)


### Features

* **fhir-linter:** add rule registry, TOML config and first rules ([ed25744](https://github.com/jonperron/fihr-linter/commit/ed25744fa5017d08f2afb37069ce8a9cc1ece701))
* **fhir-validator:** add references and profile layers ([815504d](https://github.com/jonperron/fihr-linter/commit/815504d3e7d6e7f12d7b94b8ac13d256640e0c8d))
* **fhir-validator:** complete phase 5 validator layers ([a4b3362](https://github.com/jonperron/fihr-linter/commit/a4b3362e46aac0a5f503840c695156392565236d))
* **fhir-validator:** implement terminology layer ([de23f1b](https://github.com/jonperron/fihr-linter/commit/de23f1b07baa69bdc311b09a628a125a11d4a95d))
* **fhir-validator:** validate nested elements ([b043b79](https://github.com/jonperron/fihr-linter/commit/b043b79e03b818c96f1d157bc528c4567d6276ba))
* nested validation (phase 5) and linter (phase 6) ([a859adc](https://github.com/jonperron/fihr-linter/commit/a859adc9b0c104c0a555bd5f2a64e8a98254427c))


### Bug Fixes

* **fhir-fhirpath:** harden evaluator (panics, equality, regex) ([1bfc557](https://github.com/jonperron/fihr-linter/commit/1bfc5576c7e50d0be13157391f78649187230a5b))
* **fhir-fhirpath:** harden evaluator panics, set equality and regex ([274eaf3](https://github.com/jonperron/fihr-linter/commit/274eaf3dd81f9994102745b7aeb21b1ec092ddd7))
* **fhir-fhirpath:** harden parser string, bool and nesting handling ([96805eb](https://github.com/jonperron/fihr-linter/commit/96805ebc20c2309ddc9ecc954662f81ec2c3ff7b))
* **fhir-fhirpath:** harden parser string, bool and nesting handling ([9751612](https://github.com/jonperron/fihr-linter/commit/9751612ea1d4036b224dc416d2c1eedc7f1aeb3f))
* **fhir-fhirpath:** make nesting guard skip comments and backticks ([3381cfc](https://github.com/jonperron/fihr-linter/commit/3381cfc3d5861c8cea81c472adf1f8ea14cbcbb8))
* **fhir-fhirpath:** restore descendants order and tolerate base64 whitespace ([3e222be](https://github.com/jonperron/fihr-linter/commit/3e222beec5510ce2e58e6ed33e5181533bd68aec))
* **fhir-fhirpath:** tighten parser limits to prevent stack overflow ([868f773](https://github.com/jonperron/fihr-linter/commit/868f773b86c228b57091db35883a0f449a0ca067))
* **fhir-fhirpath:** tighten parser limits to prevent stack overflow ([504559e](https://github.com/jonperron/fihr-linter/commit/504559efac4650194aa3656696eafa3b7ab3ba49))
* **fhir-fhirpath:** use checked i64 arithmetic for integers ([82eb980](https://github.com/jonperron/fihr-linter/commit/82eb9808fd6b3c91aeb37f83355b864b302dbe43))
* **fhir-fhirpath:** use checked i64 arithmetic for integers ([3bf0ead](https://github.com/jonperron/fihr-linter/commit/3bf0eadfb0a7a6e2990073209e40dea94ecd2f79))
* **fhir-parser:** cap nesting depth and harden definitions loader ([fbb6e1e](https://github.com/jonperron/fihr-linter/commit/fbb6e1e09159f29bc1d1636ffd3f05dee43f5193))
* **fhir-parser:** cap nesting depth and harden definitions loader ([8c37ca4](https://github.com/jonperron/fihr-linter/commit/8c37ca4d8383dda8ef9b7b571f3294eba5ad200b))
* **fhir-validator:** address phase 5 review findings ([415c1ad](https://github.com/jonperron/fihr-linter/commit/415c1ad7acde523b5f03d1e714f7866370df422b))
* **fhir-validator:** handle contentReference and primitive extensions ([69402ec](https://github.com/jonperron/fihr-linter/commit/69402ecee9d35ccfaa3aee346ca28554a2c98b8f))
* **fhir-validator:** handle contentReference and primitive extensions ([58c08db](https://github.com/jonperron/fihr-linter/commit/58c08db50825842d11cb9a55f7ba99642a452d79))
* **fhir-validator:** non-ascii panic, dedupe, INVARIANT_PARSE ([2cccbb2](https://github.com/jonperron/fihr-linter/commit/2cccbb21c878f5e7c09caa03068980dfc2736972))
* **fhir-validator:** prevent non-ascii panic and warn on unparsable invariants ([7bdccbd](https://github.com/jonperron/fihr-linter/commit/7bdccbd625cf2378049ed9bccd73bbe4d54f5d9d))
* **fhir-validator:** skip unparseable invariants silently ([fcc3789](https://github.com/jonperron/fihr-linter/commit/fcc37896b08de072f2de49d5493cc9f4970f2d57))
* **fhir-validator:** skip unparseable invariants silently ([5dd7973](https://github.com/jonperron/fihr-linter/commit/5dd7973bbf210f7a2a04e3488e4016646fe64517))

## [0.2.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.1.0...fhir-linter-v0.2.0) (2026-11-01)


### Features

* **agents:** bootstrap agents ([88e5fae](https://github.com/jonperron/fihr-linter/commit/88e5faeef6a1012eb797e4da42836879c53e4f4d))
* **agents:** bootstrap agents ([ff96ec7](https://github.com/jonperron/fihr-linter/commit/ff96ec7c0b7169fcef1071e6328f6d2032f60171))
* **fhir-definitions:** registry and model ([435dbad](https://github.com/jonperron/fihr-linter/commit/435dbad29cc5ec2b02bc5655fdb94c575cc690da))
* **fhir-definitions:** registry and model ([3bb5b3e](https://github.com/jonperron/fihr-linter/commit/3bb5b3e9ba3a9c708c7288ea994478d041f7420a))
* **fhir-fhirpath:** implement FHIRPath 2.0 evaluator ([cf6bcfa](https://github.com/jonperron/fihr-linter/commit/cf6bcfa8aec791fddee75767949ea2eef28734df))
* **fhir-fhirpath:** upgrade evaluator to FHIRPath 3.0 ([78ef577](https://github.com/jonperron/fihr-linter/commit/78ef5779eff21948cec7df6987901edae9d9cd13))
* **fhir-fhirpath:** upgrade evaluator to FHIRPath 3.0 ([ca061d1](https://github.com/jonperron/fihr-linter/commit/ca061d1b1d4c4b248b22bd2e122d814db3d7c5e6))
* **fhir-parser:** implement JSON parser with source spans ([64feb71](https://github.com/jonperron/fihr-linter/commit/64feb71d9284000f59fe05407872ceace2d1367e))
* **fhir-parser:** implement JSON parser with source spans ([583c008](https://github.com/jonperron/fihr-linter/commit/583c008560fb0d70dc17981f33a413140e12f7d7))
* **fhir-parser:** implement XML parser ([729a416](https://github.com/jonperron/fihr-linter/commit/729a416d2efb953e421ae4e1769f6606a5032f64))
* **fhir-parser:** implement XML parser (phase 3) ([5ce89b1](https://github.com/jonperron/fihr-linter/commit/5ce89b19e2ab3eff92461925327835862c931b06))
* **fhir-validator:** implement core validator ([771a126](https://github.com/jonperron/fihr-linter/commit/771a1263462741b62d7ff263e20db0284713c3ed))


### Bug Fixes

* **fhir-definitions,scripts:** reject malformed inputs at parse boundaries ([020012d](https://github.com/jonperron/fihr-linter/commit/020012d45fe7fae6d056d76d6c0f5553fc414ef7))
* **fhir-parser:** preserve non-value attributes and reject trailing elements ([a74e6fa](https://github.com/jonperron/fihr-linter/commit/a74e6fa7049535abcddc0907a7e597e65004586a))
* **security:** harden download script and forbid unsafe code ([fea6011](https://github.com/jonperron/fihr-linter/commit/fea6011837e2e77948409dfe34726881b6555d32))
* **typo:** fix types and json parser ([a1e4bc5](https://github.com/jonperron/fihr-linter/commit/a1e4bc5d486104192c7724e2663fe121b0322a78))
* **typo:** fix types and json parser ([99245d1](https://github.com/jonperron/fihr-linter/commit/99245d102cd616099af37f5cfce1cdf1d93aca78))
