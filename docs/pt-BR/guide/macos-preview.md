---
description: "Compile e explore a primeira interface nativa do Rove: importe uma matriz, examine os caracteres e prepare uma análise."
---

# A prévia para Mac

O Rove tem uma primeira interface nativa para macOS, feita com SwiftUI e
AppKit. Ela importa matrizes, prepara a configuração e executa o motor Rust
incluído com progresso real e cancelamento. Não precisa de BEAST nem Java.

![A interface nativa do Rove](/images/macos-preview.png)

## Atualizações e controles no app

As versões assinadas para distribuição têm **Rove → Buscar atualizações…** e a
seção **Atualizações** nos Ajustes. A busca automática vem ligada; baixar e
instalar ao sair é opcional. A instalação espera os cálculos terminarem. Se você
pediu para instalar durante um cálculo, termine o trabalho e escolha **Instalar
atualização e reabrir**. Builds locais não consultam o feed de distribuição.
Quem está na versão 0.1.1 precisa instalar uma primeira atualização manualmente,
porque ela ainda não tem atualizador. A primeira atualização publicada também
exige configurar a chave de assinatura e testar a troca entre dois builds
assinados; veja o [procedimento de release](https://github.com/getcortexlabs/rove/blob/main/scripts/RELEASES.md).

O plano de exploração aceita de 1 a 16 cadeias independentes. Em **Plano de
amostragem personalizado**, informe os passos e um intervalo positivo que divida
esse total. O burn-in da comparação também pode ser alterado. Em **Amostragem
avançada → Cálculo**, escolha Automático, ARM NEON ou Portável e o intervalo de
atualização do progresso. Automático usa a implementação nativa. Atualizar o
progresso não altera a frequência de salvamento das amostras.

**Arquivo → Abrir arquivo de traço…** analisa um traço externo sem alterá-lo.
O burn-in pode ser uma porcentagem ou uma quantidade de amostras salvas, incluindo
o estado inicial. Quantidade de amostras não é quantidade de passos de MCMC.
Os relatórios ficam na pasta de saídas escolhida no Rove. **Histórico → Adicionar
pasta de execução** também importa amostragens concluídas pela CLI: escolha a
matriz original para conferir sua assinatura. O app cria uma pasta de histórico
que aponta para os resultados; mantenha a pasta original disponível.

**Estimar evidência do modelo…**, na revisão, abre um plano de power posteriors
sem exigir uma execução anterior. A configuração fica no histórico como
**Configuração de modelo**. Potências, alpha, passos, intervalo de amostragem,
burn-in e semente podem ser alterados. Execuções já concluídas oferecem os mesmos
controles. A estimação de parâmetros do processo FBD continua excluída, pelos
motivos científicos explicados no guia de comparação de modelos. As verificações
preditivas também permitem definir o máximo de tentativas de simulação por
caráter; atingir o limite encerra a verificação sem flexibilizar o modelo.

Recursos de análise da CLI precisam chegar ao Swift na mesma alteração. Eventos
JSON, assinatura dos arquivos e cancelamento são automáticos. Essa regra e o mapa
de comandos fazem parte das orientações de contribuição e das verificações do CI.


## Acesso ao beta e primeira visita

O beta nativo começa com **Já tenho convite** e **Entrar na lista**. Use um e-mail
aprovado para pedir um código de oito dígitos, válido por dez minutos. Entrar na
lista também exige confirmar o e-mail, mas registra apenas seu interesse. Isso não
libera o aplicativo. A equipe do projeto aprova os convites.

Depois de confirmar o acesso, uma carta do Rafael agradece ao Thiago J. Silva
antes da apresentação. Ao clicar em **Continuar**, ela fica marcada como lida
neste Mac e não reaparece ao sair da conta ou atualizar o app. A carta acompanha
o idioma do aplicativo. Quem já usa o Rove também a vê uma vez, sem repetir a
apresentação que já concluiu.

Em seguida, uma apresentação em quatro partes mostra o aplicativo, as
matrizes, os grupos externos e os resultados. Você pode pular, testar o exemplo ou
abrir sua matriz; nenhuma análise começa sozinha. Reabra em **Ajuda → Conhecer o
Rove**. **Ajuda → Novidades do Rove** reúne os destaques por versão e as notas
anteriores. As novidades aparecem uma vez por atualização documentada, quando não
há uma operação em andamento. A primeira apresentação não emenda outra janela de
novidades. Reabrir essas telas preserva a matriz e suas configurações.

O acesso fica salvo no Chaves do macOS. O Rove confere o convite pela internet ao
abrir e a cada seis horas enquanto está ativo. Após uma verificação bem-sucedida,
você pode usar o aplicativo sem internet por até sete dias. Um convite expirado
ou revogado impede novas análises e retomadas de pontos salvos; uma execução já
em andamento pode terminar. Para sair da conta, use o menu Ajuda.

A Cloudflare verifica o acesso e o Resend entrega os códigos. O serviço guarda
seu e-mail, situação do convite, interesse confirmado, idioma e horários de acesso.
Matrizes, árvores e arquivos das análises ficam no Mac. A lista não é uma assinatura
de newsletter. Fale com a [Cortex Labs](https://getcortexlabs.com/) para pedir a
remoção da lista. A CLI Rust independente não exige acesso ao beta nem esses
serviços de rede.

Depois de configurar os avisos de lançamento, participantes com convite ativo recebem e-mails de novas versões, com link para baixar no Mac e ver as novidades. Estar apenas na lista de interessados não gera esses avisos. Para sair do beta, responda a um desses e-mails.

## Compilar e abrir

Cada [release](https://github.com/getcortexlabs/rove/releases) traz o `Rove-<versão>.dmg` para Apple silicon, assinado com um certificado Developer ID e notarizado pela Apple: abra a imagem, arraste o Rove para Aplicativos e abra-o. O app tem como destino macOS 14 ou mais recente.

Para compilar por conta própria, instale Xcode 26.3 ou mais recente e Rust em um Mac compatível com essa versão do Xcode. Execute na raiz do repositório:

```sh
scripts/build-macos.sh
open build/macos/Rove.app
```

Isso gera um aplicativo local com assinatura ad hoc. O `scripts/package-macos.sh` compila a configuração de release e a empacota em uma imagem de disco; com `CODESIGN_ID` e `NOTARY_PROFILE` definidos, assina e notariza o app e a imagem da mesma forma que as releases são construídas. A interface usa um tema escuro inspirado na
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

Matrizes binárias e multiestado podem ser analisadas. **Partições por estados**
agrupa os caracteres pelos estados observados/possíveis ou usa explicitamente
o alfabeto declarado. A matriz e a revisão mostram as quantidades por partição.
Entradas incompatíveis recebem uma explicação. Gaps exigem uma escolha
explícita para serem tratados como ausentes; caracteres potencialmente
constantes não podem usar Mkv.

## Preparar e executar uma análise

Escolha o relógio estrito ou a [configuração de referência com relógio relaxado](./reference-model),
Mk, Mkv ou [Mkparsinf](./character-selection), tamanho da cadeia, intervalo de amostragem e semente. Os priors
mantêm os padrões do motor. Em Modelo, **Adicionar grupo** seleciona os terminais
que devem formar um clado. Os grupos precisam estar contidos um no outro ou ser
separados. Nada é imposto automaticamente, nem o grupo interno do exemplo.
Suporte imposto por uma restrição não é recuperação independente pelos dados.

Em **Enraizamento → Táxon usado para enraizar**, escolha um terminal como T20. Ele continua como uma ponta, do lado oposto ao clado imposto de todos os demais táxons. Esses táxons não são necessariamente um grupo interno biológico. A ponta não é a raiz ancestral, e seus estados observados não são atribuídos ao ancestral. A escolha não força uma direção como 0 → 1. O controle aceita um táxon e começa em **Não selecionado**. Incluir táxons, restringir o enraizamento e girar a árvore na tela são escolhas separadas.

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
durante a execução. O tempo estimado usa progresso medido; ESS e suporte exigem seus próprios diagnósticos.

Cada execução recebe uma pasta nova, por padrão em `~/Rove Runs`. **Abrir pasta**
mostra a configuração salva, o registro da sessão, os eventos de progresso e o
log do motor. A subpasta `results` contém `trace.log`, `trees.nexus` e `run.json`.
Após o término, os botões Traço, Árvores e Registro da execução mostram esses
arquivos no Finder. **Concluída** significa que os passos solicitados terminaram
e os metadados da saída foram conferidos. Não significa convergência. Compare
execuções independentes e avalie os diagnósticos de amostragem e árvores antes
de interpretar os resultados; veja o [guia de amostragem](./sampler).

**Pausar análise** salva no próximo passo amostrado e encerra o motor. Em um grupo, todas as cadeias pausam juntas. No **Histórico**, selecione a execução e clique em **Retomar análise**; o cartão mostra o passo salvo. É preciso manter a mesma matriz, configurações e executável do Rove, além da pasta original da execução.

**Cancelar análise** pede confirmação e mantém os arquivos parciais. Fechar o app também encerra o motor. Se ocorrer uma interrupção, o Histórico oferece recuperação quando existe um checkpoint; os passos posteriores ao último salvamento são refeitos. Execuções antigas sem checkpoint não podem ser retomadas. Uma configuração salva continua servindo para iniciar uma execução nova; retomar continua uma execução existente.

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

**Comparar com outras execuções** aparece numa execução concluída. Lista as
outras execuções concluídas do Histórico com a mesma matriz, modelo, restrições
e intervalo de amostragem e sementes que não se sobrepõem, inclusive grupos.
Escolha as execuções e um burn-in, e o app executa `rove compare` nas pastas de
cada cadeia. O relatório é o mesmo mostrado para um grupo de cadeias, com uma
coluna por cadeia, e fica salvo na pasta `comparisons` da execução. Concordância
entre execuções não é convergência de nenhuma delas.

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
descartadas. O burn-in também pode ser dado como número de amostras, como em
`--burnin-samples`. Execuções parciais, canceladas ou interrompidas não podem
ser resumidas pelo app.

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

**Árvores de outro programa.** **Arquivo → Resumir arquivo de árvores…** abre uma amostra de árvores NEXUS gravada pelo Rove, pelo BEAST 2 ou por outro programa, pede o burn-in e as alturas dos nós, e salva o resumo em uma pasta com o nome do arquivo dentro da pasta de execuções. O `tree-source.json` da pasta registra o caminho e o SHA-256 do arquivo, e o Histórico a lista como arquivo de árvores, com seus resumos e o explorador. Nenhum grupo externo é conhecido, já que o Rove não rodou a amostra aqui, e o resumo registra a conclusão como não verificada, a menos que o arquivo esteja ao lado de um registro de execução do Rove. A tela inicial e a barra lateral oferecem a mesma entrada, e uma amostra de árvores escolhida ou arrastada na importação da matriz abre o painel de resumo diretamente.

O comando Rust independente faz o mesmo cálculo, sem Java ou BEAST:

```sh
target/release/rove summarize /path/to/results/trees.nexus \
  --output /path/to/new-summary --burnin-percent 25 --heights ca
```

Arquivos de árvores gravados pelo BEAST 2, que trazem um ramo de raiz de
comprimento zero e uma taxa na raiz, são aceitos; a taxa da raiz é descartada.
Use `--burnin-samples N` para contar amostras ou `--heights keep` para manter
alturas. A pasta de saída não pode existir. A extensão não muda a interpretação.
Sem `run.json` ao lado da entrada, a conclusão é registrada como **unverified**:
um arquivo sintaticamente completo não comprova que a execução terminou.
Com o registro, o comando verifica conclusão, contagem e cada intervalo amostrado.

O leitor aceita árvores temporais do Rove enraizadas e binárias, com pontas na idade zero
ou alturas explícitas de fósseis, tabela de tradução e identificadores `STATE_` crescentes.
Táxons misturados, ausência de `End;`, taxas incompletas e formatos de árvore
não suportados são rejeitados. Veja [datação de fósseis](./tip-dating). Politomias e blocos
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
nativos de navegação. Apontar para um nó ou ponta destaca o caminho até a
raiz sem mudar a seleção. Essas operações não têm animação.

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
automática por esses números nessa tela de uma cadeia. A parada opcional entre várias cadeias é configurada no plano de exploração.

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

## Comparando cadeias independentes

![Comparação de quatro cadeias independentes curtas](/images/macos-chains.png)

Em **Modelo → O plano de exploração**, escolha **1 a 16 cadeias**. O número de passos vale para cada cadeia. Várias cadeias rodam ao mesmo tempo, com sementes distintas e registradas; o progresso mostra cada uma separadamente. Cancelar encerra o grupo inteiro.

Os grupos concluídos e seus detalhes no Histórico incluem **As cadeias concordam?**, com R-hat e ESS para todas as estatísticas registradas e comparação das frequências de clados. Selecione uma cadeia para abrir seus diagnósticos, resumir suas árvores ou ver sua pasta. As configurações salvas preservam a quantidade de cadeias; configurações antigas continuam com uma cadeia. Os arquivos ficam separados para que o burn-in de uma não seja tratado como continuação de outra.

**Parar quando os critérios dos diagnósticos forem atingidos** é opcional e vem desligado. Os padrões são 10% de burn-in da comparação, ESS ≥ 200 por cadeia e R-hat ≤ 1,01 em todas as estatísticas escalares registradas, pelo menos 1.000 amostras mantidas por cadeia e duas verificações positivas seguidas. O passo e o motivo da parada são salvos. São critérios de diagnóstico escalar, não uma garantia de convergência das topologias; confira clados e traces. Veja [cadeias independentes](./sampler#cadeias-independentes) para opções e limites da CLI.

**Amostragem avançada**, recolhida abaixo do plano de exploração, expõe as demais opções do `rove sample`. **Amostrar só os priors** roda sem os caracteres. Com a parada automática ligada, o ESS mínimo, o R-hat máximo e o intervalo de verificação podem substituir os padrões do motor, 200, 1,01 e o cronograma próprio dele. O intervalo de checkpoint pode ser dado em passos em vez do padrão de cerca de 30 segundos. O esquema de propostas oferece o padrão clássico, os dois esquemas avaliados nos relatórios do [relógio relaxado](https://github.com/getcortexlabs/rove/blob/main/benchmarks/sampling/2026-10-29/README.md) e da [frequência da gama](https://github.com/getcortexlabs/rove/blob/main/benchmarks/gamma-mixing/2026-10-30/README.md), e pesos e tamanhos de passo iniciais personalizados por movimento, com o ajuste de passo que pode ser desligado. Cada opção é validada antes de o motor iniciar, gravada no registro da execução e no Histórico, e salva nas configurações como formato 5, que versões anteriores do app recusam em vez de rodar sem ela. Nenhum dos esquemas avaliados é recomendado; os relatórios registram o que cada um ganhou e perdeu.

## Verificações preditivas posteriores

Execuções concluídas no Histórico incluem **O modelo consegue reproduzir a matriz?**, que roda o [`rove predict`](./posterior-prediction) sobre a execução com burn-in, número de réplicas e semente escolhidos; um grupo é verificado uma cadeia por vez. Progresso e cancelamento funcionam como nas outras ferramentas, e uma verificação cancelada não deixa arquivos. Cada verificação salva lista, para o comprimento de parcimônia, os índices de consistência e retenção, a surpresa do padrão e os caracteres informativos, os pares válidos, as frações de cauda inferior e superior com seus intervalos de Wilson de 95% e os empates, exatamente como o `predictive.json` registra. As verificações ficam na pasta `predictions` da execução, junto com as matrizes réplicas. Frações de cauda pequenas marcam uma discrepância a investigar; o app não aplica nenhum limiar.

## Comparação de modelos

Os mesmos detalhes incluem **Qual modelo explica melhor a matriz?**, que roda o [`rove marginal`](./model-comparison) para o modelo e as restrições da execução com número de potências, alfa, passos por potência, burn-in e semente escolhidos. O intervalo de amostragem, os pesos de proposta e os tamanhos de passo da execução são reaproveitados; cadeias, checkpoints e parada automática não estão disponíveis, e nenhuma estimativa é oferecida com parâmetros FBD estimados. O progresso mostra a potência e o passo atuais; uma estimativa cancelada mantém os traces parciais na pasta da execução, mas não é listada. Cada estimativa salva mostra os valores de stepping stone e path sampling com seus erros padrão e a tabela por potência do `marginal.json`. Quando outras execuções concluídas da mesma matriz, das mesmas restrições e da mesma política de gaps têm estimativas salvas, o painel as lista com a diferença em relação à estimativa mais recente desta execução e um erro padrão que combina os dois erros em quadratura; esses dois números derivados são os únicos que o app calcula. As ressalvas do guia valem: a estimativa inclui os priors, Mk, Mkv e Mkparsinf descrevem processos de observação diferentes, e modelos datados só se comparam com configurações de datação idênticas.

## Testar este Mac

[Testar este Mac](./mac-benchmark)

## Atalhos de duração e estimativas de tempo

O plano oferece 1, 10, 20, 30, 50 e 100 milhões de passos, com intervalos de 100, 1.000, 2.000, 3.000, 5.000 e 10.000 passos. Cada atalho registra 10.000 amostras mais `STATE_0`, antes do burn-in. São estados salvos, não amostras independentes nem uma meta de ESS. Escolher um atalho aplica os dois valores; mudar o intervalo torna o plano personalizado. Configurações reabertas mantêm seus valores exatos. Se os intervalos explícitos de diagnóstico ou checkpoint deixarem de ser múltiplos da amostragem, confira os valores arredondados para cima e clique em **Aplicar ajustes de intervalo**, ou edite-os manualmente, antes de iniciar.

Durante a execução, **Tempo estimado até o limite de passos** e **Tempo por milhão de passos** usam diferenças recentes de passos e de tempo ativo do motor. O Rove primeiro coleta pelo menos três intervalos cobrindo dez segundos e depois usa até oito intervalos recentes. Cada cadeia é medida separadamente; a estimativa do grupo é o maior tempo restante. Retomar depois de uma pausa inicia uma nova medição, sem contar o tempo parado. As estimativas desaparecem ao interromper ou concluir. A carga do processador pode mudar o ritmo. A previsão é até o limite de passos, não até a convergência; a parada automática pode encerrar antes.

**O que são estes arquivos?**, nos resultados e no Histórico, abre uma explicação offline. O [guia da pasta da execução](./run-files) mostra os arquivos e o caminho do traço até a árvore anotada.

### Orientação visual sobre o modelo

O painel de enraizamento desenha a divisão imposta: o terminal escolhido de um lado e um grupo recolhido com todos os demais do outro. Ele não estima comprimentos de ramos, relações internas ou estados ancestrais. O tour usa a mesma terminologia de enraizamento.

Abra **Prior do sigma do relógio** para ver a densidade gamma mudar com Alpha e Beta, na parametrização forma/escala. Uma linha tracejada marca a esperança do prior; o painel também mostra o desvio padrão. O trecho desenhado não é um intervalo de credibilidade. Com forma menor que um, a densidade diverge em zero e a curva começa acima de zero. Escalas numéricas extremas podem não ter prévia gráfica; isso não muda o prior do motor.

A previsão de duração estima o limite de passos solicitado usando o tempo ativo recente do motor. Ela não prevê quando a convergência será estabelecida. As opções de seleção de caracteres e datação explicam suas hipóteses junto das mensagens de validação; essas explicações não demonstram adequação a um estudo específico.

## Revisão das evidências e pacotes de reprodução

Em **Histórico → Revisão das evidências**, escolha o burn-in e atualize a revisão. O arquivo `assessments/review-…/assessment.json` registra uma fotografia datada com identificadores SHA-256 das fontes, registros originais, diagnósticos por cadeia, uma comparação nova entre cadeias e caminhos dos relatórios de árvores e preditivos correspondentes. Atualize após gerar novos relatórios. O burn-in e a opção de incluir a matriz ficam salvos localmente junto à execução.

Conclusão, amostragem dos parâmetros, concordância entre cadeias, árvores e evidências preditivas têm estados separados. Os critérios para parâmetros são pelo menos 1.000 amostras retidas e ESS de 200 por cadeia, sem truncamento de defasagens. A comparação entre cadeias exige também sementes distintas, entrada/modelo correspondentes e R-hat dividido e normalizado por postos ≤ 1,01. Somente parâmetros comprovadamente fixos pelo modelo são excluídos. Uma cadeia única não permite avaliar concordância entre cadeias. Esses critérios de diagnóstico não comprovam convergência nem substituem os limites configuráveis de parada da execução. Suporte de árvores e frações de cauda preditivas exigem interpretação; evidências ausentes ou inconsistentes ficam visíveis. Execuções apenas do prior descrevem a exploração do prior, não a adequação posterior.

**Exportar pacote de reprodução…** cria uma pasta local nova fora da execução. `manifest.json` registra tamanho e SHA-256 de cada arquivo. `assessment.json` contém uma revisão nova; `run/` contém registros conhecidos do Rove, amostras, checkpoints e pastas de resultados derivados. A matriz original só é incluída com a opção **Incluir a matriz original na exportação**, mediante confirmação do identificador registrado. A revisão indica originais alterados ou indisponíveis. Arquivos vizinhos desconhecidos, revisões anteriores e fontes externas citadas em comparações não são copiados. A exportação preserva caminhos registrados e nomes de táxons, recusa links simbólicos dentro dos artefatos e destinos existentes, verifica alterações das fontes e mantém os originais intactos. O executável não é incluído: a retomada exata exige checkpoint válido e a identidade original do executável, entrada e modelo; revise os caminhos antes de reutilizar a configuração.

As mesmas operações estão disponíveis no motor:

```sh
rove assess run-folder --output new-review-folder --burnin-percent 10
rove export-run run-folder --output new-package-folder --burnin-percent 10
# Incluir explicitamente uma matriz original correspondente:
rove export-run run-folder --output another-package --include-matrix matrix.nex
```

A origem pode ser uma pasta de execução do app ou da CLI. Execuções incompletas podem ser revisadas e arquivadas, preservando seu estado. Registros de origem inválidos ou estruturas de arquivos inseguras são recusados. Os pacotes são pastas locais, sem envio para a internet.

As verificações preditivas salvas também mostram um histograma de estatísticas pareadas **simulado − observado**. Cada par usa a mesma árvore posterior; a estatística de parcimônia observada pode variar entre pares. Zero indica igualdade. O gráfico confere contagens, empates e caudas com `predictive.json`, informa pares indefinidos e fica indisponível quando os registros divergem. Não avalia convergência nem transforma uma fração de cauda em decisão automática de adequação.

Cópias da matriz original guardadas pelas verificações preditivas em `source/matrix.nex` são omitidas do pacote, mesmo quando a verificação é incluída. A inclusão explícita escreve uma única `matrix.nex` verificada na raiz do pacote; `omitted_source_matrices` lista as cópias omitidas. Matrizes réplicas simuladas continuam incluídas.

Registros nativos de execuções importadas da CLI podem vincular a pasta `results` principal; essa origem declarada é copiada como arquivos comuns no pacote.
