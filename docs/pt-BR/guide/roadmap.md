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
| Benchmarks de inferência completa e entre motores | Pendente, #8 |
| Correção de ascertainment Mkv | Concluído |
| Poda direta do Mk | Concluído |
| MCMC independente mínimo | Concluído |
| Vetorização NEON | Próximo, #5 |
| Modelo de referência com relógio relaxado | Planejado, #16 |
| Validação completa da inferência | Planejado, #12 |
| Avaliação de repetições dentro de subárvores | Planejado, #17 |
| Adaptadores opcionais BEAST 2 JNI e BEAGLE C ABI | Possível, #6 e #7 |

## Estabelecer o baseline

O [núcleo](./kernel) é verificado contra uma implementação escalar independente. O [leitor de NEXUS](./nexus) fornece observações ponderadas sem o BEAST. O [experimento de referência](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md) registra entradas exatas, configurações de modelo, duas execuções completas do BEAST e estados numéricos fixos.

O [baseline no Apple M4 Max](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-26/README.md) registra 11 cenários em Rust, separando preparação, avaliações completas, atualizações parciais e manutenção do estado das propostas. Inclui amostras brutas, observações de alocação e variação entre três processos. Os benchmarks de inferência completa e entre motores continuam pendentes. A comparação sob relógio estrito, mais abaixo, registra o tempo decorrido dos dois motores numa configuração pequena, mas não é um benchmark controlado.

## Corretude antes da otimização

O Mkv condiciona a verossimilhança ao processo de observação em matrizes só com caracteres variáveis. Ele reproduz os dois estados arquivados do BEAST, e seus padrões constantes ficam separados dos pesos dos padrões observados.

O caminho direto do Mk substitui o produto genérico de matrizes nos modelos Mk. Num Apple M4 Max, o caminho de matrizes levou 1,25 vez o tempo do caminho Mk numa avaliação completa da matriz de referência, e 2,66 vezes com oito estados ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md)). O NEON em Apple silicon vem a seguir, para que perfis de execuções completas do amostrador mostrem onde o tempo do kernel pesa. A implementação escalar continua como referência e fallback. Cada otimização precisa de uma medição antes/depois, não só de uma contagem de operações aritméticas.

## Inferência independente

O [amostrador mínimo](./sampler) suporta Mk ou Mkv binário, um relógio estrito e o prior de nascimento–morte. Com os mesmos movimentos e passos fixos do BEAST 2.7.8, as suas médias posteriores e frequências de clados na matriz de referência atenderam a critérios fixados antes das execuções ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/strict-clock/2026-10-28/README.md)). A configuração com relógio relaxado da análise de referência vem depois do NEON. A validação avança das verossimilhanças em estados fixos até execuções completas em matrizes conhecidas. A qualidade da amostragem, incluindo ESS por tempo e diagnósticos de árvore, conta junto com o tempo de execução.

O produto pretendido é um aplicativo nativo para macOS. O trabalho na interface do aplicativo não começou. Os adaptadores BEAST e BEAGLE são caminhos de compatibilidade opcionais, não dependências da engine em Rust.

Acompanhe as [issues do repositório](https://github.com/getcortexlabs/rove/issues) para critérios de aceitação e resultados.
