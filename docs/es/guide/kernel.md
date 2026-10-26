---
description: "Qué calcula hoy el núcleo de verosimilitud de Rove y cómo llamarlo desde Rust: terminales, matrices de transición, parciales, reescalado y la log-verosimilitud."
---

# El núcleo de verosimilitud

`LikelihoodCore` calcula la log-verosimilitud de un árbol binario enraizado para una matriz de patrones de sitio, dada una matriz de transición por rama y por categoría de tasa. Esta página describe la API tal como existe en el repositorio. Todavía no hay un crate en crates.io; para probarlo, usa el repositorio como dependencia de Git.

::: info Alcance
El núcleo recibe matrices de transición y datos de los terminales y devuelve verosimilitudes. El [lector de NEXUS](./nexus), separado, prepara las observaciones y los pesos de los patrones. Las propuestas de árboles y el cálculo de tasas gamma siguen siendo trabajo futuro. Ver el [roadmap](./roadmap).
:::

## Nodos y búferes

Los nodos se numeran de `0` a `node_count − 1`. El núcleo no sabe qué nodos son terminales; solo necesita que se llame a `calculate_partials` para cada nodo interno una vez que sus dos hijos están listos.

Cada nodo posee dos búferes de parciales y dos búferes de matrices de transición. Llama a `set_node_partials_for_update` o `set_node_matrix_for_update` antes de sobrescribir un nodo durante una propuesta, y usa `store`, `restore` y `unstore` para gestionar qué conjunto es el actual. La página de [diseño](./design#buferes-dobles-y-reescalado) explica el mecanismo.

## Terminales

`set_tip_states(node, states)` recibe un índice de estado por patrón. Un valor igual o mayor que el número de estados significa datos faltantes o ambiguos: cada estado recibe un parcial de 1.0.

`set_tip_partials(node, partials)` recibe `patterns × states` valores, dispuestos por patrones, para terminales con incertidumbre.

Ambos escriben en los dos búferes del nodo, de modo que los terminales sobreviven a `restore`.

## Matrices de transición

`set_node_matrix(node, category, matrix)` almacena una matriz `states × states` en orden por filas para la rama sobre `node`, en una categoría de tasa. `model::Mk` construye esas matrices:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`Mk::uniform(k)` usa frecuencias iguales; `Mk::with_frequencies` acepta las tuyas. `transition_matrix(distance, out)` llena `out` para una rama con el número esperado de cambios por carácter indicado. La heterogeneidad de tasas queda en manos de quien llama: calcula una matriz por categoría con la longitud de rama multiplicada por la tasa de esa categoría.

## Calcular una verosimilitud

Cuatro patrones de un carácter binario en tres terminales, sobre el árbol `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nodos: A = 0, B = 1, C = 2, (A, B) = 3, raíz = 4.
let states = 2;
let patterns = 4;
let categories = 1;
let nodes = 5;

let mut core = LikelihoodCore::new(states, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 está fuera de rango: datos faltantes

let model = Mk::uniform(states);
let mut matrix = vec![0.0; states * states];
for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    model.transition_matrix(length, &mut matrix);
    core.set_node_matrix(node, 0, &matrix);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

`log_likelihood` integra los parciales de la raíz sobre los pesos de las categorías, multiplica por las frecuencias de estados, toma el logaritmo por patrón, suma los factores de escala que haya y suma ponderando con los pesos de los patrones. Los pasos también están disponibles por separado como `integrate_partials`, `calculate_log_likelihoods` y `pattern_log_likelihoods`.

La implementación de referencia da el mismo valor para este árbol. Eso es lo que comprueban las pruebas de igualdad, con entradas aleatorias.

## Reescalado

Pasa `Scaling::Always` para dividir los parciales de cada nodo interno por su máximo por patrón a medida que se calculan. El logaritmo de cada máximo se guarda y se vuelve a sumar al final, de modo que el resultado coincide con el cálculo sin escalar, sin el underflow en árboles profundos. `Scaling::None` omite ese trabajo.

## Comprobarlo

`cargo test` ejecuta las pruebas de propiedades de `tests/equality.rs`. Generan árboles aleatorios con 2 a 10 terminales, 2 a 6 estados, 1 a 4 categorías de tasa y hasta 40 patrones, y comparan el núcleo con `reference::Problem` en cuatro combinaciones: estados o parciales en los terminales, con o sin reescalado. Una prueba separada comprueba que `restore` recupera la verosimilitud calculada antes de una propuesta.
