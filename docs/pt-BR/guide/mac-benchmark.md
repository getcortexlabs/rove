---
description: "Meça o motor incluído no Rove no seu Mac e compartilhe o resultado se quiser."
---

# Testar este Mac

Abra **Testar este Mac** na barra lateral ou no menu Ajuda e escolha **Rodar teste**. O Rove usa o próprio motor e uma matriz sintética incluída no aplicativo. O teste não exige BEAST, Java, ferramentas de desenvolvimento ou internet. O acesso ao beta continua seguindo as regras normais de uso offline.

Para comparar melhor os resultados, conecte o Mac à tomada, desative o Modo Pouca Energia e feche outros aplicativos pesados. Uma análise em andamento impede iniciar o teste; um teste em andamento impede iniciar ou retomar uma análise. Você pode cancelar a qualquer momento. Fechar o Rove também encerra o motor do teste.

## O que medimos

O protocolo fixo `rove-mac-v1` usa 90 táxons e 250 caracteres binários simulados sob Mk, incluindo constantes. O modelo usa quatro categorias gama, relógio lognormal relaxado e nenhum filtro de ascertainment. Cada medição começa do zero com as mesmas sementes, faz 5.000 propostas de aquecimento por cadeia, fora do cronômetro, e depois mede 50.000 propostas por cadeia. O ajuste dos operadores continua ativado.

São três medições com **uma cadeia** e três com **quatro cadeias em paralelo**. O segundo caso faz quatro vezes mais trabalho. Cada cartão mostra o tempo mediano, o total de propostas por segundo e os três tempos individuais, deixando a variação visível. Leitura da matriz, preparação, aquecimento e gravação de resultados de pesquisa ficam fora do intervalo medido. O intervalo paralelo inclui o custo de agendar e aguardar as threads.

Isso mede uma carga fixa de trabalho, não convergência, qualidade científica ou duração da sua análise. Não há comparação com BEAST neste teste. Compare o mesmo protocolo e a mesma impressão digital do motor, mantendo as quantidades de cadeias separadas. Energia, temperatura, tarefas em segundo plano, RAM e número de núcleos podem mudar os tempos mesmo com o mesmo chip.

## Resultados salvos e compartilhamento

Cada teste tem seu próprio registro em `~/Library/Application Support/Rove/Benchmarks`. **Testes anteriores** reabre os resultados; **Resultados salvos** mostra a pasta. Testes cancelados, interrompidos ou inválidos não podem ser compartilhados. O aplicativo nunca envia um teste automaticamente.

**Compartilhar com a Cortex Labs** envia o resultado concluído selecionado ao banco Cloudflare D1 do projeto. Inclui:

- As seis medições, quantidades de cadeias e verificações dos estados finais sintéticos.
- Identificador do modelo do Mac, chip, núcleos físicos/lógicos e de desempenho/eficiência quando disponíveis, memória, macOS, arquitetura e uso de Rosetta.
- Fonte de energia, Modo Pouca Energia e categoria térmica antes e depois do teste.
- Versões e impressões digitais dos executáveis do app e do motor, backend, configurações do protocolo e impressão digital da entrada.
- Um identificador aleatório para este teste e sua data.

O registro não inclui matriz, árvore, trace ou caminho de arquivo de pesquisa, número de série do Mac ou email do beta. O envio usa a sessão existente do beta para autorização; o servidor mantém identificadores temporários de conta/IP transformados em hashes para limitar envios, separados dos resultados. Não há ranking público nem identificação permanente do dispositivo. Os resultados são informados pelo cliente, não certificados pelo hardware.

Se o envio falhar, o resultado fica no Mac e pode ser enviado novamente. Reenviar o mesmo registro não cria outra entrada no banco. Para pedir a remoção de um resultado compartilhado, informe o ID do teste que está no JSON local. Apagar o arquivo local não remove a cópia compartilhada.

O [protocolo e a entrada sintética](https://github.com/getcortexlabs/rove/tree/main/fixtures/benchmarks) são versionados com o motor. O CLI também oferece `rove benchmark`, com progresso e medições em linhas JSON, sem gravar saídas de análise.
