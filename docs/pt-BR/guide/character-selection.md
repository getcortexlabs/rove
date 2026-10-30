---
description: "Escolha Mk, Mkv ou Mkparsinf conforme a coleta dos caracteres morfológicos."
---

# Seleção de caracteres

A correção precisa corresponder aos caracteres que podiam entrar na matriz. Não é um ajuste de velocidade ou de qualidade.

| Caracteres mantidos na coleta | Opção na CLI | Aplicativo Mac |
| --- | --- | --- |
| Constantes e variáveis | `--conditioning none` | Todos os caracteres · Mk |
| Apenas variáveis | `--conditioning variable` (padrão) | Só variáveis · Mkv |
| Apenas informativos para parcimônia | `--conditioning informative` | Só informativos · Mkparsinf |

Um caráter completo é informativo para parcimônia quando pelo menos dois estados diferentes aparecem, cada um, em dois ou mais táxons. `0011` atende; `0001` e `00123` não. Com vários estados, excluir só os constantes e os casos com uma única ponta diferente não basta.

Mkparsinf funciona com os dois relógios, partições por número de estados, cadeias independentes e checkpoints. Cada partição usa Mk não ordenado, de frequências iguais. O Rove divide a verossimilhança observada pela probabilidade de um caráter ser informativo na mesma árvore e com as mesmas taxas de ramo, depois de combinar as categorias de taxa. Guarda probabilidades por subárvore em vez de enumerar colunas excluídas. Essa recorrência não se aplica a modelos ordenados ou de frequências desiguais.

Células faltantes e incertas são aceitas somente se toda forma permitida de preenchê-las for informativa: `0011?` atende, `001??` não. O condicionamento considera o caráter completo em todos os táxons, não um evento separado para cada máscara de dados faltantes. `rove inspect` informa `informative_error`; o app bloqueia análises incompatíveis e a CLI identifica o número original do caráter. O Rove nunca remove caracteres automaticamente. Árvores muito curtas cuja probabilidade de retenção não pode ser distinguida de zero em precisão dupla são rejeitadas.

No app, abra **Modelo → Quais caracteres você incluiu?** e escolha **Só informativos · Mkparsinf** apenas se corresponder à coleta. O exemplo 20 × 50 incluído contém caracteres não informativos; mantenha Mkv nesse exemplo.

Para uma checagem rápida de funcionamento:

```sh
target/release/rove sample fixtures/validation/informative/matrix.nex \
  --conditioning informative --model reference \
  --output runs/informative --seed 42 --steps 10000 --sample-every 100
```

Essa execução curta verifica o fluxo, não a convergência. A [validação](https://github.com/getcortexlabs/rove/tree/main/fixtures/validation/informative) inclui enumeração completa de padrões, dados faltantes, restauração de checkpoints e concordância em estados fixos com BEAST 2.7.8. Não demonstra concordância posterior nem ganho de velocidade na inferência.
