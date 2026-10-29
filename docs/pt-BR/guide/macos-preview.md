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

**Rove → Sobre o Rove** mostra a origem do projeto, a versão do app e os
créditos de Rafael Augusto (software e design) e Thiago J. Silva (biologia e
pesquisa). A janela acompanha o idioma do app e tem links para o site, o guia
e o repositório.

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

Em **Grupo externo**, você pode escolher um terminal, como T20. Todos continuam
na análise: escolher T20 obriga T1–T19 a formar um clado, com T20 do outro lado
da divisão da raiz. O grupo externo é um terminal, não o ancestral na raiz.
Este controle aceita um grupo externo e começa em **Não selecionado**.

Em **Grupos adicionais**, adicione grupos ou use o lápis para editar um existente.
Marcar terminais prepara uma seleção; **Aplicar grupo** confirma e **Cancelar**
descarta as mudanças pendentes. Terminais desmarcados continuam na análise.
Conflitos com o grupo externo impedem a execução. Remover o grupo externo mantém
os grupos aplicados manualmente; confira **Grupos aplicados** na Revisão.

A Revisão mostra a divisão da raiz, os grupos aplicados e todos os terminais
mantidos. Os mesmos dados aparecem em **Configuração aplicada à análise** durante
a execução e nos detalhes do Histórico. Salvar/reabrir e **Usar esta configuração**
preservam a escolha; outra matriz ou **Nova análise** começa sem grupo externo.
Configurações antigas mantêm os grupos originais sem presumir um grupo externo.
Os novos arquivos no formato 2 exigem o app atualizado. Alterar essas escolhas
prepara outra análise, não muda a raiz de resultados existentes. O suporte de
um grupo imposto vem da restrição.

![Escolha explícita do grupo externo no aplicativo](/images/macos-outgroup.png)

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

## Resumir as árvores posteriores

![Resumo das árvores no Rove](/images/macos-summary.png)

Após concluir uma execução, escolha **Resumir árvores** no resultado ou na
entrada do Histórico. Defina o burn-in e as alturas dos nós e clique em **Criar
resumo das árvores**. Os resumos salvos reaparecem ao abrir esse painel. O
processamento ocorre fora da thread da interface e pode ser cancelado.

O burn-in descarta as primeiras `floor(total de amostras × porcentagem / 100)`
árvores, arredondando para baixo. `STATE_0` conta como amostra. Os 10% iniciais
são uma sugestão editável, não uma avaliação de convergência; escolha após
inspecionar o trace. O painel mostra quantas árvores foram mantidas e
descartadas. Execuções parciais, canceladas ou interrompidas não podem ser
resumidas pelo app.

MCC (máxima credibilidade dos clados) seleciona uma **topologia amostrada**, não
um consenso de maioria. O Rove soma os logaritmos das frequências dos clados
internos de cada árvore mantida. Os clados são ordenados antes da soma; em
empates exatos, vence a primeira candidata mantida. `posterior` é a fração das
árvores mantidas que contém o clado. Grupos impostos por restrições registradas
recebem `constrained=true`; esse suporte não é recuperação independente dos dados.

**Manter a árvore selecionada** preserva suas alturas. **Média do ancestral
comum** usa a altura média do ancestral comum mais recente de cada grupo em
**todas** as árvores mantidas. Comprimentos dos ramos são diferenças entre
alturas do pai e do filho, nas unidades de tempo da origem. Taxas continuam
sendo multiplicadores separados; não se inferem datas de calendário nem
distâncias multiplicadas pelas taxas. Médias e faixas de altura/taxa são
condicionais à presença do clado; `CAheight_mean` e `CAheight_range` usam todas
as árvores mantidas. Faixas são mínimos e máximos, **não intervalos HPD ou de
credibilidade**. Medianas de altura e intervalos HPD ainda não estão implementados.

Cada resumo cria uma pasta nova em `summaries`, dentro da execução:

- `mcc.tree`: uma árvore NEXUS enraizada e anotada para o FigTree. Escolha
  `posterior` em Node Labels no FigTree para exibir os suportes.
- `samples.trees`: cópia idêntica de todas as amostras originais, incluindo o
  burn-in, para ferramentas que esperam a extensão `.trees`.
- `summary.json`: hashes da origem e do executável, metadados da execução,
  burn-in, contagem mantida, estado escolhido, método, alturas e versão do Rove.

Os botões mostram esses arquivos no Finder. `results/trees.nexus` e os demais
arquivos originais são preservados. Cada resumo ocupa espaço para mais uma
cópia das árvores brutas. Resumo e suporte alto não comprovam convergência;
compare execuções independentes e diagnósticos de amostragem.

O comando Rust independente faz o mesmo cálculo, sem Java ou BEAST:

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

Use `--burnin-samples N` para contar amostras ou `--heights keep` para manter
alturas. A pasta de saída não pode existir. A extensão não muda a interpretação.
Sem `run.json` ao lado da entrada, a conclusão é registrada como **unverified**:
um arquivo sintaticamente completo não comprova que a execução terminou.
Com o registro, o comando verifica conclusão, contagem e cada intervalo amostrado.

O leitor aceita árvores do Rove enraizadas, binárias e ultramétricas, com pontas
contemporâneas, tabela de tradução e identificadores `STATE_` crescentes.
Táxons misturados, ausência de `End;`, taxas incompletas e formatos de árvore
não suportados são rejeitados. Datas de fósseis nas pontas, politomias e blocos
NEXUS externos arbitrários não são suportados. Os limites são 4.096 táxons,
512 níveis, 2 MiB por comando e um milhão de clados distintos ou uma estimativa
de 128 MiB para suas chaves, o que ocorrer primeiro. Essa estimativa não é um
teto rígido de memória do processo. Quatro passagens sequenciais guardam
contagens e estatísticas da árvore escolhida, não todas as árvores na memória.

[A validação e as medições em Apple silicon](https://github.com/getcortexlabs/rove/blob/main/benchmarks/tree-summary/2026-10-29/README.md)
registram comparações com TreeAnnotator e leitores NEXUS externos em versões fixas.

## Explorar uma árvore anotada

![Explorador nativo da árvore MCC](/images/macos-tree.png)

Em um resultado concluído ou no Histórico, abra **Resumir árvores** e escolha
**Ver árvore** em um resumo salvo. Uma janela nativa separada abre o resultado
MCC; ela não navega pelas amostras brutas nem apresenta `STATE_0` como resultado
final. O cabeçalho identifica a execução e as quantidades retidas e descartadas.
**Sobre este resumo** mostra burn-in, política de alturas, amostra de origem e
grupos impostos.

Clique em um nó ou ponta para consultar suporte, comprimento do ramo, alturas
e intervalos de taxas disponíveis. Expanda **Táxons descendentes** para navegar
pelos filhos ou selecionar uma ponta. Anotações ausentes aparecem como
**Indisponível**. Losangos marcam grupos impostos; seu suporte não é evidência
independente. Um grupo externo verificado é identificado como ponta, não como
nó da raiz. A ordem vertical das pontas não determina o enraizamento.

Role ou arraste para mover, use pinça ou os botões de zoom para ampliar e
**Ajustar largura** para voltar a 100%. Árvores altas continuam roláveis para
manter os rótulos legíveis. A busca encontra táxons em clados recolhidos e
expande seus ancestrais ao selecioná-los. **Girar**, **Recolher clado** e
**Restaurar visualização** mudam somente o desenho. Com o desenho em foco, use
Cima/Baixo para percorrer nós visíveis, Esquerda para o ancestral, Direita para
um filho e Espaço para recolher ou expandir. O painel também oferece botões
nativos de navegação. Essas operações não têm animação.

**Suporte** controla os rótulos; ramos curtos e linhas próximas omitem alguns
para evitar sobreposição, mas os valores continuam disponíveis no painel.
Nomes longos são abreviados no desenho e aparecem completos na busca e no painel.
A escala e os ramos usam unidades de tempo da origem. As taxas são multiplicadores
separados; os intervalos observados não são intervalos HPD.

**Exportar SVG** salva toda a disposição exibida, incluindo giros e clados
recolhidos, com escala, legenda e origem do resumo. Zoom e rolagem escolhem a área
visível, não um recorte da figura exportada. SVG é uma imagem vetorial; `mcc.tree`,
as árvores brutas e os arquivos da execução permanecem intactos. Esse visualizador
não inclui PNG, importação de árvores externas ou reenraizamento.

O comando incluído `rove inspect-tree /caminho/do/resumo` valida o registro salvo
e a impressão digital da árvore antes de emitir JSON versionado para exibição.
Leitura, disposição, busca e geração de SVG acontecem fora da thread da interface.
O visualizador aceita uma árvore MCC do Rove, até 16 MiB, 4.096 táxons e 512 níveis
de aninhamento; os limites do leitor de amostras brutas acima continuam iguais.
Um resumo continua sem comprovar convergência.

## Inspecionar os diagnósticos

![Diagnósticos do traço com 25% de burn-in no aplicativo nativo](/images/macos-diagnostics.png)

Escolha **Diagnósticos** em um resultado finalizado ou em uma entrada do Histórico
com arquivo de amostras. A janela lista todos os parâmetros, começando por
posterior, likelihood e prior. Selecione uma linha para ver média, desvio padrão,
ESS, intervalo de 95% com caudas iguais e gráfico. `posterior` é uma **densidade
em log**, não o suporte posterior de um nó.

Altere **Burn-in (%)** e clique em **Aplicar**. As contagens e o primeiro estado
mantido correspondem ao cálculo aplicado; uma edição pendente não muda o rótulo
do resultado anterior. A regra é a mesma do resumo de árvores: descartar
`floor(N × porcentagem / 100)` amostras, incluindo `STATE_0` numa execução padrão.
**Resumir árvores com este burn-in** abre o painel com a porcentagem aplicada.
Os dois leitores conferem os estados e os intervalos com o registro da execução.

**Execução inteira** inclui o começo descartado em cinza e marca o limite do
burn-in. **Após burn-in** ajusta o gráfico às amostras mantidas sem mudar os
cálculos. O controle abaixo do gráfico permite consultar estado e valor pelo
teclado. Tabela, busca e controles têm navegação nativa; não há animação na troca
do gráfico. A exibição usa até 1.028 pontos por parâmetro, preservando primeiro,
mínimo, máximo e último de cada trecho, além do limite do burn-in. **Os cálculos
usam todas as amostras mantidas**, nunca apenas os pontos reduzidos do gráfico.

O ESS estima informação equivalente a amostras independentes, não o número de
passos da MCMC. Usamos a convenção de autocovariâncias positivas em pares do
BEAST 2.7.8, com defasagens abaixo de 2.000. Não é ESS de centro ou caudas por
normalização dos postos. Séries constantes ou com menos de quatro valores
mantidos ficam com ESS indisponível. Se a correlação positiva chegar ao limite
de defasagens avaliado, isso é indicado. Não há selo de convergência nem parada
automática por esses números.

O intervalo usa os percentis 2,5 e 97,5 com interpolação linear (tipo 7 do R).
Ele descreve a distribuição mantida; **não é HPD** nem o erro de Monte Carlo da
média. Uma única amostra não tem intervalo, e séries muito curtas são sinalizadas.
Mistura dos parâmetros, acordo entre corridas independentes e convergência das
topologias são coisas distintas: essa tela não calcula R-hat ou compara clados.

Cada cálculo bem-sucedido salva `diagnostics/trace-*/diagnostics.json` numa pasta
nova, com hash da origem, burn-in, estados, métodos, estatísticas e pontos.
**Registro do cálculo** mostra o arquivo no Finder. Ao reabrir, a última
porcentagem válida é recuperada e os dados atuais são recalculados. Relatórios
anteriores e arquivos originais ficam intactos. Leitura e cálculo acontecem fora
da thread da interface e podem ser cancelados enquanto outras análises continuam.

Execuções parciais ou não verificadas são identificadas. Valores não finitos,
colunas inconsistentes, estados irregulares e última linha incompleta são
recusados. Também recusamos uma execução registrada como concluída se faltarem
amostras. Sem `run.json`, um arquivo válido não comprova conclusão. Os limites
são 128 MiB, um milhão de linhas, 128 parâmetros e oito milhões de células
numéricas; não são um teto para toda a memória do processo.

O comando independente usa o mesmo cálculo Rust:

```sh
target/release/rove diagnose /caminho/results/trace.log --burnin-percent 25 \
  --output /caminho/novo-diagnostico
```

Sem `--output`, o resultado JSON vai para a saída padrão. `--burnin-samples N`
substitui a porcentagem por uma quantidade. A pasta de saída precisa ser nova.
Não é necessário Java ou Tracer.
[A validação e as medições](https://github.com/getcortexlabs/rove/blob/main/benchmarks/diagnostics/2026-10-29/README.md)
incluem séries sintéticas e uma referência BEAST com versão fixada.
