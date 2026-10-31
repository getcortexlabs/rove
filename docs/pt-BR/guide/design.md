---
description: "Como o Rove organiza os dados de verossimilhança, por que o kernel conhece o modelo, para que serve a referência escalar e como funcionam os buffers e o reescalonamento."
---

# Design

O Rove é uma única crate Rust. Ela calcula a verossimilhança de uma árvore binária enraizada sob o modelo Mk para uma matriz de caracteres discretos. As decisões abaixo são as que já estão visíveis no código. O trabalho de SIMD que elas preparam começou com o [backend NEON](./kernel#backends); o resto está no [roadmap](./roadmap).

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

Com `Scaling::Always`, cada nó reescalona por uma potência de dois escolhida a partir do máximo entre os estados e categorias de cada padrão. O buffer guarda um expoente inteiro para toda a subárvore: o expoente local mais os totais dos dois filhos. A raiz recompõe a escala com `E * ln(2)` uma vez por padrão, sem percorrer todos os nós. Os expoentes são restaurados junto com os buffers de parciais. Veja a [convenção exata](./kernel#reescalonamento).

## Padrões constantes no mesmo núcleo

A correção Mkv precisa da probabilidade de cada padrão constante sob a mesma árvore, as mesmas taxas e o mesmo escalonamento que as observações. O Rove acrescenta esses padrões às entradas dos terminais do núcleo em vez de calculá-los separadamente, de modo que atualizações parciais, reescalonamento e store/restore os cobrem sem mudanças. Na referência 20 × 50, os dois padrões extras aumentam o stride com padding de 24 para 32, e cada atualização de nó passa a cobrir um terço a mais de lanes.

## Um amostrador sobre os mesmos buffers

O [amostrador](./sampler) mantém a árvore e os seus parâmetros fora do núcleo. Antes de cada passo, ele os guarda e registra os índices dos buffers do núcleo. Depois de uma proposta, compara a nova árvore com a guardada, dá a cada ramo alterado uma nova distância no seu outro buffer e recalcula só os ancestrais desses ramos. Um passo rejeitado troca os índices de volta. As propostas reproduzem os operadores e as razões de Hastings do BEAST 2, então uma comparação com o BEAST pode usar os mesmos movimentos e pesos.

A configuração de referência acrescenta forma gama, sigma do relógio e taxas explícitas de ramos ao estado guardado. Mudar apenas sigma não altera a verossimilhança; o movimento conjunto de sigma e taxas atualiza as distâncias dos ramos; mudar a forma atualiza as distâncias por categoria em toda a árvore. Suas propostas diferem das do ORC, como registra o [guia do modelo de referência](./reference-model#propostas-e-saida).

## Onde o amostrador gasta tempo

O [perfil com 90 e 500 terminais](https://github.com/getcortexlabs/rove/blob/main/benchmarks/profile/2026-10-31/README.md) mede o amostrador atual num Apple M4 Max. Uma compilação opcional para benchmarks amostra as fases ao longo do tempo decorrido; não é um perfil nativo das pilhas da CPU. Oito pares de execuções reproduziram os valores do traço, as árvores e as estatísticas dos operadores da compilação normal. O relatório preserva as razões de tempo da instrumentação, a variação do ambiente e as duas repetições. A compilação padrão não contém código de perfilamento.

No preset de referência, as parciais dos nós e o reescalonamento somam cerca de 68% das observações com 90 terminais e 82% com 500. Só o reescalonamento responde por cerca de 22–23%, mesmo depois da troca dos logaritmos por expoentes binários: ainda é preciso escalar as parciais e acumular os expoentes dos filhos. Nessas frações instrumentadas, zerar o custo do núcleo limitaria o ganho na janela observada a cerca de 3,14× e 5,50×, respectivamente, mantendo a redução na raiz, os percursos, as propostas e a saída. São limites condicionais, não promessas de aceleração em produção. O [estudo com cadeias independentes](https://github.com/getcortexlabs/rove/blob/main/benchmarks/convergence/2026-10-31/README.md) avalia a qualidade da amostragem separadamente.

## NEON primeiro, AVX2 depois

O layout existe para que os laços internos possam ser vetorizados ao longo dos padrões. Nos builds de release, o compilador já vetoriza os laços portáteis, um de cada vez: uma atualização de nó zera dois acumuladores, soma neles a contribuição de cada estado, combina-os com as parciais dos filhos, e o reescalonamento relê os resultados para achar o máximo de cada padrão. O [backend NEON](./kernel#backends) faz, em vez disso, a atualização de nó do Mk numa única passada por blocos de oito padrões, com as somas e os máximos em registradores. Para cada padrão ele faz as mesmas operações na mesma ordem, então reproduz bit a bit os resultados do código portátil, exceto o payload dos NaN, e os testes exigem isso. Num Apple M4 Max, o código portátil levou 1,48 vez o tempo do NEON numa avaliação completa da matriz de referência ([relatório](https://github.com/getcortexlabs/rove/blob/main/benchmarks/neon/2026-10-28/README.md)). O reescalonamento agora extrai expoentes binários com operações inteiras; o NEON trata dois padrões por registrador. Não há caminho AVX2.

## Uma única crate

O Rove não depende do BEAST nem do BEAGLE. O BEAST é uma referência de validação e benchmarking. Uma C ABI do BEAGLE e um adaptador `LikelihoodCore` do BEAST 2 via JNI são caminhos de compatibilidade possíveis. Nenhum dos dois é exigido pelo núcleo em Rust, e nenhum está sendo construído agora.

## O que não é objetivo

Competir com o backend CUDA do BEAGLE em alinhamentos grandes de nucleotídeos. Para isso, use a GPU.
