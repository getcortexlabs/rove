---
description: "Como executar o primeiro amostrador independente do Rove numa matriz morfológica binária: o modelo, o comando, os arquivos que ele grava, as propostas e as verificações por trás delas."
---

# Amostrando árvores

`rove sample` executa uma análise de Monte Carlo via cadeias de Markov numa matriz morfológica binária, sem o BEAST. O modelo é pequeno de propósito. A variação de taxas gama, o relógio relaxado e os outros componentes da análise de referência estão no [roadmap](./roadmap).

## O modelo

- **Caracteres.** Uma partição de caracteres binários sob Mk com frequências iguais e uma categoria de taxa. Por padrão, a verossimilhança é condicionada a caracteres variáveis com a [correção Mkv](./kernel#condicionando-a-caracteres-variaveis); `--conditioning none` usa o Mk simples.
- **Relógio.** Estrito, com a taxa fixa em 1. As alturas dos nós ficam, portanto, em mudanças esperadas por caractere, a convenção da análise de referência, que não tem calibrações. Todos os terminais estão na altura zero.
- **Prior da árvore.** O prior de nascimento–morte de Gernhard (2008), calculado como o BEAST 2.7 calcula o `BirthDeathGernhard08Model` com `type="unscaled"`: todas as linhagens são amostradas, e a densidade é condicionada ao número de terminais, mas não à altura da raiz. Seus parâmetros são r = λ − μ, com prior Uniform(0, 1000), e a = μ/λ, com prior Uniform(0, 1). Estas são as definições do XML de referência, onde os parâmetros se chamam `BDBirthRate` e `BDDeathRate`.
- **Restrições.** Cada opção `--monophyly` lista táxons que precisam formar um clado. Propostas que violariam uma restrição são rejeitadas. As restrições precisam ser aninhadas ou disjuntas.

## Executando

```sh
cargo build --release
target/release/rove sample fixtures/reference/20x50/matrix.nex \
  --output runs/seed-1 --seed 1 --steps 10000000 --sample-every 1000 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

| Opção | Significado |
| --- | --- |
| `--output <diretório>` | Diretório para os arquivos de saída. Ele ainda não pode existir. |
| `--seed <n>` | Semente do gerador de números aleatórios. Opção obrigatória. |
| `--steps <n>` | Número de passos da cadeia. Opção obrigatória. |
| `--sample-every <n>` | Passos entre amostras; precisa dividir `--steps`. O estado inicial é a amostra 0. Opção obrigatória. |
| `--monophyly <táxons>` | Nomes de táxons, separados por vírgula, que precisam formar um clado; como na matriz, `_` representa um espaço. Pode ser repetida. |
| `--gaps reject\|missing` | Rejeitar matrizes com gaps (o padrão) ou ler os gaps como dados faltantes. |
| `--conditioning variable\|none` | Mkv (o padrão) ou Mk. |
| `--sample-from-prior` | Ignorar os caracteres e amostrar o prior. |
| `--no-tuning` | Manter fixos todos os tamanhos de deslocamento e fatores de escala. |
| `--tuning <movimento>=<valor>` | Tamanho inicial de deslocamento ou fator de escala de um movimento, como `subtree_slide=0.001`. Pode ser repetida. |
| `--print-every <n>` | Passos entre as linhas de progresso na saída de erro padrão; por padrão, um vigésimo da cadeia. |

O comando se encerra com uma mensagem quando recebe algo que não suporta: caracteres com mais de dois estados, partições ou blocos de modelo no arquivo NEXUS, caracteres que poderiam ser constantes sob Mkv, nomes de táxons com caracteres além de letras, dígitos, espaços, `_`, `.` e `-` (ou com espaços e `_` ao mesmo tempo), táxons desconhecidos numa restrição, restrições sobrepostas que não são aninhadas, valores de ajuste fora do intervalo e opções que ele não conhece ou que aparecem duas vezes.

Com o mesmo executável, a mesma entrada, as mesmas configurações e a mesma semente, na mesma plataforma, uma execução produz as mesmas amostras. Os logaritmos e as exponenciais vêm da biblioteca matemática da plataforma, então outra plataforma pode diferir nos últimos dígitos e, a partir daí, divergir.

## Arquivos de saída

- **`trace.log`** é separado por tabulações, depois de linhas iniciadas por `#` que registram a versão, o comando e o SHA-256 da entrada e do executável. Suas colunas são `Sample`, `posterior`, `likelihood`, `prior`, `birth_death` (a log-densidade do prior da árvore), `birth_difference_rate`, `relative_death_rate`, `tree_height` e `tree_length`. O formato é o de um traço do BEAST, e o LogAnalyser do BEAST consegue lê-lo.
- **`trees.nexus`** é um bloco de árvores NEXUS com uma tabela de tradução e uma árvore por amostra, que o NexusParser do BEAST consegue ler. Os espaços nos nomes dos táxons são escritos como `_`. Os comprimentos de ramo estão em mudanças esperadas por caractere.
- **`run.json`** registra os argumentos do comando, a plataforma, o caminho e o SHA-256 da entrada, o SHA-256 do executável calculado no início da execução, o modelo, todas as configurações, o peso, o ajuste e as contagens de aceitação de cada movimento, a posterior final e o tempo decorrido.

## Propostas

Cada movimento segue a definição e a razão de Hastings do operador do BEAST 2.7 indicado na segunda coluna. Os pesos são os da comparação com o BEAST sob relógio estrito.

| Movimento | Operador do BEAST 2 | O que ele muda | Peso |
| --- | --- | --- | ---: |
| `node_height` | `Uniform` | A altura de um nó interno que não é a raiz, uniforme entre o filho mais alto e o pai | 30 |
| `subtree_slide` | `SubtreeSlide` | Desloca um nó para cima ou para baixo da sua linhagem por um passo gaussiano, reenxertando a subárvore quando ele passa por outro nó | 15 |
| `narrow_exchange` | `Exchange`, narrow | Troca um nó com o seu tio | 15 |
| `wide_exchange` | `Exchange`, wide | Troca duas subárvores quando as alturas permitem | 3 |
| `wilson_balding` | `WilsonBalding` | Poda uma subárvore e a reenxerta noutro ramo, nunca na raiz | 3 |
| `root_scale` | `ScaleOperator`, só a raiz | Escala a altura da raiz | 3 |
| `tree_scale` | `ScaleOperator` na árvore | Escala todas as alturas internas | 3 |
| `birth_difference_scale` | `ScaleOperator` | Escala λ − μ | 3 |
| `relative_death_scale` | `ScaleOperator` | Escala μ/λ | 3 |

O tamanho do deslocamento e os fatores de escala são ajustados durante a execução em direção a uma taxa de aceitação de 0,234, com a regra de atualização do BEAST. O ajuste começa depois de 10.000 propostas ajustáveis. Depois da `n`-ésima proposta de um movimento a partir daí, o passo é `(α − 0,234)/√(n + 1)`: é a transformação opcional de raiz quadrada do BEAST, enquanto o padrão do BEAST divide por `n + 1`. Uma proposta inválida, ou que viola uma restrição, conta como falha e entra no ajuste com `α = 0`. Para uma proposta inválida, o BEAST, em vez disso, reaproveita o α do passo anterior. Com `--no-tuning` os valores ficam fixos, o que torna comparáveis as propostas de dois motores.

## Estado e restauração

Antes de cada passo, o amostrador guarda a árvore e os parâmetros de taxa, e registra os índices dos buffers do [núcleo de verossimilhança](./kernel#nos-e-buffers). Depois de uma proposta, ele compara a nova árvore com a guardada. Cada ramo cujo comprimento ou pai mudou recebe a nova distância num buffer novo, e só as parciais dos seus ancestrais são recalculadas. Um passo rejeitado traz de volta a árvore guardada, os parâmetros e os buffers do núcleo. Os movimentos de λ − μ e μ/λ não mexem na verossimilhança.

## Verificando

`tests/sampler.rs` e os testes unitários em `src/mcmc/moves.rs` conferem o amostrador contra resultados conhecidos de antemão:

- **O prior, em cinco terminais.** Sob este prior, toda topologia ranqueada tem a mesma probabilidade, então a frequência de cada topologia é proporcional ao número de maneiras de ordenar seus nós internos. Sendo F a função de distribuição dos tempos de especiação, F(altura da raiz) segue uma Beta(4, 2), e a média de F sobre os nós internos é 5/12. Isso é conferido para os movimentos de altura sozinhos, para todos os movimentos de árvore juntos e com uma restrição de monofilia.
- **Cada movimento de topologia isolado.** Sob o prior de nascimento–morte, baixar um nó sempre aumenta a densidade, então todo deslocamento para baixo é aceito, qualquer que seja o seu termo de Hastings. Um alvo sintético cuja densidade cresce com a altura dos nós, em que cada topologia continua ponderada pelas suas ordenações, confere a razão de Hastings de cada movimento. Tirar as contagens da razão do *subtree slide* faz a sua distância das frequências exatas ficar cerca de três vezes maior.
- **O movimento de escala** mantém um alvo uniforme em [0, 1], e λ − μ e μ/λ mantêm os seus priors uniformes quando a cadeia amostra o prior com todos os movimentos.
- **O ajuste** move cada valor na direção certa e na medida certa, conferido contra um passo calculado à mão.
- **Manutenção do estado.** Numa matriz de seis táxons com células faltantes e incertas, depois de cada passo de cadeias de 4.000 passos sob Mkv e Mk, a posterior guardada coincide com uma avaliação feita do zero, um passo rejeitado restaura exatamente o estado anterior, a restrição continua valendo e todo movimento é aceito e rejeitado ao menos uma vez.
- **O prior da árvore** reproduz o valor do BEAST 2.7.8 nos dois estados arquivados da análise de referência, a menos de 10⁻¹².
- **Estados fixos.** No estado final de cada cadeia do BEAST da comparação abaixo, a verossimilhança e a posterior do Rove reproduzem as do BEAST 2.7.8 a menos de 10⁻¹⁰, e o prior da árvore e o prior, a menos de 10⁻¹².

`tests/allocations.rs` confere que 20.000 passos na matriz de referência não fazem nenhuma alocação no heap. `tests/cli.rs` executa o comando, confere os três arquivos e as configurações rejeitadas, e confere que fechar a saída de erro padrão não interrompe uma execução.

## Comparação com o BEAST

O Rove e o BEAST 2.7.8 amostraram a matriz de referência sob este modelo, com e sem a correção Mkv, usando os mesmos movimentos, pesos e tamanhos de passo fixos, com quatro cadeias cada. Toda média posterior ficou a no máximo quatro erros-padrão de Monte Carlo combinados da média do BEAST, e toda frequência de clado a no máximo 0,05, os critérios fixados antes das execuções. As taxas de aceitação não faziam parte dos critérios; com 36 cadeias Mkv por motor, nenhum movimento diferiu em mais de 1,7 erro-padrão.

A matriz não tem homoplasia, então sob Mkv a verossimilhança não penaliza árvores curtas: a altura posterior da árvore fica em cerca de 0,003 mudanças esperadas por caractere, a posterior de λ − μ é cortada pelo limite superior do seu prior, 1000, e a topologia quase não se move. Sob Mk, a topologia de referência tem frequência posterior entre 0,82 e 0,83 em todas as cadeias dos dois motores. O [relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md) traz os números, os tempos de execução e os comandos.

## Limitações

- Uma partição de caracteres binários. Variação de taxas gama, relógios relaxados, datas de terminais, calibrações e outros priors de árvore não estão implementados; os componentes da análise de referência são acompanhados na [#16](https://github.com/getcortexlabs/rove/issues/16).
- A árvore inicial é aleatória, com tempos de nós parecidos com os de um coalescente, e satisfaz as restrições.
- Uma cadeia, numa thread. Uma execução não pode ser retomada de um checkpoint.
- O comando não calcula diagnósticos de convergência. Os scripts do repositório em `tools/beast` os calculam a partir da saída dele.
