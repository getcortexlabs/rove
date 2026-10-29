---
description: "Compile e explore a primeira interface nativa do Rove: importe uma matriz, examine os caracteres e prepare uma análise."
---

# A prévia para Mac

O Rove tem uma primeira interface nativa para macOS, feita com SwiftUI e
AppKit. Ela importa matrizes pelo motor Rust e prepara as configurações da
análise. A execução e o acompanhamento do progresso dentro da janela ainda
não estão implementados.

![A interface nativa do Rove](/images/macos-preview.png)

## Compilar e abrir

No macOS 14 ou mais recente, com Xcode compatível com Swift 6 e Rust
instalados, execute na raiz do repositório:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

Isso gera um aplicativo local com assinatura ad hoc. Não é uma versão
notarizada nem um instalador. A interface está em inglês e usa um tema escuro
inspirado na Cortex Labs, respeitando a preferência de reduzir movimento.

## Explorar uma matriz

Abra um NEXUS com **Command-O**, arraste um arquivo para a tela inicial ou
escolha **Try an example**. O exemplo é a matriz de referência fornecida,
com 20 terminais e 50 caracteres.

A tela mostra táxons, caracteres observados, padrões comprimidos e células
ausentes. Clique numa célula para ver o táxon, o caráter e a observação.
As estatísticas cobrem a matriz inteira; a prévia com rolagem mostra até
200 táxons e 120 caracteres. O aplicativo aceita arquivos de até 32 MB.

Conseguir importar não significa conseguir analisar. Uma matriz multiestado
pode ser visualizada, mas o amostrador atual aceita apenas caracteres binários.
Entradas incompatíveis recebem uma explicação. Gaps exigem uma escolha
explícita para serem tratados como ausentes; caracteres potencialmente
constantes não podem usar Mkv.

## Preparar uma análise

Escolha o relógio estrito ou a [configuração de referência com relógio relaxado](./reference-model),
Mk ou Mkv, tamanho da cadeia, intervalo de amostragem e semente. Revise as
escolhas antes de salvar. Os priors mantêm os padrões do motor, e nenhum grupo
recebe uma restrição automaticamente. O tamanho da cadeia sozinho não
estabelece convergência.

**Save setup** grava um `.rove.json` com o caminho da matriz original,
SHA-256 e configurações. Não copia a matriz nem inicia uma corrida. Reabrir
esses registros no aplicativo ainda não está implementado.

**Copy terminal command** prepara um comando para o motor Rust incluído.
Colá-lo no Terminal inicia uma análise real usando a matriz original no
momento da execução. O comando usa uma pasta nova dentro de `~/Rove Runs`.
O [guia de amostragem](./sampler) explica os arquivos gerados e os diagnósticos.

## Inspecionar pelo terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

O comando emite JSON versionado com contagens, uma prévia limitada das células,
o hash da entrada e erros separados de compatibilidade com o amostrador e Mkv.
Adicione `--gaps missing` somente quando essa interpretação for adequada.
Ler uma matriz não grava resultados de análise.
