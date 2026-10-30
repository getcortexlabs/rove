---
description: "Declare caracteres morfológicos ordenados, preserve estados intermediários e combine partições ordenadas e não ordenadas na mesma análise."
---

# Caracteres ordenados

Um caráter ordenado muda passando por estados vizinhos: `0 ↔ 1 ↔ 2`. Um ramo ainda pode começar em `0` e terminar em `2`, passando por `1`. A ordenação restringe cada mudança individual, não a mudança total ao longo do ramo. O Mk não ordenado permite uma mudança direta entre qualquer par de estados.

O Rove lê essa escolha do NEXUS. Adicione um bloco `ASSUMPTIONS` **depois** do bloco `DATA` ou `CHARACTERS`:

```nexus
BEGIN ASSUMPTIONS;
  OPTIONS DEFTYPE=UNORD;
  TYPESET * character_order = ORD: 2-3 5 8;
END;
```

Aqui os caracteres 2, 3, 5 e 8 são ordenados; os demais não são. A contagem começa em um. O aplicativo mostra **Ordenados** ou **Não ordenados** em cada partição na matriz e na revisão da análise. Ainda não há um editor separado de ordenação.

## Sintaxe e alfabetos de estados

- `DEFTYPE` aceita `ORD` ou `UNORD`; o padrão é `UNORD`. Vale para os caracteres omitidos do conjunto de tipos.
- É permitido um `TYPESET` com nome e `*` opcional. O único conjunto é aplicado com ou sem a estrela. A forma padrão aceita índices individuais, intervalos, `.` para o último caráter, `ALL` e passos como `1-7\2`.
- `(STANDARD)` é opcional. `(VECTOR)` exige um token `ORD` ou `UNORD` por caráter, por exemplo `TYPESET order (VECTOR) = ORD UNORD ORD;` para três caracteres.
- Múltiplos conjuntos de tipos, sobreposições, conjuntos de caracteres nomeados, outros tipos de transformação, pesos e exclusões são rejeitados. O que não é suportado nunca é descartado silenciosamente. Veja [Lendo NEXUS](./nexus) para o restante do formato.

A ordem declarada em `SYMBOLS` define a vizinhança; os rótulos descritivos não. Com `--partitioning observed`, caracteres ordenados mantêm o alfabeto desde o primeiro símbolo até o maior estado observado ou explicitamente possível. Com `SYMBOLS="0123"`, observações `0`, `2`, `3` exigem **quatro** estados ordenados, incluindo o intermediário `1`, mesmo sem observação. As mesmas observações não ordenadas formam um modelo de três estados.

Dados ausentes não definem estados possíveis. Um alfabeto binário declarado sempre mantém ambos os estados; nos demais casos, um alfabeto inferido com menos de dois estados é rejeitado. Se estados superiores não observados forem biologicamente possíveis, use `--partitioning declared` para manter o alfabeto inteiro. A API Rust `compress_sets` também aceita alfabetos explícitos; os ordenados devem ser contíguos e seguir a ordem declarada dos símbolos.

As partições agrupam por número de estados e ordenação. A compressão nunca mistura caracteres ordenados e não ordenados, mesmo com observações iguais. Eles compartilham árvore, relógio e forma gama. `rove inspect` e `run.json` registram a ordenação, os índices originais e o mapeamento de estados de cada partição.

## Modelo e limites

Para `k` estados, as taxas instantâneas entre vizinhos são `k / (2(k − 1))`; as demais taxas fora da diagonal são zero, e cada diagonal é o negativo da soma das taxas de saída. As frequências estacionárias são uniformes. Isso normaliza a quantidade esperada de mudanças por caráter para uma por unidade de distância do ramo; com dois estados, o modelo equivale ao Mk binário uniforme.

O motor Rust calcula matrizes de transição por uniformização positiva com escalonamento e quadraturas, reutilizando os buffers. Partições ordenadas usam o cálculo geral com matrizes; ainda não há um kernel NEON dedicado a elas. O caminho direto do Mk continua disponível para partições não ordenadas.

Caracteres ordenados aceitam a verossimilhança sem seleção de caracteres e o condicionamento Mkv para caracteres variáveis, com relógio estrito ou relaxado e categorias gama. **Mkparsinf, para caracteres apenas informativos, não é suportado nos ordenados** e é rejeitado antes da execução. Escolha o condicionamento conforme a coleta da matriz; trocá-lo para contornar um erro muda o modelo estatístico. Veja [seleção de caracteres](./character-selection).

Checkpoints preservam as matrizes ordenadas e distâncias em cache. A retomada continua exigindo o mesmo motor, entrada e configurações; esta mudança não migra checkpoints de versões anteriores.

## Validação

A [matriz de validação](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/ordered) registra 36 exponenciais de matriz independentes calculadas com 160 dígitos, uma conferência analítica de três estados e 12 comparações em estados fixos com o modelo `Ordinal` do BEAST 2.7.8 / MM 1.2.1. A maior diferença absoluta no log da verossimilhança foi `1,82e-9`, entre Mk/Mkv, ambos os relógios e três escalas de árvore. Os testes também verificam matrizes estocásticas, estados intermediários, rejeição de propostas, retomada de checkpoints e alocações após a preparação.

Isso demonstra concordância numérica nos estados testados. Não demonstra convergência posterior nem ganho de velocidade para análises ordenadas. O BEAST só gera a referência de validação; executar o Rove não exige instalá-lo.
