---
description: "Como o Rove organiza os dados de verossimilhança, por que o kernel conhece o modelo, para que serve a referência escalar e como funcionam os buffers e o reescalonamento."
---

# Design

O Rove é uma única crate Rust. Ela calcula a verossimilhança de uma árvore binária enraizada sob o modelo Mk para uma matriz de caracteres discretos. As decisões abaixo são as que já estão visíveis no código. O trabalho de SIMD que elas preparam está no [roadmap](./roadmap).

## Layout pattern-major

Um kernel de verossimilhança passa o tempo multiplicando matrizes de transição por verossimilhanças parciais. Os kernels de CPU do BEAGLE vetorizam ao longo dos estados e são especializados em quatro estados, o que serve bem a nucleotídeos. Matrizes morfológicas são, em sua maioria, caracteres binários e ternários, então esses kernels seguem o caminho genérico, com padding.

O Rove armazena as parciais no sentido inverso. Para cada categoria de taxa e cada estado há uma sequência contígua de valores, um por padrão de sítio, com padding até um múltiplo de oito lanes. O laço interno percorre os padrões, então tem a mesma forma para dois estados e para sessenta e um.

`Layout` descreve esse arranjo: o número de estados, de categorias de taxa e de padrões, e o stride com padding. `offset(category, state)` dá o início de uma sequência.

## Caminhos rápidos que conhecem o modelo

O Mk tem forma fechada. A matriz de transição de um ramo é `P(t) = e·I + (1 − e)·π`, onde `e` depende apenas do comprimento de ramo e `π` contém as frequências de estados. Multiplicá-la por um vetor custa O(k) por padrão em vez de O(k²).

Um núcleo Mk, construído com `LikelihoodCore::mk`, usa essa forma diretamente. Cada ramo guarda uma distância, `e` e `1 − e` por categoria de taxa em vez de uma matriz, e a atualização de um filho calcula, para cada padrão, uma única soma ponderada pelas frequências, que serve a todos os estados. O núcleo de matrizes continua disponível para modelos sem essa forma, e os testes comparam os dois com a referência.

Num Apple M4 Max, a [comparação do Mk direto](https://github.com/getcortexlabs/rove/blob/main/benchmarks/rove/2026-10-28/README.md) mediu os dois núcleos nos mesmos processos. Numa avaliação completa da referência 20 × 50, o núcleo de matrizes levou 1,25 vez o tempo do núcleo Mk; numa matriz binária de 90 × 250, 1,11 vez; com oito estados, 2,66 vezes. Com dois estados, os dois caminhos fazem quase as mesmas contas, e o caminho Mk percorre a memória menos vezes.

## Uma referência escalar, sempre

`reference::Problem` é uma implementação recursiva simples do algoritmo de poda de Felsenstein. É lenta e simples de propósito. Os testes de igualdade constroem árvores aleatórias e comparam `LikelihoodCore` com ela, com estados nos terminais e com parciais nos terminais, com e sem reescalonamento, a uma tolerância relativa de 1e-9. `variable_log_likelihood` aplica a correção Mkv da mesma forma direta.

Todo caminho otimizado adicionado depois tem de passar nos mesmos testes. A referência permanece.

## Buffers duplos e reescalonamento

Cada nó possui dois buffers para parciais e dois para o seu ramo: matrizes de transição ou distâncias do Mk. Antes que uma proposta sobrescreva um nó, `set_node_partials_for_update` ou `set_node_matrix_for_update` alterna esse nó para o seu outro buffer, de modo que os valores anteriores sobrevivem. `store` registra quais buffers estão em uso. `restore` traz esse conjunto de volta trocando os buffers, `unstore` copiando. Aceitar um movimento não custa nada. É disso que um amostrador MCMC precisa para rejeitar um movimento a baixo custo.

Com `Scaling::Always`, as parciais de cada nó interno são divididas pelo seu máximo por padrão depois de calculadas, e o logaritmo desse máximo é guardado por nó. A log-verossimilhança soma os fatores de escala de volta. Sem escalonamento, árvores profundas sofrem underflow.

## Padrões constantes no mesmo núcleo

A correção Mkv precisa da probabilidade de cada padrão constante sob a mesma árvore, as mesmas taxas e o mesmo escalonamento que as observações. O Rove acrescenta esses padrões às entradas dos terminais do núcleo em vez de calculá-los separadamente, de modo que atualizações parciais, reescalonamento e store/restore os cobrem sem mudanças. Na referência 20 × 50, os dois padrões extras aumentam o stride com padding de 24 para 32, e cada atualização de nó passa a cobrir um terço a mais de lanes.

## Um amostrador sobre os mesmos buffers

O [amostrador](./sampler) mantém a árvore e os seus parâmetros fora do núcleo. Antes de cada passo, ele os guarda e registra os índices dos buffers do núcleo. Depois de uma proposta, compara a nova árvore com a guardada, dá a cada ramo alterado uma nova distância no seu outro buffer e recalcula só os ancestrais desses ramos. Um passo rejeitado troca os índices de volta. As propostas reproduzem os operadores e as razões de Hastings do BEAST 2, então uma comparação com o BEAST pode usar os mesmos movimentos e pesos.

## NEON primeiro, AVX2 depois

O layout existe para que o laço interno possa ser vetorizado. O plano é NEON em Apple silicon primeiro e AVX2 depois, com o código escalar mantido como referência e fallback. Nenhum dos dois existe ainda; o kernel atual é Rust escalar.

## Uma única crate

O Rove não depende do BEAST nem do BEAGLE. O BEAST é uma referência de validação e benchmarking. Uma C ABI do BEAGLE e um adaptador `LikelihoodCore` do BEAST 2 via JNI são caminhos de compatibilidade possíveis. Nenhum dos dois é exigido pelo núcleo em Rust, e nenhum está sendo construído agora.

## O que não é objetivo

Competir com o backend CUDA do BEAGLE em alinhamentos grandes de nucleotídeos. Para isso, use a GPU.
