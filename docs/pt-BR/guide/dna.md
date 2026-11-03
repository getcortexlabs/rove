---
description: "DNA alinhado em NEXUS e FASTA com JC69, ambiguidades e tratamento explícito de gaps."
---

# DNA alinhado com JC69

[Morfologia e DNA combinados](./mixed-data.md).

Abra no app uma matriz NEXUS `DATATYPE=DNA` ou um arquivo FASTA já alinhado.
A importação seleciona **DNA · JC69**, mantém A/C/G/T possíveis em cada sítio
e inclui sítios constantes. Rove não alinha, recorta ou monta leituras.
O mesmo arquivo funciona com `sample`, `inspect` e `marginal`:

```sh
rove sample alignment.fasta --output dna-run --seed 42 --steps 30000000 --sample-every 3000 --model reference --gaps missing
```

Escolha `--gaps missing` somente quando gaps não devem informar o estado.
O padrão rejeita gaps. `?` é ausente; `R` permite A/G, `Y` C/T, e os demais
códigos IUPAC mantêm seus estados possíveis. `N` permite os quatro. São
máscaras de verossimilhança com valores um, não probabilidades fracionadas
nem estados adicionais. Polimorfismo entre parênteses é preservado, mas não
aceito na inferência. DNA usa `--conditioning none --partitioning declared`;
Mkv, Mkparsinf e recodificação morfológica explícitos são rejeitados.

Sequências FASTA podem ocupar várias linhas e devem ter comprimentos iguais
e positivos. O primeiro identificador após `>`, até o primeiro espaço, nomeia
o táxon; a descrição restante é ignorada, conforme a [convenção de cabeçalho FASTA](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/).
A convenção NEXUS já usada pelo Rove também vale para FASTA: sublinhados viram
espaços, duplicatas são verificadas sem diferenciar maiúsculas ASCII e o resto
da grafia é mantido. Confira os nomes exibidos antes de escolher a raiz ou
combinar dados. Os arquivos originais não são alterados.

Em NEXUS DNA, bases não distinguem maiúsculas, o alfabeto é A/C/G/T fixo e o
gap padrão é `-`. `SYMBOLS="ACGT"` é aceito; alfabetos próprios, RESPECTCASE,
DNA ordenado, RNA, proteínas e diretivas NEXUS não suportadas são rejeitados.
`MISSING=N` declara N como ausente. Valem as mesmas regras de linhas
sequenciais ou intercaladas já suportadas para morfologia.

JC69 fixa frequências em 1/4 e taxas de troca iguais, normalizadas para uma
substituição esperada por sítio: `Pii(t)=1/4+3/4 exp(−4t/3)` e
`Pij(t)=1/4−1/4 exp(−4t/3)` entre bases diferentes. Os controles existentes
servem para relógios estrito e relaxado lognormal, categorias gama, raiz e
priors de árvore suportados. Não há correção morfológica de seleção de
caracteres. HKY e GTR não estão disponíveis. A análise combinada tem um fluxo próprio.

Configurações de formato 11, Histórico, registros e checkpoints preservam
DNA/JC69 e o tratamento de gaps. Apps antigos rejeitam essas configurações.
É possível importar execuções CLI no Histórico; modelos incompatíveis não
são agrupados para avaliar convergência. Árvores, diagnósticos e estimativas
marginais mantêm o modelo ajustado. Testes curtos verificam integração, não
convergência posterior. A predição aceita bases individuais e células
ausentes; ambiguidades IUPAC precisam de modelo de observação e são recusadas
na simulação. Réplicas saem em NEXUS DNA. A exportação reprodutível mantém os
bytes originais em `matrix.fasta` ou `matrix.nex`. A inclusão continua explícita
e também controla cópias originais guardadas pelas verificações preditivas.

As fixtures sintéticas em `fixtures/validation/dna` validam a soma independente
dos estados internos pela fórmula de JC69, seis estados fixos do BEAST.base
2.7.8, frequências simuladas e retomada exata. Com as mesmas taxas de categoria,
as diferenças absolutas de log-verossimilhança são menores que 1e-10. Com
quantis gama calculados separadamente, a tolerância é 1e-7; a aproximação AS91
do BEAST difere menos de 3e-8 nas taxas desses estados. É validação numérica,
sem comparação de velocidade.
