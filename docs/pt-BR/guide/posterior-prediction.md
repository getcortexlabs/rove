---
description: "Simule matrizes morfológicas a partir de uma execução concluída do Rove e compare discrepâncias preditivas posteriores."
---

# Checagens preditivas posteriores

`rove predict` verifica se o modelo ajustado consegue gerar matrizes com propriedades parecidas com as dos dados observados. Usa árvores e parâmetros da posterior, gera novos caracteres e compara a matriz original com a simulada na **mesma árvore amostrada**. Isso avalia aspectos da adequação do modelo; não comprova convergência nem fornece a probabilidade de o modelo ser verdadeiro. O [tutorial de morfologia do RevBayes](https://revbayes.github.io/tutorials/pps_morpho/) apresenta a abordagem.

O recurso está disponível pelo CLI em Rust. Ainda não há uma tela nova no aplicativo para Mac. Não exige instalação de BEAST, Java, R ou RevBayes.

## Rodar uma checagem

Primeiro examine os diagnósticos de amostragem e convergência. Depois informe a pasta de uma execução concluída, com `run.json`, `trace.log` e `trees.nexus`, junto do NEXUS original exato:

```sh
cargo run --release --bin rove -- predict runs/seed-1 \
  --matrix fixtures/reference/20x50/matrix.nex \
  --output runs/prediction-1 --burnin-percent 25 \
  --replicates 100 --seed 42
```

Use a matriz que entrou na análise, incluindo qualquer seleção de táxons feita no app. Seu SHA-256 precisa corresponder ao registro. Em análises paralelas, escolha a pasta de uma cadeia fria concluída; este comando não combina cadeias. Pastas de saída existentes, execuções só do prior, arquivos incompletos e estados incompatíveis entre trace e árvores são rejeitados. Os arquivos de origem são copiados para a nova pasta de saída e permanecem intactos.

O burn-in descarta `floor(total de amostras × porcentagem / 100)` linhas, contando `STATE_0` no total. Cada réplica sorteia uma linha retida da posterior uniformemente, **com reposição**, e recebe uma semente registrada. Linhas repetidas são esperadas. A verossimilhança ajustada é reconstruída para cada linha escolhida antes da simulação, conferindo a combinação da topologia, taxas dos ramos e forma da gama.

## Regras do modelo e das observações

- Preservar o alfabeto e a ordenação ajustados de cada caráter, mesmo quando algum estado possível não aparece na réplica. Caracteres ordenados mantêm os estados intermediários. Não deduzir novos alfabetos das observações simuladas.
- Usar o relógio estrito ajustado ou as taxas explícitas do relógio relaxado, com a mistura gama de quatro categorias ajustada quando presente.
- Para Mkv e Mkparsinf, rejeitar **caracteres latentes completos** excluídos pelo modelo e só depois restaurar a máscara original de dados ausentes e gaps. A categoria gama é sorteada novamente a cada tentativa. Uma réplica mascarada pode parecer constante mesmo que o caráter latente seja variável; isso é intencional.
- Aceitar estados únicos, dados ausentes e gaps explicitamente tratados como ausentes. Conjuntos de incerteza e polimorfismo são rejeitados porque falta um modelo gerador dessas observações. Mkparsinf ordenado continua sem suporte, como na inferência.

Eventos de condicionamento raros podem exceder o limite padrão de 100.000 tentativas por caráter. O comando falha sem publicar um resultado parcial; `--max-attempts` permite elevar esse limite até 10.000.000. O cancelamento pelo processo dono (`--cancel-on-stdin-close`) também remove a saída parcial. Encerrar o processo à força pode deixar uma pasta incompleta; só um `predictive.json` completo indica uma checagem terminada. Há limites para réplicas, tamanho das saídas e memória de trabalho.

## Ler o resultado

`predictive.json` registra modelo, identificadores dos arquivos e da compilação, burn-in, sementes, alfabetos fixados e estimativas de cauda globais e por caráter. `replicates.jsonl` liga cada matriz ao estado de origem e à semente, com as estatísticas pareadas. `characters.tsv` contém os escores pareados por caráter. `matrices/` contém réplicas NEXUS legíveis; interprete seus alfabetos usando `frozen_character_sets` do relatório.

A parcimônia usa escores de Sankoff em cada árvore posterior, sem procurar outra topologia ótima. Mudanças não ordenadas custam um; ordenadas custam `|i − j|`. Para cada caráter, `s` é o escore na árvore, `m` o mínimo possível e `g` o máximo possível. A consistência global é `Σm / Σs`; a retenção é `(Σg − Σs) / (Σg − Σm)`. Denominadores zero são indefinidos, não zero. A contagem de caracteres informativos usa `g > m`, considerando os custos ordenados. Observações ausentes não têm custo de mudança.

O ajuste por caráter também inclui o **negativo do logaritmo da verossimilhança não condicionada do padrão** no estado posterior pareado, com a máscara original de ausências. É uma estatística de discrepância, não uma densidade preditiva condicionada. A seleção de caracteres é aplicada na simulação.

`p_upper` é a fração dos pares válidos em que a estatística simulada é maior ou igual à observada; `p_lower` usa menor ou igual. Empates entram nas duas caudas e são registrados separadamente. Pares com CI/RI indefinidos são contados e excluídos. Frações pequenas indicam uma discrepância a investigar; não há limite automático de aprovação ou reprovação. Checagens por caráter são exploratórias, sem ajuste para testes múltiplos.

Os intervalos de Wilson de 95% descrevem o erro de simulação **condicionado à posterior empírica finita usada como origem**. Não incluem o erro da MCMC. Aumentar as réplicas não corrige uma posterior mal amostrada, e um resultado não extremo não prova adequação.

## Validação

Os testes comparam frequências simuladas com probabilidades independentes em forma fechada em árvores de quatro terminais, incluindo misturas gama condicionadas e máscaras de ausências. A enumeração exaustiva de estados ancestrais e topologias de quartetos confere parcimônia, consistência, retenção e verossimilhanças para caracteres ordenados e não ordenados. Os testes do CLI cobrem os dois relógios, todos os condicionamentos suportados, reprodução, arquivos incompatíveis, cancelamento e rejeição de resultados parciais.
