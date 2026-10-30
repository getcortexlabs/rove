---
description: "O Rove é um projeto de inferência filogenética em Rust, pensado primeiro para ARM. Saiba por que ele começa com um kernel de verossimilhança para dados morfológicos, o que existe hoje e o que não existe."
---

# O que é o Rove?

O Rove é um projeto de inferência filogenética escrito em Rust. Ele começa com um kernel de verossimilhança para dados morfológicos, projetado primeiro para CPUs ARM. O objetivo é a inferência independente: executar uma análise desde a matriz de caracteres até uma amostra de árvores sem depender do BEAST.

O nome vem de explorar, vagar e descobrir caminhos.

::: info Status do projeto
Estágio inicial. O núcleo de verossimilhança com o caminho direto do Mk e um kernel NEON para ele, a correção Mkv, o leitor de NEXUS, a compressão de padrões, os testes numéricos e um amostrador independente mínimo estão implementados. Um experimento de referência com o BEAST 2.7.8 está registrado. O [modelo de referência](./reference-model) com relógio relaxado está implementado e conferido em estados fixos. Comparações de posterior cobrem seis casos construídos com 20 táxons. O [benchmark ampliado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/expanded/2026-10-30/README.md) cobre 20, 90 e 500 táxons e compara adaptadores de núcleo do BEAST/BEAGLE, separando tempo e qualidade da amostragem. As execuções maiores não atingiram os critérios de convergência. Um caminho AVX2 continua como trabalho futuro. O trabalho é acompanhado nas [issues](https://github.com/getcortexlabs/rove/issues).
:::

## Por que outro kernel

Espaços de estados pequenos e alinhamentos morfológicos curtos merecem ser medidos como uma carga de trabalho própria.

- O Mk tem forma fechada, `P(t) = e·I + (1 − e)·π`, o que permite uma atualização de filho em O(k) por padrão em vez de uma multiplicação completa de matriz em O(k²). O núcleo Mk do Rove faz a poda diretamente com ela.
- Vetorização específica para ARM e reaproveitamento de cálculos repetidos são otimizações candidatas. Seu valor precisa de benchmarks pareados, incluindo os custos de preparação e de atualização parcial.
- A referência registrada com o BEAST 2.7.8 usou o núcleo de verossimilhança em Java do próprio BEAST. Essa observação não estabelece qual backend é usado por toda versão, instalação ou modelo do BEAST.

O Rove, em vez disso, parte dos dados: um layout que vetoriza ao longo dos padrões de sítio e caminhos rápidos para os modelos que os morfologistas usam. A página de [design](./design) percorre as decisões.

## O que existe hoje

O motor é uma única crate Rust, sem dependências em tempo de execução. Um pacote Swift
separado oferece a [prévia nativa para Mac](./macos-preview) para importar matrizes, revisar configurações, executar análises e acompanhar o progresso.

- `LikelihoodCore`: poda de Felsenstein sobre um layout pattern-major, com categorias de taxa, reescalonamento opcional por nó e store/restore do estado dos nós para um amostrador. `LikelihoodCore::mk` faz a poda com a forma fechada do Mk; `LikelihoodCore::new` multiplica uma matriz de transição por ramo.
- `Backend`: código portátil para qualquer alvo e, em aarch64, um kernel NEON para a atualização de nó do Mk, que dá os mesmos resultados. Veja [Backends](./kernel#backends).
- `model::Mk`: o modelo Mk com sua matriz de transição em forma fechada, para frequências de estados uniformes ou personalizadas, que são validadas.
- `model::Ordered`: mudanças entre estados vizinhos com frequências uniformes e partições ordenadas/não ordenadas na mesma análise. Veja [caracteres ordenados](./ordered-characters).
- `ascertainment::VariableCharacters`: a correção Mkv para matrizes de caracteres variáveis, verificada contra dois estados fixos do BEAST 2.7.8.
- `ascertainment::InformativeProbability`: condicionamento Mkparsinf com cache para Mk não ordenado de frequências iguais, conferido por enumeração completa de padrões e BEAST 2.7.8. Veja [seleção de caracteres](./character-selection).
- `reference::Problem`: uma implementação recursiva simples, mantida como referência nos testes.
- `rates::GammaRates`, `rates::RelaxedClock`: [Mk + gama, lognormal](./reference-model).
- `mcmc::Sampler` e o comando `rove sample`: um amostrador de Metropolis–Hastings para caracteres binários e multiestado sob Mk ou Mkv particionado, relógio estrito ou relaxado lognormal e um prior de nascimento–morte para a árvore, com restrições de monofilia. Veja [Amostrando árvores](./sampler).
- `nexus::parse` e `data::Matrix::compress`: entrada morfológica, observações preservadas, partições explícitas e padrões de sítio ponderados. Veja [Lendo NEXUS](./nexus).
- Testes de propriedade que comparam os dois tipos de núcleo com a referência em árvores aleatórias com 2 a 10 terminais, 2 a 6 estados e 1 a 4 categorias de taxa, incluindo comprimentos de ramo de 10⁻⁹ a 100 e sequências de mudanças aceitas e rejeitadas.
- Uma matriz sintética de controle positivo com 20 táxons e 50 caracteres binários, com o script que a gera e variantes degradadas.

A página do [núcleo de verossimilhança](./kernel) mostra como chamá-lo. A página de [matrizes sintéticas](./fixtures) descreve o controle.

## O que ainda não existe

- Modelos de partições definidos pelo usuário, datas de terminais, calibrações e outros priors de árvore.
- Um caminho de código AVX2. Alvos que não são aarch64 rodam o código portátil.
- Adaptadores de produção para BEAST 2 ou BEAGLE. Os adaptadores de benchmark são ferramentas separadas.

Nada desta lista está documentado como disponível em qualquer parte deste site. O [roadmap](./roadmap) dá a ordem.

## BEAST, como referência

O BEAST não é uma dependência. É a referência para validação e benchmarking: as verossimilhanças são comparadas primeiro em estados fixos, depois em execuções completas de inferência. BEAST 2 e BEAST X são bases de código diferentes, e resultados de um não estabelecem resultados do outro, por isso as comparações nomeiam a implementação e a versão contra as quais foram executadas.

A [datação de fósseis](./tip-dating) aceita idades fixas, intervalos e relógio em unidades de tempo pelo CLI.
