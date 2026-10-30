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
| Eficiência da amostragem com relógio relaxado | Avaliada; movimentos conjuntos opcionais, padrão clássico mantido, #27 |
| Prévia nativa para importar e preparar análises | Implementado, #28 |
| Execução e progresso no aplicativo | Implementado, #29 |
| Histórico de execuções e reutilização de configurações | Implementado, #30 |
| Escolha explícita do grupo externo e restrições aplicadas | Implementado, #31 |
| Resumo MCC posterior e exportação anotada | Implementado, #32 |
| Explorador nativo de árvores anotadas e figuras SVG | Implementado, #33 |
| Diagnósticos de amostragem e inspeção do burn-in no app | Implementado, #34 |
| Cadeias paralelas e relatórios de convergência | Implementado, #23 |
| Reescalonamento por potências de dois | Implementado, #18 |
| Checkpoints e retomada de execuções | Implementado na CLI e no Histórico nativo, #22 |
| Partições por número de estados | Implementado, #19 |
| Condicionamento a caracteres informativos para parcimônia | Implementado, #20 |
| Caracteres ordenados | Planejado, #21 |
| Verossimilhanças marginais para comparar modelos | Planejado, #24 |
| Prior de nascimento–morte fossilizado com datas de terminais | Planejado, #25 |
| Checagens preditivas posteriores | Planejado, #26 |
| Avaliação de repetições dentro de subárvores | Avaliado; experimental, desligado por padrão, #17 |
| Adaptadores opcionais BEAST 2 JNI e BEAGLE C ABI | Possível, #6 e #7 |

## Estabelecer o baseline

O [núcleo](./kernel) é verificado contra uma implementação escalar independente. O [leitor de NEXUS](./nexus) fornece observações ponderadas sem o BEAST. O [experimento de referência](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra entradas exatas, configurações de modelo, duas execuções completas do BEAST e estados numéricos fixos.

O [baseline no Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 cenários em Rust, separando preparação, avaliações completas, atualizações parciais e manutenção do estado das propostas. Inclui amostras brutas, observações de alocação e variação entre três processos. Uma [comparação do modelo completo](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md) agora registra tempo e ESS por segundo em seis casos de 20 táxons; cenários maiores continuam pendentes. A comparação sob relógio estrito, mais abaixo, registra o tempo decorrido dos dois motores numa configuração pequena, mas não é um benchmark controlado.

## Corretude antes da otimização

O Mkv condiciona a verossimilhança ao processo de observação em matrizes só com caracteres variáveis. Ele reproduz os dois estados arquivados do BEAST, e seus padrões constantes ficam separados dos pesos dos padrões observados.

O caminho direto do Mk e o backend NEON estão implementados e medidos nos relatórios de [Mk](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) e [NEON](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md). O reescalonamento por potências de dois remove os logaritmos por nó e guarda os totais de expoentes por subárvore. A [comparação](https://github.com/getcortexlabs/rove/blob/main/benchmarks/scaling/2026-10-30/README.md) separa as duas mudanças nos dois backends, inclui execuções do amostrador e registra os casos mais lentos. A implementação portátil continua como referência e fallback.

## Inferência independente

O primeiro [amostrador mínimo](./sampler) suportava Mk ou Mkv binário, um relógio estrito e o prior de nascimento–morte. Com os mesmos movimentos e passos fixos do BEAST 2.7.8, as suas médias posteriores e frequências de clados na matriz de referência atenderam a critérios fixados antes das execuções ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). O [modelo de referência](./reference-model), com categorias gama e relógio relaxado lognormal, agora reproduz os dois estados arquivados componente por componente. Os resumos de posterior e as frequências de clados atenderam aos critérios definidos antes das corridas em seis casos construídos com 20 táxons ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/inference/2026-10-29/README.md)). A validação avança das verossimilhanças em estados fixos até execuções completas em matrizes conhecidas. A qualidade da amostragem, incluindo ESS por tempo e diagnósticos de árvore, conta junto com o tempo de execução.

Com a avaliação das propostas registrada (#27), os recursos planejados seguem o que as análises morfológicas precisam: caracteres ordenados (#21), verossimilhanças marginais para comparar modelos (#24), o prior de nascimento–morte fossilizado com datas de terminais (#25) e checagens preditivas posteriores (#26). O RevBayes documenta cada um deles para dados morfológicos e pode servir de segunda referência onde o BEAST não tiver um recurso equivalente.

A [prévia nativa para Mac](./macos-preview) importa e exibe matrizes e prepara as configurações da análise. Ela executa o motor Rust incluído com progresso real, cancelamento e reabertura de configurações com verificação da impressão digital. Os adaptadores BEAST e BEAGLE são caminhos de compatibilidade opcionais, não dependências da engine em Rust.

Acompanhe as [issues do repositório](https://github.com/getcortexlabs/rove/issues) para critérios de aceitação e resultados.

As partições por número de estados (#19) estão implementadas no motor, CLI, app nativo e checkpoints. As regras e a validação em estados fixos estão no [guia NEXUS](./nexus#particoes-por-numero-de-estados).

O condicionamento a caracteres informativos (#20) está implementado no motor, na CLI, no app nativo e nos checkpoints. Veja [seleção de caracteres](./character-selection) para as regras de entrada e a validação.
