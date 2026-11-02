# A pasta da sua execução

Abra **O que são estes arquivos?** ao lado da pasta nos resultados ou no Histórico para consultar o guia sem internet. Uma execução com uma cadeia costuma ter esta estrutura:

```text
analysis-<id>/
  setup.rove.json
  session.json
  events.jsonl
  engine.stderr.log
  results/
    run.json
    trace.log
    trees.nexus
    checkpoint.json
    checkpoint-<step>.bin
```

| Arquivo | Para que serve |
| --- | --- |
| `setup.rove.json` | Configuração pedida, caminho e impressão digital da matriz. Reabra para preparar outra execução. |
| `session.json` | Sessão do aplicativo, estado e detalhes da execução. |
| `events.jsonl` | Eventos de progresso, um objeto JSON por linha. |
| `engine.stderr.log` | Mensagens do motor para investigar problemas; não é o traço numérico. |
| `dating.json` | Dados da datação de fósseis, quando ativada. |
| `results/run.json` | Configuração do motor, identificação da compilação e da entrada, e registro de conclusão. |
| `results/trace.log` | Valores numéricos amostrados. Abra nos Diagnósticos do Rove ou no Tracer. |
| `results/trees.nexus` | Árvores amostradas brutas, incluindo o estado inicial. |
| `results/checkpoint.json` e `checkpoint-<step>.bin` | Índice de retomada e estado binário. Guarde o binário indicado e as saídas associadas. |

Com várias cadeias, cada uma tem seus arquivos em `results/chain-01/`, `chain-02/` etc. O registro do grupo fica em `results/run.json`.

## Das amostras à árvore

Experimente o exemplo público de 20 × 50 do aplicativo. Depois da execução, confira os **Diagnósticos** e compare execuções independentes. Escolha **Resumir árvores**, decida quanto do início descartar (burn-in) e abra `summaries/summary-<id>/mcc.tree` no Rove ou no FigTree. Essa é uma árvore representativa anotada. `samples.trees` contém amostras brutas; `summary.json` registra as escolhas e o cálculo de suporte. Resumir árvores ou terminar os passos pedidos não prova convergência.

`.nexus` e `.trees` podem nomear uma amostra de árvores em NEXUS. Trocar a extensão não converte o arquivo nem calcula suporte dos nós.

## Relatórios criados depois

As pastas opcionais `diagnostics/`, `summaries/`, `comparisons/`, `predictions/` e `marginal/` aparecem após usar as ferramentas correspondentes. Os diagnósticos incluem `diagnostics.json`. Uma [checagem preditiva](./posterior-prediction) inclui `predictive.json`, `replicates.jsonl`, `characters.tsv` e matrizes simuladas em `matrices/`. Guarde os relatórios com a execução de origem. Amostras de árvores importadas usam `tree-source.json` para registrar caminho e impressão digital do arquivo original.

## O que guardar ou compartilhar

Para retomar, guarde a pasta inteira, a matriz original sem alterações e a mesma compilação do Rove. O aplicativo **não** copia a matriz nem o motor para a pasta. Um JSON de configuração ou relatório não substitui o checkpoint binário. Se a análise usou um subconjunto, guarde a matriz exata registrada na configuração.

Para uma figura, compartilhe `mcc.tree` e as opções do resumo. Para avaliar amostragem, inclua traço e árvores brutas. Para reproduzir a análise, inclua o contexto completo e as opções do modelo. Confira caminhos e dados sensíveis antes de compartilhar. Execuções parciais ou interrompidas podem ter saídas incompletas; confira o estado antes de interpretar.
