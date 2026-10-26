---
description: "A matriz de controle sintética que acompanha o Rove, o script que a regenera e degrada, e como executá-la no BEAST 2."
---

# Matrizes sintéticas

`fixtures/synthetic/control_20x50.nex` é um controle positivo: 20 táxons, 50 caracteres binários, sem homoplasia, sem dados faltantes, árvore conhecida. T20 é o grupo externo, só zeros. A árvore do grupo interno é balanceada, e cada um de seus 18 clados recebe sinapomorfias exclusivas: três para os nove maiores clados e duas para os demais. Os últimos cinco caracteres são autapomorfias. Essa construção não garante suporte posterior: enraizamento, modelos de relógio e priors também afetam o resultado.

`control_20x50.tre` é a árvore verdadeira em Newick, repetida também como comentário no final do arquivo NEXUS. `control_20x50.legend.txt` associa cada caractere ao clado que ele sustenta.

## Regenerando e degradando a matriz

`tools/synth_matrix.py` regenera o controle e produz variantes degradadas:

```sh
python3 tools/synth_matrix.py --out fixtures/synthetic/control_20x50
python3 tools/synth_matrix.py --homoplasy 10 --seed 7 --out /tmp/homoplasy10
python3 tools/synth_matrix.py --missing 0.2 --out /tmp/missing20
python3 tools/synth_matrix.py --drop T1-T4 --out /tmp/no_t1_t4
```

As degradações são listadas no final do arquivo de legenda.

## Executando no BEAST 2

No BEAUti, instale o pacote MorphModels, importe o arquivo NEXUS, escolha o modelo Mk de Lewis com a correção de ascertainment para caracteres variáveis, um relógio estrito e um prior de árvore Yule.

Esta é uma configuração de exemplo para o controle gerado. Ela difere da análise de referência selecionada, descrita abaixo.

## Matriz de referência fornecida

`fixtures/reference/20x50/matrix.nex` é um conjunto de dados separado, com sua legenda original e o XML do BEAST. Sua topologia e a atribuição dos caracteres diferem do controle gerado. O experimento registrado com o BEAST 2.7.8 usa relógio relaxado, quatro categorias gama, prior de nascimento–morte e uma restrição de monofilia do grupo interno. Entradas, procedência e resultados estão no [relatório de referência](https://github.com/getcortexlabs/rove/blob/main/benchmarks/beast-2.7.8/2026-10-25/README.md).

O [leitor de NEXUS](./nexus) importa as duas fixtures de forma independente. Cada uma tem 23 padrões observados representando 50 caracteres, mas os padrões diferem. Nenhuma das matrizes substitui a outra na validação.
