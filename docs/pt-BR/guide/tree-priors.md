---
description: "Priors de árvore disponíveis e a próxima alternativa escolhida após revisar BEAST e RevBayes."
---

# Escolher o prior da árvore

O prior da árvore descreve o processo de ramificação antes de considerar os
caracteres. É diferente do prior do sigma do relógio (variação entre ramos)
e do prior da forma gamma (variação entre caracteres).

## Decisão · 2 de novembro de 2026

Manter birth–death como padrão e FBD para datação. A próxima alternativa será
**Yule, opcional, para espécies contemporâneas**. Essa é nossa decisão de
desenvolvimento após a revisão; não significa que Yule descreva melhor as
matrizes do Thiago. Yule ainda não está implementado no Rove; a implementação está na [#96](https://github.com/getcortexlabs/rove/issues/96).

| Análise | Escolha | Situação e limite no Rove |
| --- | --- | --- |
| Espécies tratadas como amostradas no mesmo momento | Birth–death | Disponível; pontas em zero, amostragem completa, taxas constantes |
| Mesmo cenário, assumindo ausência de extinção | Yule | Próxima alternativa; modelo mais restrito para comparação |
| Fósseis com idades | Fossilized birth–death (FBD) | Disponível; processo fixo ou estimado, idades incertas; fósseis apenas nas pontas |
| Fósseis que podem ser ancestrais diretos | FBD com ancestrais amostrados | Trabalho separado na #52 |
| Amostras dentro de uma população | Coalescente / skyline demográfico | Adiado; responde a outra pergunta |
| Morfologia sem relógio nem datação | Topologia uniforme com prior de comprimentos de ramo | Alternativa a avaliar separadamente; exige outra parametrização da árvore |
| Diversificação ou recuperação de fósseis variando no tempo | Skyline birth–death/FBD | Adiado até haver uma pergunta definida e informação para estimar mais taxas |

A [orientação do BEAST sobre Yule](https://beast2-dev.github.io/hmc/hmc/YuleModel/index/)
restringe esse modelo a pontas contemporâneas. O
[tutorial de morfologia do RevBayes](https://revbayes.github.io/tutorials/morph_tree/V2)
mostra uma alternativa sem relógio. Seu
[tutorial FBD](https://revbayes.github.io/tutorials/fbd_simple/)
modela diversificação e recuperação de fósseis em conjunto.

Um fóssil sem idade não vira uma espécie atual. Colocar todas as pontas em zero
é uma hipótese, não uma solução para a falta de idades. A amostragem, a extinção
e a escala de tempo precisam combinar com a análise. O tamanho da matriz e um
ESS alto não escolhem o melhor prior.

Nosso birth–death usa a convenção Gernhard sem escala, condicionada no número de pontas, não na raiz/origem. FBD condiciona na origem, sem condicionar na sobrevivência/amostragem atual; a probabilidade de remoção é zero. Processo fixo e estimado têm convenções de densidade distintas registradas. Veja [Datação de fósseis](./tip-dating) antes de comparar modelos.

## O que precisamos validar para adicionar Yule

A primeira versão seguirá o `YuleModel` do BEAST 2.7.8: amostragem completa,
condicionamento no número de pontas, sem condicionar na raiz/origem, sem pontas
fósseis e sem calibrações de nós. Para n pontas, taxa de nascimento λ e alturas
internas h, a log-densidade sem fator de escala é
`(n−1) log(λ) − λ (sum(h) + altura_da_raiz)`.
É o caso de extinção zero do nosso birth–death;
veja [Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005).

A implementação exige comparação de densidades com BEAST, simulações
independentes do prior, retirada do parâmetro e dos movimentos de extinção,
prior explícito para a taxa de nascimento, configurações versionadas,
checkpoints, registros de saída e controles em português e inglês.
Uniform(0,1000) permanece como referência para comparar sob relógio unitário,
com escala documentada e configuração deliberada. Precisamos conferir
constantes e restrições antes de comparar verossimilhanças marginais.

Antes de interpretar, comparar cadeias independentes, execuções só do prior
e a sensibilidade dos clados e das alturas às hipóteses. Rodar mais rápido ou
ter suporte maior não torna um modelo biologicamente melhor.

## ORC é outra decisão

Já temos taxas lognormais explícitas e movimentos conjuntos opcionais, mas não
toda a coleção de operadores do ORC. A
[auditoria do XML](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
registra versões, correspondências e testes necessários. O padrão fica como
está enquanto #48 e #72 avaliam os pesos. O próximo experimento isolado será
[NER #97](https://github.com/getcortexlabs/rove/issues/97): mudar ramos próximos e suas taxas juntos. Ainda precisamos medir se isso
ajuda nas matrizes morfológicas.
