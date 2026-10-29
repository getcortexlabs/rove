---
description: "A ordem de implementação do Rove: estabelecer a corretude e baselines medidos, depois otimizar o kernel e construir a inferência independente."
---

# Roadmap

O Rove está em estágio inicial. Não há releases, nem crate publicada, nem datas. Esta é a ordem de implementação, não um cronograma.

| Etapa | Estado |
| --- | --- |
| Núcleo de verossimilhança escalar e testes numéricos de referência | Concluído |
| CI em ARM64, x86_64 e na versão mínima do Rust | Concluído |
| Análise de referência reprodutível com o BEAST 2.7.8 | Concluído |
| NEXUS morfológico e compressão de padrões observados | Concluído |
| Baseline do núcleo Rust | Concluído, #8 |
| Benchmarks de inferência completa e entre motores | Comparação com 20 táxons registrada; ampliação pendente, #8 |
| Correção de ascertainment Mkv | Concluído |
| Poda direta do Mk | Concluído |
| MCMC independente mínimo | Concluído |
| Vetorização NEON | Concluído |
| Modelo de referência com relógio relaxado | Concluído, #16 |
| Validação da inferência completa | Concluído em seis casos de 20 táxons, #12 |
| Eficiência da amostragem com relógio relaxado | Próximo, #27 |
| Prévia nativa para importar e preparar análises | Implementado, #28 |
| Execução e progresso no aplicativo | Implementado, #29 |
| Histórico de execuções e reutilização de configurações | Implementado, #30 |
| Escolha explícita do grupo externo e restrições aplicadas | Implementado, #31 |
| Resumo MCC posterior e exportação anotada | Implementado, #32 |
| Cadeias paralelas e relatórios de convergência | Planejado, #23 |
| Reescalonamento por potências de dois | Planejado, #18 |
| Checkpoints e retomada de execuções | Planejado, #22 |
| Partições por número de estados | Planejado, #19 |
| Condicionamento a caracteres informativos para parcimônia | Planejado, #20 |
| Caracteres ordenados | Planejado, #21 |
| Verossimilhanças marginais para comparar modelos | Planejado, #24 |
| Prior de nascimento–morte fossilizado com datas de terminais | Planejado, #25 |
| Checagens preditivas posteriores | Planejado, #26 |
| Avaliação de repetições dentro de subárvores | Planejado, #17 |
| Adaptadores opcionais BEAST 2 JNI e BEAGLE C ABI | Possível, #6 e #7 |

## Estabelecer o baseline

O [núcleo](./kernel) é verificado contra uma implementação escalar independente. O [leitor de NEXUS](./nexus) fornece observações ponderadas sem o BEAST. O [experimento de referência](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra entradas exatas, configurações de modelo, duas execuções completas do BEAST e estados numéricos fixos.

O [baseline no Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 cenários em Rust, separando preparação, avaliações completas, atualizações parciais e manutenção do estado das propostas. Inclui amostras brutas, observações de alocação e variação entre três processos. Uma [comparação do modelo completo](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md) agora registra tempo e ESS por segundo em seis casos de 20 táxons; cenários maiores continuam pendentes. A comparação sob relógio estrito, mais abaixo, registra o tempo decorrido dos dois motores numa configuração pequena, mas não é um benchmark controlado.

## Corretude antes da otimização

O Mkv condiciona a verossimilhança ao processo de observação em matrizes só com caracteres variáveis. Ele reproduz os dois estados arquivados do BEAST, e seus padrões constantes ficam separados dos pesos dos padrões observados.

O caminho direto do Mk substitui o produto genérico de matrizes nos modelos Mk. Num Apple M4 Max, o caminho de matrizes levou 1,25 vez o tempo do caminho Mk numa avaliação completa da matriz de referência, e 2,66 vezes com oito estados ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md)). Depois, o backend NEON fundiu a atualização de nó do Mk numa única passada por blocos de oito padrões. Na mesma máquina, o código portátil levou 1,48 vez o tempo do NEON numa avaliação completa da matriz de referência, de 1,10 a 1,32 vez nos casos 90 × 250 com 250 padrões e de 1,08 a 1,11 vez numa execução do amostrador ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). O logaritmo do reescalonamento, uma chamada escalar por padrão e por nó, passou a ocupar cerca de dois quintos do tempo das atualizações de nó numa execução do amostrador. A implementação portátil continua como referência e fallback. Cada otimização precisa de uma medição antes/depois, não só de uma contagem de operações aritméticas.

## Inferência independente

O [amostrador mínimo](./sampler) suporta Mk ou Mkv binário, um relógio estrito e o prior de nascimento–morte. Com os mesmos movimentos e passos fixos do BEAST 2.7.8, as suas médias posteriores e frequências de clados na matriz de referência atenderam a critérios fixados antes das execuções ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). O [modelo de referência](./reference-model), com categorias gama e relógio relaxado lognormal, agora reproduz os dois estados arquivados componente por componente. Os resumos de posterior e as frequências de clados atenderam aos critérios definidos antes das corridas em seis casos construídos com 20 táxons ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)). A validação avança das verossimilhanças em estados fixos até execuções completas em matrizes conhecidas. A qualidade da amostragem, incluindo ESS por tempo e diagnósticos de árvore, conta junto com o tempo de execução.

Depois da eficiência da amostragem (#27), os recursos planejados seguem o que as análises morfológicas precisam: cadeias paralelas com relatórios de convergência (#23), checkpoints (#22), partições por número de estados (#19), condicionamento a caracteres informativos para parcimônia (#20), caracteres ordenados (#21), verossimilhanças marginais para comparar modelos (#24), o prior de nascimento–morte fossilizado com datas de terminais (#25) e checagens preditivas posteriores (#26). O RevBayes documenta cada um deles para dados morfológicos e pode servir de segunda referência onde o BEAST não tiver um recurso equivalente.

A [prévia nativa para Mac](./macos-preview) importa e exibe matrizes e prepara as configurações da análise. Ela executa o motor Rust incluído com progresso real, cancelamento e reabertura de configurações com verificação da impressão digital. Os adaptadores BEAST e BEAGLE são caminhos de compatibilidade opcionais, não dependências da engine em Rust.

Acompanhe as [issues do repositório](https://github.com/getcortexlabs/rove/issues) para critérios de aceitação e resultados.
