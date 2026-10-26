---
description: "O que o núcleo de verossimilhança do Rove calcula hoje e como chamá-lo a partir do Rust: terminais, matrizes de transição, parciais, reescalonamento e a log-verossimilhança."
---

# O núcleo de verossimilhança

`LikelihoodCore` calcula a log-verossimilhança de uma árvore binária enraizada para uma matriz de padrões de sítio, dada uma matriz de transição por ramo e por categoria de taxa. Esta página descreve a API como ela existe no repositório. Ainda não há crate no crates.io; para experimentar, use o repositório como dependência Git.

::: info Escopo
O núcleo recebe matrizes de transição e dados de terminais e devolve verossimilhanças. O [leitor de NEXUS](./nexus), que é separado, prepara observações e pesos de padrões. Propostas de árvore e o cálculo de taxas gama continuam como trabalho futuro. Veja o [roadmap](./roadmap).
:::

## Nós e buffers

Os nós são numerados de `0` a `node_count − 1`. O núcleo não sabe quais nós são terminais; ele só precisa que `calculate_partials` seja chamado para cada nó interno depois que seus dois filhos estiverem prontos.

Cada nó possui dois buffers de parciais e dois buffers de matriz de transição. Chame `set_node_partials_for_update` ou `set_node_matrix_for_update` antes de sobrescrever um nó durante uma proposta, e use `store`, `restore` e `unstore` para controlar qual conjunto está em uso. A página de [design](./design#buffers-duplos-e-reescalonamento) explica o mecanismo.

## Terminais

`set_tip_states(node, states)` recebe um índice de estado por padrão. Um valor igual ou maior que o número de estados significa dados faltantes ou ambíguos: todos os estados recebem parcial 1.0.

`set_tip_partials(node, partials)` recebe `patterns × states` valores, em ordem pattern-major, para terminais com incerteza.

Ambos escrevem nos dois buffers do nó, então os terminais sobrevivem a `restore`.

## Matrizes de transição

`set_node_matrix(node, category, matrix)` armazena uma matriz `states × states` em ordem row-major para o ramo acima de `node`, em uma categoria de taxa. `model::Mk` constrói essas matrizes:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`Mk::uniform(k)` usa frequências iguais; `Mk::with_frequencies` recebe as suas. `transition_matrix(distance, out)` preenche `out` para um ramo cujo número esperado de mudanças por caractere é o informado. A heterogeneidade de taxas fica a cargo de quem chama: calcule uma matriz por categoria com o comprimento de ramo multiplicado pela taxa dessa categoria.

## Calculando uma verossimilhança

Quatro padrões de um caractere binário em três terminais, na árvore `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nós: A = 0, B = 1, C = 2, (A, B) = 3, raiz = 4.
let states = 2;
let patterns = 4;
let categories = 1;
let nodes = 5;

let mut core = LikelihoodCore::new(states, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 está fora do intervalo: dados faltantes

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

`log_likelihood` integra as parciais da raiz sobre os pesos das categorias, multiplica pelas frequências de estados, toma o logaritmo por padrão, soma os fatores de escala que houver e faz a soma ponderada pelos pesos dos padrões. As etapas também estão disponíveis separadamente como `integrate_partials`, `calculate_log_likelihoods` e `pattern_log_likelihoods`.

A implementação de referência dá o mesmo valor para esta árvore. É isso que os testes de igualdade verificam, com entradas aleatórias.

## Reescalonamento

Passe `Scaling::Always` para dividir as parciais de cada nó interno pelo seu máximo por padrão à medida que são calculadas. O logaritmo de cada máximo é guardado e somado de volta no final, de modo que o resultado coincide com o cálculo sem escalonamento, sem o underflow em árvores profundas. `Scaling::None` dispensa esse trabalho.

## Verificando

`cargo test` executa os testes de propriedade em `tests/equality.rs`. Eles geram árvores aleatórias com 2 a 10 terminais, 2 a 6 estados, 1 a 4 categorias de taxa e até 40 padrões, e comparam o núcleo com `reference::Problem` em quatro combinações: estados ou parciais nos terminais, com ou sem reescalonamento. Um teste separado verifica que `restore` traz de volta a verossimilhança calculada antes de uma proposta.
