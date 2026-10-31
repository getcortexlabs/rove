---
description: "Estime log-verossimilhanças marginais a partir de posteriores de potência, por stepping stone e path sampling, e compare modelos com fatores de Bayes."
---

# Comparação de modelos

`rove marginal` estima a log-verossimilhança marginal de um modelo configurado: a probabilidade da matriz sob o modelo, integrando todas as árvores e parâmetros. A diferença entre as log-verossimilhanças marginais de dois modelos é o log do fator de Bayes. A estimativa vem de posteriores de potência: o alvo da cadeia com a verossimilhança elevada a um expoente entre zero, o prior, e um, a posterior. A abordagem segue o [tutorial de seleção de modelos do RevBayes](https://revbayes.github.io/tutorials/model_selection_bayes_factors/bf_intro) e o PathSampler do pacote MODEL_SELECTION do BEAST 2. Nenhum software externo é necessário para executá-la.

O comando está disponível no CLI em Rust. O app para Mac não o expõe.

## Executar uma estimativa

```sh
cargo run --release --bin rove -- marginal fixtures/reference/20x50/matrix.nex \
  --output runs/marginal-mkv --seed 1 --steps 1000000 --sample-every 1000 \
  --powers 32 --alpha 0.3 --burnin-percent 20 \
  --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
```

Valem as opções de modelo do `rove sample`: `--model`, `--conditioning`, `--partitioning`, `--gaps`, `--dating`, `--monophyly`, `--weight`, `--tuning`, `--no-tuning` e `--backend`. Para comparar modelos, execute o comando uma vez por modelo na mesma matriz, com as mesmas restrições, e subtraia as estimativas. `--sample-from-prior`, cadeias paralelas, checkpoints e diagnósticos de parada não estão disponíveis.

`--powers` é o número de expoentes da verossimilhança, incluindo zero e um. Eles são quantis de uma distribuição Beta(α, 1) em espaçamento igual, `β_i = (i / (powers − 1))^(1/α)`; um `--alpha` menor concentra mais expoentes perto de zero, onde a média da log-verossimilhança muda mais rápido. Os valores acima seguem as convenções do PathSampler do BEAST 2. Cada potência executa `--steps` propostas e registra a cada `--sample-every`; os primeiros `--burnin-percent` das amostras de cada potência, em contagem, são descartados. A cadeia vai da potência um até zero, e cada potência continua do estado final da anterior, mantendo o ajuste das propostas.

## Saída

O diretório de saída contém `run.json`, com o modelo, a sequência de potências e as contagens dos operadores, `marginal.json`, com as estimativas, e um `power-NN/trace.log` por potência. Esses traces trazem as colunas de posterior, verossimilhança e prior sem o expoente aplicado, como em um trace comum; o expoente entrou só na razão de aceitação. Árvores não são gravadas.

`marginal.json` reporta duas estimativas com erros padrão:

- **Stepping stone** ([Xie et al. 2011](https://doi.org/10.1093/sysbio/syq085)) soma, sobre potências adjacentes, o log da média de `L^(β_k − β_{k−1})` sob a potência inferior. Seu erro padrão soma a variância de cada termo pelo método delta, usando o tamanho efetivo de amostra dos pesos daquele termo.
- **Path sampling** ([Lartillot e Philippe 2006](https://doi.org/10.1080/10635150500433722)) integra a média da log-verossimilhança sobre o expoente pela regra do trapézio. Seu erro padrão soma a variância de cada média a partir do seu tamanho efetivo de amostra. O erro da própria regra do trapézio não entra, então as duas estimativas podem diferir por mais que seus erros padrão quando a sequência de potências é grosseira.

Para cada potência, o arquivo registra o número de amostras retidas, a média e o desvio padrão da log-verossimilhança, sua ESS e se o limite de defasagem da autocorrelação foi atingido. ESS baixa perto da potência zero é comum: essas cadeias exploram o prior, onde as árvores se movem livremente e a verossimilhança varia muito. Mais passos por potência, ou mais potências, reduzem o erro.

## Como ler o resultado

Seguindo [Kass e Raftery (1995)](https://doi.org/10.1080/01621459.1995.10476572), um log do fator de Bayes acima de cerca de 3 a 5 costuma ser lido como suporte forte ao primeiro modelo. Compare apenas estimativas da mesma matriz, dos mesmos táxons e das mesmas restrições, e repita a estimativa com outras sementes antes de ler uma diferença menor que alguns erros padrão. A verossimilhança marginal depende dos priors, inclusive dos priors de nascimento–morte e do relógio, então um fator de Bayes compara os modelos junto com seus priors.

Mk e Mkv definem verossimilhanças para processos de observação diferentes: Mkv condiciona a que todo caráter seja variável. Um fator de Bayes entre eles compara esses processos além dos modelos; veja [seleção de caracteres](./character-selection).

Com datação de fósseis e processo fixo, a densidade FBD omite um normalizador que depende das configurações do processo, dos terminais e dos limites de idade. Compare modelos datados somente quando essas configurações forem idênticas, para que a constante se cancele. Com parâmetros do processo estimados, o alvo conjunto de amostragem inclui fatores de observação cuja normalização não foi derivada para comparação de modelos; essas comparações não têm suporte. Veja [datação de fósseis](./tip-dating).

## Validação

Os testes conferem os dois estimadores num modelo gaussiano conjugado com amostras exatas, incluindo a calibração dos erros padrão em quarenta réplicas, e contra uma quadratura numa árvore de três táxons com uma altura de nó livre, calculada com as densidades do próprio motor. Uma cadeia na potência zero reproduz passo a passo a cadeia que amostra o prior, ainda avaliando a verossimilhança; expoentes fora de [0, 1] são rejeitados, e os checkpoints registram o expoente em uso. A [comparação com o BEAST 2.7.8](https://github.com/getcortexlabs/rove/blob/main/benchmarks/marginal-likelihood/2026-10-31/README.md) rodou o PathSampler do MODEL_SELECTION 1.6.2 e o `rove marginal` nas configurações de relógio estrito Mkv e Mk da matriz fornecida, com 32 potências, 1.000.000 de passos por potência e três sementes por programa. Todo par Rove–BEAST de estimativas por stepping stone concordou dentro de 2,6 erros padrão combinados, com erros padrão entre 0,20 e 0,35 unidades de log, e o path sampling concordou do mesmo modo; uma única implementação recalculou as estimativas dos dois programas a partir dos seus logs. Isso estabelece concordância entre implementações do mesmo estimador numa matriz pequena, não desempenho de seleção de modelos em dados empíricos.
