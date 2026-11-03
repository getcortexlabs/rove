---
description: "Combine morfologia e DNA alinhado com fontes, correspondência de táxons e taxas relativas explícitas."
---

# Morfologia e DNA juntos

Escolha **Combinar morfologia e DNA** na tela inicial do Mac. Selecione uma matriz
morfológica NEXUS STANDARD e DNA alinhado em NEXUS/FASTA, confira a política para
táxons ausentes e salve uma entrada `.mixed.json`. Na CLI:

```sh
rove combine --morphology morphology.nex --dna alignment.fasta --missing-taxa reject --output study.mixed.json
rove sample study.mixed.json --output combined-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --morphology-rate estimate --morphology-rate-log-sd 1
```

O manifesto incorpora os textos UTF-8 originais e seus hashes SHA-256, sem seguir
caminhos externos. Continua utilizável se os originais mudarem de lugar. Não
sobrescreve arquivos existentes e verifica formato, versão, tipos e hashes ao
abrir. O arquivo contém dados de pesquisa: sua distribuição exige a mesma
permissão necessária para compartilhar as duas fontes.

Os táxons correspondem pelo **nome normalizado exato**, nunca pela posição das
linhas. Vale a conversão existente de sublinhados para espaços. `reject` exige
os mesmos táxons. A escolha explícita `--missing-taxa missing` mantém a união e
preenche ausências com dados faltantes; o app lista os táxons ausentes por fonte.
São necessários ao menos dois táxons compartilhados. Diferenças apenas de
maiúsculas/minúsculas são rejeitadas. Confira outras diferenças de escrita antes
de tratá-las como ausências biológicas. Células faltantes não informam estados;
gaps mantêm a política separada `--gaps reject|missing`. O Rove não alinha sequências.

Cada bloco preserva alfabeto, ordem, rótulos e observações. A morfologia admite
alfabetos observados, declarados ou completos pelos rótulos, caracteres ordenados
ou não e `--conditioning none|variable|informative`. Essas escolhas valem **só
para a morfologia**. O DNA usa JC69 sem correção de amostragem, quatro bases e
todos os sítios, inclusive constantes. Ambiguidades IUPAC são máscaras de bases
possíveis; polimorfismos verdadeiros continuam impedindo a inferência.

Os blocos compartilham topologia, tempos dos ramos, relógio estrito ou lognormal
e parâmetro de forma gama quando habilitado. As categorias são integradas por
sítio. Não há relógios ou formas gama independentes por bloco. O DNA fixa a taxa
relativa em 1. Todas as partições morfológicas usam um multiplicador positivo
comum `r`: distância `tempo × taxa do ramo × r`; no DNA, `tempo × taxa do ramo`.
Isso define a escala relativa, sem transformar comprimentos não datados em
idades de calendário. A datação fóssil mantém relógio global e calibrações explícitos.

Escolha o modelo de taxa no app ou na CLI:

- `--morphology-rate 2.5` fixa a razão nesse valor positivo.
- `--morphology-rate estimate --morphology-rate-log-sd s` estima a razão com
  `log(r) ~ Normal(0, s²)`. O desvio padrão do log é 1 se omitido; a mediana é 1
  e a média é `exp(s²/2)`. Avalie a sensibilidade a essa premissa.

O log prior normalizado é
`−log(r) − log(s) − log(2π)/2 − (log(r)/s)²/2`.
A proposta de escala usa `f ~ Uniform(a, 1/a)` e log da razão de Hastings
`−log(f)`. O movimento `morphology_rate_scale` tem peso inicial 3 na estimação
e não está disponível com razão fixa. O estado fica no `trace.log` e no
checkpoint. A log verossimilhança é a soma dos blocos, com a correção de
amostragem escolhida para a morfologia. Os priors entram uma vez; o prior da
razão é normalizado para a estimativa marginal.

Setups de formato 12 preservam fontes, política de táxons e taxas. Revisão,
histórico, importação da CLI, comparação, retomada e estimativas marginais
conferem essa identidade. A predição reconstrói a razão amostrada e gera réplicas
`.mixed.json`, com alfabetos por bloco e máscaras de dados faltantes. Incerteza
e polimorfismo ainda exigem um modelo de observação para simulação. A inclusão
da matriz na exportação é explícita, inclusive para cópias originais guardadas
pela predição. As simulações exportadas são sintéticas e não substituem as observações.

A validação inclui 36 estados fixos BEAST.base 2.7.8 + MM 1.2.1, soma independente
dos blocos, quadratura do prior e amostragem estacionária, restrições, recálculo
do cache, retomada exata, predição e execução nativa. A tolerância é 1e-7,
incluindo aproximações independentes dos quantis gama. Dados e gerador estão em
`fixtures/validation/mixed` e `tools/beast/MixedStates.java`. São testes de
correção, sem certificação de convergência ou comparação de velocidade.
