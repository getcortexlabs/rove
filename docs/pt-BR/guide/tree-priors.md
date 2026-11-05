---
description: "Priors de árvore disponíveis, suas hipóteses e os limites de cada escolha."
---

# Escolher o prior da árvore

O prior da árvore descreve o processo de ramificação antes de considerar os
caracteres. É diferente do prior do sigma do relógio (variação entre ramos)
e do prior da forma gamma (variação entre caracteres).

## Opções disponíveis · 5 de novembro de 2026

**Yule está disponível como prior opcional para espécies contemporâneas.** Birth–death continua como padrão, e a datação de fósseis usa FBD. Escolha conforme a amostragem e as hipóteses biológicas do estudo; a implementação não demonstra que Yule descreva melhor as matrizes do Thiago. Veja o [modelo e os controles disponíveis](./reference-model).

| Análise | Escolha | Situação e limite no Rove |
| --- | --- | --- |
| Espécies tratadas como amostradas no mesmo momento | Birth–death | Disponível; pontas em zero, amostragem completa, taxas constantes |
| Mesmo cenário, assumindo ausência de extinção | Yule | Disponível; nascimento puro, sem extinção |
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

## Implementação e validação de Yule

A implementação segue o `YuleModel` do BEAST 2.7.8: amostragem completa,
condicionamento no número de pontas, sem condicionar na raiz/origem, sem pontas
fósseis e sem calibrações de nós. Para n pontas, taxa de nascimento λ e alturas
internas h, a log-densidade sem fator de escala é
`(n−1) log(λ) − λ (sum(h) + altura_da_raiz)`.
É o caso de extinção zero do nosso birth–death;
veja [Gernhard (2008)](https://doi.org/10.1016/j.jtbi.2008.04.005).

A validação inclui 24 estados fixos do BEAST 2.7.8, simulações independentes do prior, o limite de extinção zero do birth–death, quadratura de posteriores de potência e retomada idêntica por checkpoint. O parâmetro e os movimentos de extinção não existem nesse modelo. App e CLI preservam as escolhas na configuração, Revisão/Histórico, predição e comparação de modelos.

O prior da taxa de nascimento é Uniform(0,U), próprio, com U positivo configurável. U=1000 é uma referência na escala de alturas do relógio unitário, não uma recomendação biológica universal. Confira execuções só do prior e limites alternativos defensáveis. Comparações marginais exigem os mesmos dados, restrições e convenções de normalização compatíveis; veja os [detalhes do modelo](./reference-model).

Antes de interpretar, comparar cadeias independentes, execuções só do prior
e a sensibilidade dos clados e das alturas às hipóteses. Rodar mais rápido ou
ter suporte maior não torna um modelo biologicamente melhor.

## ORC é outra decisão

Já temos taxas lognormais explícitas e movimentos conjuntos opcionais, mas não
toda a coleção de operadores do ORC. A
[auditoria do XML](https://github.com/getcortexlabs/rove/blob/main/fixtures/reference/20x50/orc-audit.md)
registra versões, correspondências e testes necessários. O padrão fica como
está enquanto #48 e #72 avaliam os pesos. A troca conjunta de topologia e taxas da [#97](https://github.com/getcortexlabs/rove/issues/97) já existe como experimento opcional para relógio relaxado sem datação, com peso zero por padrão. O benefício comparativo em morfologia ainda não foi medido; a [#128](https://github.com/getcortexlabs/rove/issues/128) registra o suporte parcial ao ORC e os operadores e validações pendentes.
