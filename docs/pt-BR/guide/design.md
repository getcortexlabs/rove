---
description: "Como o Rove organiza os dados de verossimilhança, por que o kernel conhece o modelo, para que serve a referência escalar e como funcionam os buffers e o reescalonamento."
---

# Design

O Rove é uma única crate Rust. Ela calcula a verossimilhança de uma árvore binária enraizada sob o modelo Mk para uma matriz de caracteres discretos. As decisões abaixo são as que já estão visíveis no código. O trabalho de SIMD e de caminhos rápidos que elas preparam está no [roadmap](./roadmap).

## Layout pattern-major

Um kernel de verossimilhança passa o tempo multiplicando matrizes de transição por verossimilhanças parciais. Os kernels de CPU do BEAGLE vetorizam ao longo dos estados e são especializados em quatro estados, o que serve bem a nucleotídeos. Matrizes morfológicas são, em sua maioria, caracteres binários e ternários, então esses kernels seguem o caminho genérico, com padding.

O Rove armazena as parciais no sentido inverso. Para cada categoria de taxa e cada estado há uma sequência contígua de valores, um por padrão de sítio, com padding até um múltiplo de oito lanes. O laço interno percorre os padrões, então tem a mesma forma para dois estados e para sessenta e um.

`Layout` descreve esse arranjo: o número de estados, de categorias de taxa e de padrões, e o stride com padding. `offset(category, state)` dá o início de uma sequência.

## Caminhos rápidos que conhecem o modelo

O Mk tem forma fechada. A matriz de transição de um ramo é `P(t) = e·I + (1 − e)·π`, onde `e` depende apenas do comprimento de ramo e `π` contém as frequências de estados. Multiplicá-la por um vetor custa O(k) por padrão em vez de O(k²).

Hoje `model::Mk` produz a matriz completa e o kernel a multiplica de forma genérica. A forma fechada é testada contra a fórmula de Lewis. Medições de baseline e a correção de ascertainment Mkv precedem o caminho direto do Mk no [roadmap](./roadmap).

## Uma referência escalar, sempre

`reference::Problem` é uma implementação recursiva simples do algoritmo de poda de Felsenstein. É lenta e simples de propósito. Os testes de igualdade constroem árvores aleatórias e comparam `LikelihoodCore` com ela, com estados nos terminais e com parciais nos terminais, com e sem reescalonamento, a uma tolerância relativa de 1e-9.

Todo caminho otimizado adicionado depois tem de passar nos mesmos testes. A referência permanece.

## Buffers duplos e reescalonamento

Cada nó possui dois buffers para parciais e dois para matrizes de transição. Antes que uma proposta sobrescreva um nó, `set_node_partials_for_update` ou `set_node_matrix_for_update` alterna esse nó para o seu outro buffer, de modo que os valores anteriores sobrevivem. `store` registra quais buffers estão em uso. `restore` traz esse conjunto de volta trocando os buffers, `unstore` copiando. Aceitar um movimento não custa nada. É disso que um amostrador MCMC precisa para rejeitar um movimento a baixo custo.

Com `Scaling::Always`, as parciais de cada nó interno são divididas pelo seu máximo por padrão depois de calculadas, e o logaritmo desse máximo é guardado por nó. A log-verossimilhança soma os fatores de escala de volta. Sem escalonamento, árvores profundas sofrem underflow.

## NEON primeiro, AVX2 depois

O layout existe para que o laço interno possa ser vetorizado. O plano é NEON em Apple silicon primeiro e AVX2 depois, com o código escalar mantido como referência e fallback. Nenhum dos dois existe ainda; o kernel atual é Rust escalar.

## Uma única crate

O Rove não depende do BEAST nem do BEAGLE. O BEAST é uma referência de validação e benchmarking. Uma C ABI do BEAGLE e um adaptador `LikelihoodCore` do BEAST 2 via JNI são caminhos de compatibilidade possíveis. Nenhum dos dois é exigido pelo núcleo em Rust, e nenhum está sendo construído agora.

## O que não é objetivo

Competir com o backend CUDA do BEAGLE em alinhamentos grandes de nucleotídeos. Para isso, use a GPU.
