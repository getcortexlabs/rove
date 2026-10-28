---
description: "O que o núcleo de verossimilhança do Rove calcula hoje e como chamá-lo a partir do Rust: terminais, distâncias de ramo no Mk, matrizes de transição, parciais, a correção Mkv, reescalonamento e a log-verossimilhança."
---

# O núcleo de verossimilhança

`LikelihoodCore` calcula a log-verossimilhança de uma árvore binária enraizada para uma matriz de padrões de sítio. Cada ramo é informado por categoria de taxa, como uma distância sob o modelo Mk ou como uma matriz de transição. Esta página descreve a API como ela existe no repositório. Ainda não há crate no crates.io; para experimentar, use o repositório como dependência Git.

::: info Escopo
O núcleo recebe distâncias de ramo ou matrizes de transição, além dos dados de terminais, e devolve verossimilhanças. O [leitor de NEXUS](./nexus), que é separado, prepara observações e pesos de padrões, e a [correção Mkv](#condicionando-a-caracteres-variaveis) condiciona a verossimilhança a caracteres variáveis. Propostas de árvore e o cálculo de taxas gama continuam como trabalho futuro. Veja o [roadmap](./roadmap).
:::

## Nós e buffers

Os nós são numerados de `0` a `node_count − 1`. O núcleo não sabe quais nós são terminais; ele só precisa que `calculate_partials` seja chamado para cada nó interno depois que seus dois filhos estiverem prontos.

Cada nó possui dois buffers de parciais e dois buffers para o seu ramo. Chame `set_node_partials_for_update` ou `set_node_matrix_for_update` antes de sobrescrever um nó durante uma proposta, e use `store`, `restore` e `unstore` para controlar qual conjunto está em uso. `set_node_matrix_for_update` alterna o buffer do ramo nos dois tipos de núcleo, seja ele de distâncias ou de matrizes. A página de [design](./design#buffers-duplos-e-reescalonamento) explica o mecanismo.

## Terminais

`set_tip_states(node, states)` recebe um índice de estado por padrão. Um valor igual ou maior que o número de estados significa dados faltantes ou ambíguos: todos os estados recebem parcial 1.0.

`set_tip_partials(node, partials)` recebe `patterns × states` valores, em ordem pattern-major, para terminais com incerteza.

Ambos escrevem nos dois buffers do nó, então os terminais sobrevivem a `restore`.

## Ramos no Mk

`LikelihoodCore::mk(&model, patterns, categories, nodes, scaling)` constrói um núcleo para um modelo `Mk`. `set_node_distance(node, category, distance)` define o ramo acima de `node`, em uma categoria de taxa, como uma distância em mudanças esperadas por caractere. A heterogeneidade de taxas fica a cargo de quem chama: passe o comprimento de ramo multiplicado pela taxa dessa categoria.

O núcleo nunca constrói uma matriz de transição. A matriz do Mk é `P = e·I + (1 − e)·1πᵀ`, com `e = exp(−d/β)` e `β = 1 − Σᵢ πᵢ²`, a normalização que faz com que se espere uma mudança por unidade de distância. Para cada ramo, o núcleo guarda `e` e `1 − e` e atualiza as parciais `L` de um filho com

```text
(P·L)ᵢ = e·Lᵢ + (1 − e)·Σⱼ πⱼ·Lⱼ
```

A soma ponderada é calculada uma vez por padrão e compartilhada por todos os estados, então atualizar um filho custa O(k) por padrão e por categoria de taxa, em vez do O(k²) de um produto de matrizes. `1 − e` vem de `f64::exp_m1`, que preserva a precisão em ramos curtos.

`Mk::uniform(k)` usa frequências iguais. `Mk::with_frequencies` aceita frequências desiguais: a mesma forma vale para qualquer modelo equal-input, em que uma mudança sorteia o novo estado segundo `π`. Ele devolve um `model::Error`, a menos que haja pelo menos duas frequências, cada uma positiva e finita, com soma igual a um, salvo erro de arredondamento. Elas são usadas como foram passadas, sem normalização, então valores arredondados com menos dígitos, como três frequências de 0,3333333, são rejeitados: divida-os antes pela soma. `log_likelihood` continua recebendo as frequências da raiz, como num núcleo de matrizes; passe `model.frequencies()`.

Num núcleo Mk, `set_node_matrix` e `node_matrix` causam panic; num núcleo de matrizes, `set_node_distance` e `node_distance` também. `set_node_distance` e `transition_matrix` também causam panic com uma distância negativa ou NaN.

## Matrizes de transição

`LikelihoodCore::new(states, patterns, categories, nodes, scaling)` constrói, em vez disso, um núcleo que recebe matrizes. `set_node_matrix(node, category, matrix)` armazena uma matriz `states × states` em ordem row-major para o ramo acima de `node`, em uma categoria de taxa, e o núcleo a multiplica por inteiro. Este é o caminho para modelos além do Mk. `model::Mk` também constrói as próprias matrizes:

```rust
use rove::model::Mk;

let model = Mk::uniform(2);
let mut matrix = vec![0.0; 4];
model.transition_matrix(0.1, &mut matrix);
```

`transition_matrix(distance, out)` preenche `out` para um ramo cujo número esperado de mudanças por caractere é o informado, com os mesmos `e` e `1 − e` de um núcleo Mk.

## Calculando uma verossimilhança

Quatro padrões de um caractere binário em três terminais, na árvore `((A, B), C)`:

```rust
use rove::model::Mk;
use rove::{LikelihoodCore, Scaling};

// Nós: A = 0, B = 1, C = 2, (A, B) = 3, raiz = 4.
let patterns = 4;
let categories = 1;
let nodes = 5;

let model = Mk::uniform(2);
let mut core = LikelihoodCore::mk(&model, patterns, categories, nodes, Scaling::Always);
core.set_tip_states(0, &[0, 0, 1, 1]);
core.set_tip_states(1, &[0, 1, 1, 0]);
core.set_tip_states(2, &[0, 1, 0, 2]); // 2 está fora do intervalo: dados faltantes

for (node, length) in [(0, 0.1), (1, 0.1), (2, 0.3), (3, 0.2)] {
    core.set_node_distance(node, 0, length);
}

core.calculate_partials(0, 1, 3);
core.calculate_partials(3, 2, 4);

let pattern_weights = [1.0, 1.0, 1.0, 1.0];
let log_likelihood = core.log_likelihood(4, &[1.0], model.frequencies(), &pattern_weights);
assert!((log_likelihood - -8.957188856221643).abs() < 1e-12);
```

Um núcleo de matrizes, `LikelihoodCore::new(2, patterns, categories, nodes, Scaling::Always)`, com as matrizes de `model.transition_matrix`, devolve o mesmo valor, a menos de arredondamento.

`log_likelihood` integra as parciais da raiz sobre os pesos das categorias, multiplica pelas frequências de estados, toma o logaritmo por padrão, soma os fatores de escala que houver e faz a soma ponderada pelos pesos dos padrões. As etapas também estão disponíveis separadamente como `integrate_partials`, `calculate_log_likelihoods` e `pattern_log_likelihoods`.

A implementação de referência dá o mesmo valor para esta árvore. É isso que os testes de igualdade verificam, com entradas aleatórias.

## Condicionando a caracteres variáveis

Matrizes morfológicas costumam manter apenas caracteres que variam entre os táxons. A verossimilhança delas precisa ser condicionada a essa seleção, como no [modelo Mkv](https://doi.org/10.1080/106351501753462876) de Lewis. Sendo `Pₛ` a probabilidade de que todos os táxons estejam no estado `s`, combinada sobre as categorias de taxa, cada padrão observado `i`, com verossimilhança `Lᵢ` e peso `wᵢ`, contribui com

```text
wᵢ · (ln Lᵢ − ln(1 − Σₛ Pₛ))
```

`ascertainment::VariableCharacters` acrescenta um padrão constante por estado declarado às entradas dos terminais de uma partição. O núcleo os avalia junto com as observações, na mesma árvore, com os mesmos ramos, pesos de categoria e fatores de escala. Eles não têm peso de observação. Quatro caracteres variáveis na árvore do exemplo anterior:

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

// Nós como antes. Quatro padrões observados e depois os dois constantes.
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

`log_variable_probability` devolve só a correção, `ln(1 − Σₛ Pₛ)`.

`VariableCharacters::new` rejeita um caractere que poderia ser constante: aquele em que todas as células, incluindo as faltantes e as incertas, admitem o mesmo estado. `0 ? 0` e `{01} 1 1` produzem um erro que identifica o caractere. `0 {12} 0` é aceito, porque varia qualquer que seja o estado da célula incerta. Toda forma de completar um caractere aceito é variável, então o resultado é a verossimilhança das observações condicionada a que o caractere completo varie. O BEAST 2.7.8 trata de outro jeito as colunas observadas idênticas às suas colunas constantes: emite um aviso e dá peso zero a elas.

Cada estado declarado tem seu próprio padrão constante, inclusive estados que nenhum táxon apresenta. As partições são condicionadas separadamente. O condicionamento a caracteres informativos para parcimônia e as correções por caractere para dados faltantes não estão implementados.

Os padrões constantes passam pela mesma poda que as observações. Quando concentram quase toda a probabilidade, como em árvores muito curtas, seus erros de arredondamento são amplificados por `Σₛ Pₛ / (1 − Σₛ Pₛ)`, que passa de 100 nos dois estados de referência arquivados. Com dois táxons a uma distância `d`, o erro de cada log-probabilidade condicionada cresce aproximadamente como `10⁻¹⁶ / d`. Se a probabilidade total dos padrões constantes ficar a poucos erros de arredondamento de um, o resultado é `ascertainment::Error::UndefinedCorrection` em vez de um número.

Nos dois estados arquivados do BEAST 2.7.8 da [análise de referência](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md), o Rove reproduz a log-verossimilhança de cada padrão com diferença de no máximo 1,4 × 10⁻¹², e a log-verossimilhança Mkv com diferença de no máximo 2,2 × 10⁻¹². Quase toda a diferença por padrão vem do arredondamento do próprio BEAST: o modelo LewisMK calcula `1 − e` como `1 − exp(−x)`, o que perde dígitos nos ramos curtos da referência. Numa avaliação das mesmas entradas com 60 algarismos significativos, as log-verossimilhanças por padrão do Rove ficam a no máximo 6 × 10⁻¹⁵ dos valores exatos, e as do BEAST a no máximo 1,4 × 10⁻¹².

## Reescalonamento

Passe `Scaling::Always` para dividir as parciais de cada nó interno pelo seu máximo por padrão à medida que são calculadas. O logaritmo de cada máximo é guardado e somado de volta no final, de modo que o resultado coincide com o cálculo sem escalonamento, sem o underflow em árvores profundas. `Scaling::None` dispensa esse trabalho.

## Verificando

`cargo test` executa os testes de propriedade em `tests/equality.rs`. Eles geram árvores aleatórias com 2 a 10 terminais, 2 a 6 estados, 1 a 4 categorias de taxa e até 40 padrões, e comparam os dois tipos de núcleo com `reference::Problem`, com estados ou parciais nos terminais e com ou sem reescalonamento. Um deles distribui os comprimentos de ramo entre 10⁻⁹ e 100. Outro aplica sequências aleatórias de mudanças de ramo, aceita ou rejeita cada uma e confere a verossimilhança depois de cada passo; uma mudança rejeitada precisa trazer de volta exatamente o valor anterior. Um teste com dois táxons confere a probabilidade de uma mudança mesmo quando eles estão a uma distância de apenas 10⁻¹⁵, e testes unitários em `src/model.rs` conferem a validação das frequências e a normalização da taxa.

`tests/ascertainment.rs` cobre a correção Mkv: os estados arquivados do BEAST, comparados com os valores do BEAST e com a avaliação de 60 algarismos, a enumeração completa dos padrões em árvores com dois a quatro terminais, a forma fechada para dois táxons em distâncias de 10⁻⁸ a 1000, árvores aleatórias comparadas com `reference::Problem::variable_log_likelihood`, caracteres que poderiam ser constantes, a restauração depois de uma proposta rejeitada e a correção indefinida numa árvore de comprimento zero.
