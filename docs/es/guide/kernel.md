---
description: "Qué calcula hoy el núcleo de verosimilitud de Rove y cómo llamarlo desde Rust: terminales, distancias de rama en Mk, matrices de transición, parciales, la corrección Mkv, reescalado y la log-verosimilitud."
---

# El núcleo de verosimilitud

`LikelihoodCore` calcula la log-verosimilitud de un árbol binario enraizado para una matriz de patrones de sitio. Cada rama se indica por categoría de tasa, como una distancia bajo el modelo Mk o como una matriz de transición. Esta página describe la API tal como existe en el repositorio. Todavía no hay un crate en crates.io; para probarlo, usa el repositorio como dependencia de Git.

::: info Alcance
El núcleo recibe distancias de rama o matrices de transición, junto con los datos de los terminales, y devuelve verosimilitudes. El [lector de NEXUS](./nexus), separado, prepara las observaciones y los pesos de los patrones, y la [corrección Mkv](#condicionar-a-caracteres-variables) condiciona la verosimilitud a caracteres variables. El cálculo de tasas gamma sigue siendo trabajo futuro; las propuestas de árboles forman parte del [muestreador](./sampler). Ver el [roadmap](./roadmap).
:::

## Nodos y búferes

Los nodos se numeran de `0` a `node_count − 1`. El núcleo no sabe qué nodos son terminales; solo necesita que se llame a `calculate_partials` para cada nodo interno una vez que sus dos hijos están listos.

Cada nodo posee dos búferes de parciales y dos búferes para su rama. Llama a `set_node_partials_for_update` o `set_node_matrix_for_update` antes de sobrescribir un nodo durante una propuesta, y usa `store`, `restore` y `unstore` para gestionar qué conjunto es el actual. `set_node_matrix_for_update` cambia el búfer de la rama en los dos tipos de núcleo, tanto si guarda distancias como matrices. La página de [diseño](./design#buferes-dobles-y-reescalado) explica el mecanismo.

## Terminales

`set_tip_states(node, states)` recibe un índice de estado por patrón. Un valor igual o mayor que el número de estados significa datos faltantes o ambiguos: cada estado recibe un parcial de 1.0.

`set_tip_partials(node, partials)` recibe `patterns × states` valores, dispuestos por patrones, para terminales con incertidumbre.

Ambos escriben en los dos búferes del nodo, de modo que los terminales sobreviven a `restore`.

## Ramas en Mk

`LikelihoodCore::mk(&model, patterns, categories, nodes, scaling)` construye un núcleo para un modelo `Mk`. `set_node_distance(node, category, distance)` fija la rama sobre `node`, en una categoría de tasa, en una distancia expresada en cambios esperados por carácter. La heterogeneidad de tasas queda en manos de quien llama: pasa la longitud de rama multiplicada por la tasa de esa categoría.

El núcleo nunca construye una matriz de transición. La matriz de Mk es `P = e·I + (1 − e)·1πᵀ`, con `e = exp(−d/β)` y `β = 1 − Σᵢ πᵢ²`, la normalización que hace que se espere un cambio por unidad de distancia. Para cada rama, el núcleo guarda `e` y `1 − e`, y actualiza los parciales `L` de un hijo con

```text
(P·L)ᵢ = e·Lᵢ + (1 − e)·Σⱼ πⱼ·Lⱼ
```

La suma ponderada se calcula una vez por patrón y la comparten todos los estados, así que actualizar un hijo cuesta O(k) por patrón y por categoría de tasa, en lugar del O(k²) de un producto de matrices. `1 − e` se obtiene con `f64::exp_m1`, que conserva la precisión en ramas cortas.

`Mk::uniform(k)` usa frecuencias iguales. `Mk::with_frequencies` acepta frecuencias desiguales: la misma forma vale para cualquier modelo equal-input, en el que un cambio elige el nuevo estado según `π`. Devuelve un `model::Error` a menos que haya al menos dos frecuencias, cada una positiva y finita, que sumen uno salvo error de redondeo. Se usan tal como se pasan, sin normalizar, así que los valores redondeados a menos dígitos, como tres frecuencias de 0,3333333, se rechazan: divídelos antes por su suma. `log_likelihood` sigue recibiendo las frecuencias de la raíz, igual que con un núcleo de matrices; pasa `model.frequencies()`.

En un núcleo Mk, `set_node_matrix` y `node_matrix` provocan un panic; en un núcleo de matrices, también `set_node_distance` y `node_distance`. `set_node_distance` y `transition_matrix` también provocan un panic con una distancia negativa o NaN.

## Matrices de transición

`LikelihoodCore::new(states, patterns, categories, nodes, scaling)` construye en cambio un núcleo que recibe matrices. `set_node_matrix(node, category, matrix)` almacena una matriz `states × states` en orden por filas para la rama sobre `node`, en una categoría de tasa, y el núcleo la multiplica completa. Esta es la ruta para modelos distintos de Mk. `model::Mk` también puede construir sus propias matrices:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`transition_matrix(distance, out)` llena `out` para una rama con el número esperado de cambios por carácter indicado, con los mismos `e` y `1 − e` que un núcleo Mk.

## Calcular una verosimilitud

Cuatro patrones de un carácter binario en tres terminales, sobre el árbol `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nodos: A = 0, B = 1, C = 2, (A, B) = 3, raíz = 4.
let patterns = 4;
let categories = 1;
let nodes = 5;

let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 está fuera de rango: datos faltantes

for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

Un núcleo de matrices, `LikelihoodCore::new(2, patterns, categories, nodes, Scaling::Always)`, con las matrices de `model.transition_matrix`, devuelve el mismo valor salvo redondeo.

`log_likelihood` integra los parciales de la raíz sobre los pesos de las categorías, multiplica por las frecuencias de estados, toma el logaritmo por patrón, suma los factores de escala que haya y suma ponderando con los pesos de los patrones. Los pasos también están disponibles por separado como `integrate_partials`, `calculate_log_likelihoods` y `pattern_log_likelihoods`.

La implementación de referencia da el mismo valor para este árbol. Eso es lo que comprueban las pruebas de igualdad, con entradas aleatorias.

## Condicionar a caracteres variables

Las matrices morfológicas suelen conservar solo caracteres que varían entre los taxones. Su verosimilitud tiene que condicionarse a esa selección, como en el [modelo Mkv](https://doi.org/10.1080/106351501753462876) de Lewis. Si `Pₛ` es la probabilidad de que todos los taxones estén en el estado `s`, combinada sobre las categorías de tasa, cada patrón observado `i` con verosimilitud `Lᵢ` y peso `wᵢ` aporta

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` añade un patrón constante por estado declarado a las entradas de los terminales de una partición. El núcleo los evalúa junto con las observaciones, sobre el mismo árbol, con las mismas ramas, pesos de categoría y factores de escala. No tienen peso de observación. Cuatro caracteres variables sobre el árbol del ejemplo anterior:

```rust
use rove::ascertainment::VariableCharacters;
use rove::data::GapTreatment;
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling, nexus};

let input = "#NEXUS
BEGIN DATA;
  DIMENSIONS NTAX=3 NCHAR=4;
  FORMAT SYMBOLS=\"01\" MISSING=?;
  MATRIX
    A 0011
    B 0110
    C 1?01
  ;
END;";
let data = nexus::parse(input)?;
let sites = 0..data.character_count();
let patterns = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?.remove(0);
let variable = VariableCharacters::new(&patterns)?;

// Nodos como antes. Cuatro patrones observados y después los dos constantes.
let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, variable.pattern_count(), 1, 5, Scaling::Always);
for (tip, partials) in variable.tip_partials().iter().enumerate() {
    core.set_tip_partials(tip, partials);
}
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}
core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

core.integrate_partials(4, &[1.0]);
let pattern_log_likelihoods = core.calculate_log_likelihoods(model.frequencies());
let log_likelihood = variable.log_likelihood(pattern_log_likelihoods)?;
assert!((log_likelihood - -6.353478344891801).abs() < 1e-12);
```

`log_variable_probability` devuelve solo la corrección, `ln(1 − Σₛ Pₛ)`.

`VariableCharacters::new` rechaza un carácter que podría ser constante: aquel en el que todas las celdas, incluidas las faltantes y las inciertas, admiten el mismo estado. `0 ? 0` y `{01} 1 1` producen un error que identifica el carácter. `0 {12} 0` se acepta, porque varía sea cual sea el estado de la celda incierta. Toda forma de completar un carácter aceptado es variable, así que el resultado es la verosimilitud de las observaciones condicionada a que el carácter completo varíe. BEAST 2.7.8 trata de otra forma las columnas observadas idénticas a sus columnas constantes: emite un aviso y les asigna peso cero.

Cada estado declarado tiene su propio patrón constante, incluidos los estados que ningún taxón presenta. Las particiones se condicionan por separado. El condicionamiento a caracteres informativos para parsimonia y las correcciones por carácter para datos faltantes no están implementados.

Los patrones constantes pasan por la misma poda que las observaciones. Cuando concentran casi toda la probabilidad, como en árboles muy cortos, sus errores de redondeo se amplifican por `Σₛ Pₛ / (1 − Σₛ Pₛ)`, que supera 100 en los dos estados de referencia archivados. Con dos taxones a una distancia `d`, el error de cada log-probabilidad condicionada crece aproximadamente como `10⁻¹⁶ / d`. Si la probabilidad total de los patrones constantes queda a pocos errores de redondeo de uno, el resultado es `ascertainment::Error::UndefinedCorrection` en lugar de un número.

En los dos estados archivados de BEAST 2.7.8 del [análisis de referencia](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md), Rove reproduce la log-verosimilitud de cada patrón con una diferencia de hasta 1,4 × 10⁻¹², y la log-verosimilitud Mkv con una diferencia de hasta 2,2 × 10⁻¹². Casi toda la diferencia por patrón es redondeo del propio BEAST: su modelo LewisMK calcula `1 − e` como `1 − exp(−x)`, lo que pierde dígitos en las ramas cortas de la referencia. Frente a una evaluación de las mismas entradas con 60 cifras significativas, las log-verosimilitudes por patrón de Rove quedan a menos de 6 × 10⁻¹⁵ de los valores exactos, y las de BEAST a menos de 1,4 × 10⁻¹².

## Reescalado

Pasa `Scaling::Always` para dividir los parciales de cada nodo interno por su máximo por patrón a medida que se calculan. El logaritmo de cada máximo se guarda y se vuelve a sumar al final, de modo que el resultado coincide con el cálculo sin escalar, sin el underflow en árboles profundos. `Scaling::None` omite ese trabajo.

## Comprobarlo

`cargo test` ejecuta las pruebas de propiedades de `tests/equality.rs`. Generan árboles aleatorios con 2 a 10 terminales, 2 a 6 estados, 1 a 4 categorías de tasa y hasta 40 patrones, y comparan los dos tipos de núcleo con `reference::Problem`, con estados o parciales en los terminales y con o sin reescalado. Una de ellas reparte las longitudes de rama entre 10⁻⁹ y 100. Otra aplica secuencias aleatorias de cambios de rama, acepta o rechaza cada uno y comprueba la verosimilitud después de cada paso; un cambio rechazado tiene que recuperar exactamente el valor anterior. Una prueba con dos taxones comprueba la probabilidad de un cambio incluso cuando están a una distancia de solo 10⁻¹⁵, y las pruebas unitarias de `src/model.rs` comprueban la validación de las frecuencias y la normalización de la tasa.

`tests/ascertainment.rs` cubre la corrección Mkv: los estados archivados de BEAST, frente a los valores de BEAST y a la evaluación con 60 cifras, la enumeración completa de los patrones en árboles de dos a cuatro terminales, la forma cerrada para dos taxones a distancias de 10⁻⁸ a 1000, árboles aleatorios comparados con `reference::Problem::variable_log_likelihood`, caracteres que podrían ser constantes, la restauración tras una propuesta rechazada y la corrección indefinida en un árbol de longitud cero.
