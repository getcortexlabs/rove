---
description: "O modelo independente do XML fornecido: categorias gama, taxas lognormais explícitas nos ramos, priors e validação numérica."
---

# O modelo de referência

`rove sample --model reference` executa o modelo probabilístico do XML
20 × 50 fornecido, sem o BEAST. Usa o mesmo Mk binário, a correção Mkv por
padrão, o prior de nascimento–morte e as convenções dos parâmetros. A opção
padrão `--model strict` mantém a configuração anterior, com uma categoria e
relógio estrito. Ambas aceitam `--conditioning none` para Mk sem correção.

## Taxas e priors

| Componente | Configuração de referência |
| --- | --- |
| Taxas dos caracteres | Quatro quantis centrais da gama, cada um com peso 1/4, normalizados para média um |
| Forma da gama | Estimada; Exponential(média=1), começando em 1; limites [0,001; 1000] |
| Taxas dos ramos | Uma taxa explícita por ramo fora da raiz; LogNormal(μ=−σ²/2, σ) independentes, começando em 0,5; taxas positivas |
| Desvio padrão σ do relógio | Desvio padrão dos logaritmos das taxas; Gamma(forma=5, **escala**=0,2), começando em 0,2 |
| Multiplicadores de mutação e relógio | Ambos fixos em 1 |
| Parâmetros de nascimento–morte | λ−μ ~ Uniform(0,1000), μ/λ ~ Uniform(0,1), começando em 1 e 0,5 |
| Prior da árvore | Gernhard, `unscaled`, todas as linhagens amostradas, condicionado ao número de terminais, não à altura da raiz |
| Monofilia | Informada explicitamente por `--monophyly`; o comando de referência restringe T1–T19 |

Os quantis gama ficam em 1/8, 3/8, 5/8 e 7/8; não são as médias de quatro
intervalos de probabilidade. Os limites da forma também valem ao amostrar
o prior; como no BEAST, sua densidade registrada não acrescenta uma constante
de normalização pelo truncamento. Sigma é o desvio padrão dos **logaritmos
das taxas**, não a média nem o desvio das taxas no espaço real. A média da
distribuição lognormal é um; as taxas dos ramos não são normalizadas para
que sua média realizada seja um.

O vetor de taxas tem `2 × táxons − 2` entradas. Um nó fora da raiz usa seu
índice; se o último nó não for a raiz, usa o índice da raiz. É o mapeamento
de taxas explícitas do BEAST, inclusive quando um deslocamento de subárvore
muda a raiz. A raiz não tem taxa de ramo. A distância genética de um ramo
na categoria `c` é `diferença de alturas × taxa do ramo × taxa da categoria[c]`.

Ao iniciar o relógio, o BEAST troca o limite inferior de taxa de ramo
1e-100 do XML por zero. O Rove aceita toda taxa positiva e finita; zero
tem densidade lognormal nula.

## Executar a matriz fornecida

Na raiz do repositório:

```sh
cargo build --release --locked
sh fixtures/reference/20x50/rove.sh runs/reference-1 1
```

O script roda dez milhões de passos, amostra a cada mil e aplica a restrição
T1–T19. O diretório de saída ainda não pode existir. Equivale ao comando de
[Amostrando árvores](./sampler), acrescentando `--model reference`. O suporte
um da restrição é imposto, não recuperado dos dados.

## Propostas e saída

Os nove movimentos clássicos do guia mantêm seus pesos. Entram três movimentos
de escala: `gamma_shape_scale` (peso 0,1), `clock_sigma_scale` (3) e
`branch_rate_scale` (20), que escolhe uma taxa uniformemente. Cada um começa
com fator 0,5 e inclui a razão de Hastings da escala. Usam a mesma regra de
ajuste dos demais movimentos de escala.

São diferentes das misturas adaptativas do ORC, dos kernels Bactrian, dos
movimentos conjuntos de tempo e taxa e dos movimentos de época ou alongamento
da árvore. O Rove também omite a proposta de taxa de mutação do XML, que não
tem efeito em uma dimensão. Reproduzir o modelo probabilístico não estabelece
a mesma mistura da cadeia nem o mesmo ESS por segundo.

O traço acrescenta `gamma_shape`, `gamma_prior`, `clock_sigma`,
`clock_rate_prior`, `clock_sigma_prior` e `mean_branch_rate` (média aritmética
sem ponderação). As árvores mantêm comprimentos temporais e anotam cada ramo
com `[&rate=...]`; comprimento vezes taxa dá a distância antes do multiplicador
do caractere. As alturas são relativas ao multiplicador fixo do relógio,
sem calibração em tempo de calendário. `run.json` registra a configuração,
priors, valores iniciais, mapeamento e taxas finais. Não é um checkpoint para
retomar a cadeia.

## Validação

`tests/rates.rs` reconstrói os dois estados arquivados do BEAST 2.7.8 a partir
das alturas, forma gama, sigma e taxas explícitas. A maior diferença absoluta
na log-verossimilhança fica abaixo de 8e-10; cada componente do prior coincide
a menos de 2e-12. As categorias são calculadas pelo Rove, não importadas do BEAST.

O Rove inverte a função de distribuição gama usando [sua série positiva](https://dlmf.nist.gov/8.7.E1)
e passos de Newton com limites, em espaço logarítmico. O BEAST usa sua
aproximação antiga AS91. A maior diferença nas categorias, na grade arquivada
de 20 formas, fica abaixo de 1,4e-8; isso explica a tolerância maior que a dos
testes que importam as taxas prontas. Fórmulas fechadas da exponencial e de
Erlang fornecem verificações numéricas separadas.

Outros testes conferem os hiperpriors por amostragem sem dados, o mapeamento
com raízes diferentes, cálculos guardados contra avaliações do zero a cada
passo, restauração após rejeição, amostras idênticas nos dois backends e ausência
de alocações durante os passos testados.

Os resumos de posterior e as frequências de clados atenderam aos critérios definidos
antes das corridas em seis casos construídos com 20 táxons, incluindo homoplasia, dados
ausentes e sinal reduzido. O
[relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)
registra todas as baterias, inclusive a extensão do Mkv exigida pelo critério de ESS, os
tempos e o ESS por segundo. O Rove terminou os cinco casos de Mk sem correção em menos
tempo, mas o BEAST entregou mais ESS por segundo para a densidade log-posterior e o
sigma do relógio. Melhorias nas propostas estão na
[#27](https://github.com/getcortexlabs/rove/issues/27).

Seis estados fixos adicionais cobrem esses casos. Com as taxas de categoria importadas,
a diferença máxima de verossimilhança foi 1,14e-13; gerando as categorias de forma
independente, chegou a 8,16e-8, pela mesma diferença de aproximação dos quantis descrita
acima.
