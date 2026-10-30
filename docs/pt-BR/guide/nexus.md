---
description: "Leia matrizes NEXUS morfológicas em Rust, preserve a semântica das observações e comprima colunas repetidas dentro de partições de modelo explícitas."
---

# Lendo NEXUS morfológico

`nexus::parse` lê uma string UTF-8 e produz uma `data::Matrix`. Ele preserva a ordem dos táxons, o alfabeto de estados declarado, os rótulos de caracteres e de estados e cada célula observada. Não tem dependências em tempo de execução e não invoca o BEAST.

Experimente a matriz de referência fornecida, a partir da raiz do repositório:

```sh
cargo run --release --example read_nexus -- fixtures/reference/20x50/matrix.nex
```

```text
20 taxa, 50 characters, 2 declared states
23 observed patterns (50 total weight)
```

Este exemplo trata todos os caracteres como uma única partição e rejeita gaps (lacunas) e polimorfismo ao preparar as entradas de verossimilhança. Ele importa dados; não executa inferência.

## Sintaxe suportada

Entrada inválida ou não suportada devolve `nexus::Error` com linha e coluna contadas a partir de 1 e uma explicação.

| Recurso | Suporte |
| --- | --- |
| Blocos | Um bloco `DATA` ou `CHARACTERS`, opcionalmente precedido por um bloco `TAXA`. Blocos `TREES` são ignorados; árvores não são importadas. Outros blocos são rejeitados. |
| Dimensões | `NTAX` e `NCHAR` positivos. Um bloco `TAXA` anterior pode fornecer `NTAX`. `NEWTAXA` exige `NTAX`. Todos os táxons declarados devem ocorrer exatamente uma vez na matriz. |
| Formato | `DATATYPE=STANDARD`, `SYMBOLS`, `MISSING`, `GAP`, `RESPECTCASE`, `LABELS`, `NOTOKENS` e `INTERLEAVE` ou `INTERLEAVE=YES/NO`. |
| Alfabeto | Pelo menos dois símbolos ASCII válidos, um caractere por estado. Se não for declarado, `01`. A ordem declarada é preservada, incluindo estados não observados. |
| Linhas sequenciais | Linhas nomeadas, com espaços em branco e quebras de linha opcionais entre as células. |
| Linhas intercaladas | Cada página repete todos os rótulos de táxons na mesma ordem. Cada linha da matriz ocupa uma única linha de texto, com o mesmo número de caracteres dentro de uma página. As larguras das páginas podem diferir. Linhas em branco são opcionais. Páginas de continuação sem rótulos não são suportadas. |
| Nomes | Nomes entre aspas simples, apóstrofos duplicados e nomes sem aspas. Sublinhados viram espaços. A correspondência e a detecção de duplicatas ignoram maiúsculas e minúsculas ASCII; os nomes devolvidos mantêm a grafia original. |
| Rótulos | `CHARLABELS` com exatamente `NCHAR` nomes; `STATELABELS` e `CHARSTATELABELS` indexados. Rótulos descritivos de estados não reduzem o alfabeto declarado. |
| Comentários | Comentários aninhados entre colchetes, incluindo comentários dentro de palavras. Finais de linha LF, CRLF e CR são aceitos. |

Com `TAXLABELS`, as linhas devolvidas seguem a ordem declarada mesmo que as linhas da matriz apareçam em outra ordem. Caso contrário, a primeira página da matriz define a ordem.

Sem declaração em contrário, os estados não distinguem maiúsculas de minúsculas, o símbolo de faltante é `?`, não há símbolo de gap e as linhas são sequenciais. Assim, `-` não é um gap a menos que seja declarado com `GAP=-`. Os símbolos de estado, de faltante e de gap devem ser distintos sob a política de maiúsculas e minúsculas escolhida.

Tipos de dados moleculares ou contínuos, `TOKENS`, `TRANSPOSE`, `NOLABELS`, `EQUATE`, `MATCHCHAR`, intervalos de estados, frequências/contagens, `ELIMINATE`, múltiplas matrizes e comandos não suportados são rejeitados. `SETS`, `ASSUMPTIONS` e blocos específicos de aplicativos também são rejeitados: suas partições, pesos, ordenação ou exclusões não devem desaparecer na importação. O parsing de XML está fora desta API.

## Observações e verossimilhanças dos terminais

A distinção entre chaves e parênteses segue a [especificação NEXUS](https://doi.org/10.1093/sysbio/46.4.590). Para um alfabeto declarado `012`:

| Entrada | Célula armazenada | Vetor de verossimilhança do terminal |
| --- | --- | --- |
| `1` | `Cell::State(1)` | `[0, 1, 0]` |
| `{01}` | `Cell::Uncertain([0, 1])` | `[1, 1, 0]` |
| `(01)` | `Cell::Polymorphic([0, 1])` | Rejeitada na conversão para verossimilhança |
| `?` | `Cell::Missing` | `[1, 1, 1]` |
| Gap declarado | `Cell::Gap` | Rejeitada, ou `[1, 1, 1]` com `GapTreatment::Missing` explícito |

Incerteza significa um dos estados listados; polimorfismo registra múltiplos estados presentes. O modelo de observação Mk atual não representa polimorfismo. A importação o mantém para inspeção, mas a compressão devolve um erro identificando o táxon e o caractere. Não há conversão automática para incerteza. Gaps também exigem uma escolha explícita, porque gaps morfológicos podem representar inaplicabilidade.

Vetores de incerteza são máscaras de verossimilhança, não distribuições de probabilidade: `{01}` contribui com um para cada estado permitido, sem divisão por dois. Conjuntos de estados são ordenados e deduplicados na importação.

## Compressão de colunas repetidas

`Matrix::compress` mantém o comportamento original: intervalos contíguos, não vazios e começando em zero devem cobrir a matriz uma vez, em ordem; cada partição usa o alfabeto declarado completo. A separação automática por estados e os conjuntos explícitos são descritos abaixo.

```rust
use rove::{data::GapTreatment, nexus};

let input = std::fs::read_to_string("fixtures/reference/20x50/matrix.nex")?;
let data = nexus::parse(&input)?;
let sites = 0..data.character_count();
let partitions = data.compress(std::slice::from_ref(&sites), GapTreatment::Reject)?;
let patterns = &partitions[0];

assert_eq!(patterns.weights().len(), 23);
assert_eq!(patterns.weights().iter().sum::<usize>(), 50);
```

`Patterns::weights` guarda multiplicidades inteiras exatas. `site_to_pattern` mapeia cada caractere original dentro da partição para um padrão, numerado pela primeira ocorrência. `tip_partials` contém valores em ordem pattern-major para cada táxon, prontos para `LikelihoodCore::set_tip_partials`. Converta os pesos inteiros para `f64` ao chamar o `log_likelihood` do núcleo.

A compressão compara as observações completas das células, incluindo seu tipo. Células faltantes e incertas permanecem como padrões separados mesmo quando suas máscaras de verossimilhança coincidem. Colunas idênticas em partições diferentes também permanecem separadas. Isso preserva as observações originais por meio da matriz e do mapeamento, ao custo de, ocasionalmente, manter cálculos redundantes.

Apenas colunas observadas são comprimidas. Colunas constantes e colunas só com faltantes permanecem presentes, e nenhum padrão artificial de ascertainment é adicionado. A [correção Mkv](./kernel#condicionando-a-caracteres-variaveis) acrescenta seus padrões constantes separadamente e rejeita caracteres que poderiam ser constantes.

Os testes leem as duas [fixtures 20 × 50](./fixtures), reconstroem suas colunas a partir do mapeamento de padrões e comparam verossimilhanças comprimidas e não comprimidas em matrizes geradas com dados incertos e faltantes.

## Partições por número de estados

`Matrix::partition_by_states(gaps)` agrupa os caracteres pelo número de estados distintos nas observações. Conjuntos explícitos de incerteza contribuem com seus estados possíveis; dados ausentes e lacunas não acrescentam estados. Um caráter com `0`, `2` e `3` tem, portanto, três estados. Cada caráter é recodificado para um alfabeto compacto na ordem dos símbolos declarados. O modelo Mk é não ordenado; os números não indicam uma sequência obrigatória de mudanças.

Um alfabeto declarado de dois estados mantém ambos, inclusive em caracteres constantes ou só com dados ausentes. Com um alfabeto maior, uma coluna com menos de dois estados observados/possíveis não determina seu modelo: a partição automática a rejeita. Use `CharacterSet { sites, symbols }` com `Matrix::compress_sets`, ou `--partitioning declared` se todos os estados da matriz forem possíveis para cada caráter. `CHARSTATELABELS` continua descritivo e não define esses alfabetos. Blocos NEXUS `SETS` continuam rejeitados.

`compress_sets` aceita índices de caracteres ordenados, não necessariamente contíguos, começando em zero. Os conjuntos devem cobrir cada caráter uma única vez e incluir todos os estados observados/possíveis. `Patterns::sites()` lista as posições originais; `state_symbols()` registra os símbolos originais de cada caráter na ordem do modelo. A compressão ocorre após a recodificação, dentro de cada partição, mantendo o peso total dos caracteres.

`rove inspect` e `rove sample` usam `--partitioning observed` por padrão; ambos aceitam `--partitioning declared`. O app mostra essa escolha em **Partições por estados**, junto da quantidade de caracteres de cada partição. Lacunas continuam exigindo uma escolha explícita. Mkv ainda rejeita qualquer caráter que possa ser constante, mesmo que seu número de estados seja conhecido.

Cada partição tem seus próprios padrões constantes e correção Mkv, compartilhando árvore, relógio e categorias gama. `run.json` registra modo, número de estados, índices de caracteres começando em um, mapeamentos e quantidade de padrões observados. Os checkpoints preservam os caches de todas as partições. Doze casos com estado fixo, incluindo três escalas de comprimento de ramo, concordam com BEAST 2.7.8 / MM 1.2.1 com diferença absoluta máxima de log-verossimilhança de `1,34e-9`; veja o [caso de validação](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/multistate).
