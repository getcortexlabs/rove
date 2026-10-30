---
description: "Date fósseis com idades fixas ou intervalos, escala de tempo explícita e prior de nascimento–morte fossilizado."
---

# Datação de fósseis nas pontas

`rove sample --dating settings.json` acrescenta idades de fósseis e uma escala de tempo à análise morfológica. Funciona com relógios estrito e relaxado, partições, correção de seleção dos caracteres, cadeias independentes e checkpoints. A configuração é pelo CLI Rust; a tela de preparação do Mac ainda não tem controles de datação. Não precisa de BEAST nem Java.

## Idades e unidades

A idade conta **do presente para o passado**: zero é um táxon atual; 12 significa 12 unidades antes do presente. No exemplo, `time_unit: "Ma"` significa milhões de anos. Idades, alturas dos nós, origem e duração dos ramos devem usar essa mesma unidade. O Rove registra o nome, mas não converte unidades.

A taxa do relógio indica mudanças esperadas por caractere por unidade de tempo. As mudanças esperadas em um ramo são sua duração × taxa global do relógio × taxa relativa do ramo. O relógio relaxado acrescenta taxas relativas com média da distribuição igual a um; as categorias gama também multiplicam a taxa do caractere. Sem datação, os modelos existentes continuam com relógio unitário e pontas na idade zero.

Todo táxon precisa de uma idade explícita. Um número fixa a idade; `[mínimo, máximo]` permite estimá-la dentro do intervalo. Intervalos positivos representam incerteza na **idade de um espécime**, não o período entre a primeira e a última ocorrência de uma espécie. Essa distinção segue o [tutorial de espécimes do RevBayes](https://revbayes.github.io/tutorials/fbd/fbd_specimen). O intervalo não pode incluir o presente; `[12,12]` é fixo. Os nomes precisam corresponder aos rótulos lidos do NEXUS, onde um sublinhado sem aspas vira espaço.

## Experimente o exemplo construído

```sh
cargo run --release --bin rove -- sample fixtures/validation/dating/matrix.nex \
  --dating fixtures/validation/dating/dating.json \
  --output runs/dated-example --seed 25 --steps 100000 --sample-every 100
```

O exemplo verifica o software. Não é uma análise biológica nem uma garantia de convergência. O JSON contém:

```json
{
  "time_unit": "Ma",
  "birth_rate": 0.08,
  "death_rate": 0.03,
  "fossil_sampling_rate": 0.02,
  "present_sampling": 0.7,
  "origin": 50,
  "clock_rate": 0.01,
  "clock_prior_mean": 0.01,
  "ages": {
    "Living A": 0,
    "Fossil B": [5, 9],
    "Living C": 0,
    "Fossil D": 12
  }
}
```

Nascimento, morte e amostragem de fósseis são taxas por linhagem por unidade de tempo. present_sampling é uma probabilidade, não uma taxa. origin é a idade em que começou a linhagem ancestral, anterior à raiz e a todas as idades permitidas nas pontas. Esses cinco valores ficam fixos. Escolha-os conforme a pergunta científica e examine a sensibilidade a eles; os valores do exemplo não são padrões biológicos.

Com clock_prior_mean, clock_rate é o valor inicial e o relógio é estimado com prior exponencial daquela média. Omita clock_prior_mean para fixar clock_rate. Use `--model reference` para categorias gama e relógio relaxado, ou `--sample-from-prior` para examinar o prior conjunto sem a verossimilhança dos caracteres.

## O que o prior pressupõe

O processo FBD inclui nascimento, extinção, amostragem de fósseis e amostragem incompleta no presente. Esta implementação restringe as árvores a **fósseis terminais**, sem ancestrais amostrados. Amostrar um fóssil não obriga a linhagem a morrer: a probabilidade de remoção é zero. Origem e parâmetros do processo ficam fixos para que a normalização desse espaço restrito permaneça constante durante a amostragem.

A log-densidade FBD segue a convenção de origem fixa, sem condicionamento à sobrevivência, do [BEAST 2 SA](https://www.beast2.org/xml/sa.evolution.speciation.SABirthDeathModel.html), nos estados validados. É uma **densidade restrita não normalizada**, inadequada para verossimilhanças marginais ou fatores de Bayes. Esta versão não estima parâmetros do processo ou origem, não amostra ancestrais e não modela períodos de ocorrência de espécies. Um intervalo de idade é uma restrição da análise; recuperá-lo não é evidência independente para a idade do fóssil.

## Arquivos e verificações

O trace contém fossilized_birth_death, clock_rate, dating_parameter_prior e tip_age_1, tip_age_2 etc., na ordem da matriz. As colunas de gama e relógio relativo continuam quando ativadas. Parâmetros fixos têm ESS indisponível, como esperado; a comparação marca fixed_by_model e exclui somente valores comprovadamente fixos da parada automática. Um parâmetro estimado que fique constante continua impedindo a parada.

`run.json` guarda a configuração completa e sua impressão digital. O checkpoint incorpora a configuração, então retomar não depende do JSON original. As árvores preservam alturas explícitas e durações dos ramos. As taxas relativas anotadas não incluem o relógio global, salvo no trace. `rove summarize`, `inspect-tree`, `diagnose`, `compare` e `predict` aceitam essas saídas. Os resumos preservam a unidade quando o registro da execução está disponível. Um arquivo de árvore isolado não identifica sozinho a unidade física de tempo.

A validação compara oito densidades fixas com BEAST 2.7.8 / SA 2.1.1, confere distribuições pequenas do prior com um limite analítico e integração independente de equações diferenciais, e verifica restauração da verossimilhança, troca de unidade, retomada, exportações e propostas sem alocações. Isso verifica o comportamento implementado; não demonstra convergência nem desempenho em dados empíricos de fósseis. O [material de validação](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/dating) registra fórmulas, fontes, versões e comandos de reprodução.
