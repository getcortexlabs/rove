---
description: "Compile e explore a primeira interface nativa do Rove: importe uma matriz, examine os caracteres e prepare uma análise."
---

# A prévia para Mac

O Rove tem uma primeira interface nativa para macOS, feita com SwiftUI e
AppKit. Ela importa matrizes, prepara a configuração e executa o motor Rust
incluído com progresso real e cancelamento. Não precisa de BEAST nem Java.

![A interface nativa do Rove](/images/macos-preview.png)

## Compilar e abrir

No macOS 14 ou mais recente, com Xcode compatível com Swift 6 e Rust
instalados, execute na raiz do repositório:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

Isso gera um aplicativo local com assinatura ad hoc. Não é uma versão
notarizada nem um instalador. A interface usa um tema escuro inspirado na
Cortex Labs e está disponível em português brasileiro e inglês.

## Idioma e configurações

Abra **Configurações** pela barra lateral ou use **Command-vírgula**. Você pode
seguir o idioma do Mac ou escolher português brasileiro ou inglês. A mudança
é imediata e preserva a matriz aberta e a configuração da análise. As preferências
ficam salvas neste Mac. Também é possível reduzir animações e a transparência
da barra lateral; as preferências de acessibilidade do sistema sempre têm
prioridade. Os diálogos do sistema seguem o idioma do Mac, e as mensagens
originais do motor ficam disponíveis em **Detalhes técnicos**.

## Explorar uma matriz

Abra um NEXUS com **Command-O**, arraste um arquivo para a tela inicial ou
escolha **Experimentar**. O exemplo é a matriz de referência fornecida,
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

## Preparar e executar uma análise

Escolha o relógio estrito ou a [configuração de referência com relógio relaxado](./reference-model),
Mk ou Mkv, tamanho da cadeia, intervalo de amostragem e semente. Os priors
mantêm os padrões do motor. Em Modelo, **Adicionar grupo** seleciona os terminais
que devem formar um clado. Os grupos precisam estar contidos um no outro ou ser
separados. Nada é imposto automaticamente, nem o grupo interno do exemplo.
Suporte imposto por uma restrição não é recuperação independente pelos dados.

Revise a configuração e os grupos, escolha onde salvar e pressione **Iniciar
análise** ou **Command-Return**. O motor Rust incluído executa localmente enquanto
a janela mostra os passos realmente concluídos e o tempo decorrido. As
preferências continuam disponíveis; a configuração científica fica bloqueada
durante a execução. Não há previsão de término, ESS ou suporte inventados.

Cada execução recebe uma pasta nova, por padrão em `~/Rove Runs`. **Abrir pasta**
mostra a configuração salva, o registro da sessão, os eventos de progresso e o
log do motor. A subpasta `results` contém `trace.log`, `trees.nexus` e `run.json`.
Após o término, os botões Traço, Árvores e Registro da execução mostram esses
arquivos no Finder. **Concluída** significa que os passos solicitados terminaram
e os metadados da saída foram conferidos. Não significa convergência. Compare
execuções independentes e avalie os diagnósticos de amostragem e árvores antes
de interpretar os resultados; veja o [guia de amostragem](./sampler).

**Cancelar análise** pede confirmação antes de parar. Os arquivos parciais
continuam identificados; não são uma análise concluída nem um ponto de retomada.
Fechar a janela principal ou sair durante uma análise pede confirmação e espera
o motor encerrar. Se for necessário forçar a parada, os arquivos parciais podem
estar incompletos. Ainda não é possível retomar uma cadeia.

## Salvar e reabrir uma configuração

**Salvar configuração** grava um `.rove.json` com o caminho da matriz original,
SHA-256, configurações e grupos explícitos. Não copia a matriz nem inicia uma
execução. **Abrir configuração** (**Command-Shift-O**) restaura os ajustes depois
de conferir se a matriz original existe e tem a mesma impressão digital.
Arquivos ausentes ou alterados são informados antes da execução. Revise os
ajustes restaurados antes de iniciar uma nova corrida. Uma configuração não
é um ponto de retomada.

**Copiar comando do Terminal** prepara um comando para o motor Rust incluído
e uma pasta nova em `~/Rove Runs`. Ele lê a matriz no momento da execução.
O mesmo binário, entrada, configuração, grupos e semente produzem as mesmas
amostras científicas pelo app e pelo CLI; o progresso do app não altera a amostragem.

Para automação, `rove sample ... --events-json` emite linhas JSON versionadas com
os eventos `started`, `progress`, `completed`, `cancelled` ou `failed`.
`--expect-input-sha256 HASH` rejeita entradas alteradas antes de criar resultados;
`--cancel-on-stdin-close` permite cancelar quando o processo controlador fecha
o canal de entrada. O app usa essas opções. As [notas do protocolo e processo](https://github.com/getcortexlabs/rove/blob/main/apps/macos/README.md#cli-event-protocol-version-1)
definem campos, validação e códigos de saída.

## Consultar execuções anteriores

![Histórico do Rove com execuções concluídas, canceladas e com falha](/images/macos-history.png)

Abra **Histórico** na barra lateral ou use **Command-Shift-H**. As execuções
aparecem da mais recente para a mais antiga, com matriz, data, situação registrada,
tempo decorrido, modelo e semente. Busque pelo nome da matriz ou pela semente,
ou filtre por situação. Selecione uma execução para ver passos concluídos e
solicitados, configurações, grupos explícitos e impressão digital da entrada
original. Os botões dos arquivos mostram as saídas disponíveis no Finder.

As execuções anteriores do app em `~/Rove Runs` são encontradas automaticamente.
Outros locais escolhidos para salvar resultados ficam lembrados neste Mac.
**Adicionar pasta** inclui uma execução do app salva em outro lugar; escolha a
pasta que contém `session.json` ou `setup.rove.json`. **Atualizar** confere os
arquivos novamente. O app lê apenas metadados pequenos, sem carregar o conteúdo
dos traços ou das árvores, e faz essa leitura fora da thread da interface.

Pastas ausentes, registros ilegíveis e resultados incompletos são identificados.
Uma execução sem término confirmado aparece como **Interrompida**, mesmo que
existam arquivos de saída. Registros antigos continuam legíveis; medidas não
disponíveis aparecem como um traço, sem estimativas. Novas tentativas salvam uma
sessão antes de conferir a entrada, para que cancelamentos imediatos e falhas
ao iniciar também apareçam; uma impressão digital diferente impede a geração
de resultados científicos.

**Usar esta configuração** confere a matriz original e abre os ajustes salvos
para revisão. Não inicia uma análise automaticamente nem retoma uma cadeia.
Durante uma execução, você pode navegar pelo Histórico e voltar por **Análise
atual**; reutilizar uma configuração fica desabilitado até a análise terminar
ou ser cancelada. **Voltar à área de trabalho** (**Command-Shift-R**) preserva
a matriz e os ajustes abertos. O histórico não apaga arquivos nem comprova
convergência.

## Inspecionar pelo terminal

```sh
target/release/rove inspect fixtures/reference/20x50/matrix.nex
```

O comando emite JSON versionado com contagens, uma prévia limitada das células,
o hash da entrada e erros separados de compatibilidade com o amostrador e Mkv.
Adicione `--gaps missing` somente quando essa interpretação for adequada.
Ler uma matriz não grava resultados de análise.
