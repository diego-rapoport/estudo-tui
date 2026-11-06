# Curva de aprendizado: ler o aluno enquanto ele responde

O resto da skill diz *o que* ensinar e em que ordem. Este arquivo diz como perceber, resposta
a resposta, se o ritmo está certo para *este* aluno, e o que fazer quando não está. O erro
mais caro de um tutor não é uma explicação ruim: é continuar no mesmo ritmo depois que os
sinais de sobrecarga já apareceram. O aluno não diz "estou sobrecarregado"; ele erra coisas
diferentes em sequência, perde o fio, defende uma resposta errada e, por fim, desiste da
sessão.

A base: apresentar material novo em passos pequenos e checar a compreensão com frequência,
mirando cerca de 80% de acerto na prática guiada (Rosenshine, 2012, *Principles of
Instruction*, American Educator); ajustar a ajuda de forma contingente, mais apoio quando ele
falha e menos quando acerta (Wood, Bruner & Ross, 1976, a origem do termo *scaffolding*); e
lembrar que o que pesa num problema é quantos elementos precisam ser mantidos ao mesmo tempo
na cabeça (interatividade de elementos, Sweller, 2010).

## 1. Antes de propor um problema: teste de carga

Conte quantos **elementos** o aluno precisa segurar ao mesmo tempo: grandezas diferentes
(limite, taxa, piso, ciclo...), passos encadeados e relações novas. Depois compare com o
nível dele *em cada um* desses elementos:

| Nível do aluno no conceito | Elementos novos por problema | Formato |
|---|---|---|
| 0–1 | 1 | exemplo resolvido completo, depois um quase igual |
| 2 | até 2 | estrutura já vista num exemplo; muda uma coisa por vez |
| 3 | 3–4, misturando conceitos | problema típico, sem andaime |
| 4 | aberto | transferência, problemas novos |

Um problema de **transferência** (contexto novo, várias grandezas) só faz sentido quando os
conceitos de que ele depende estão no nível 3. Se o problema bom for grande demais, quebre-o:
cada parte vira um problema com 1 ou 2 elementos, e o grande fica para o fim ou para outra
sessão. Isso vale também para exercícios que ficaram pendentes de sessões anteriores: o
nível do aluno pode não ser o que você imaginava quando os escreveu, então refaça o teste
de carga antes de retomar.

**Confira a sua solução antes de propor o problema.** Resolva por dois caminhos (fórmula e
tabela, ou fórmula e simulação de poucos passos) e confira as unidades. Se a conta for
trabalhosa, rode um script curto. Um gabarito errado custa caro: o aluno perde confiança
justamente quando está inseguro.

**Escolha números que discriminam.** Evite valores em que métodos errados dão a resposta
certa por acaso: taxa 1 (dividir ou não dá no mesmo), 0, números iguais, simetrias como
1000 − 500 = 500. Com números bem escolhidos, cada erro comum dá um resultado diferente, e
a resposta dele já diz qual foi o erro. Quando o aluno generaliza (uma fórmula, uma
função), teste com valores que quebram essas coincidências: uma fórmula errada que "passa"
num caso especial vira um equívoco confiante.

## 2. A cada resposta: classificar

Em silêncio, classifique cada resposta a uma pergunta ou problema e registre (seção 5):

| Código | Quando |
|---|---|
| `sozinho` | certo, sem ajuda |
| `dica` | certo depois de uma ou mais dicas |
| `execucao` | ideia certa, erro de conta, unidade, sinal ou leitura |
| `conceito` | ideia errada (equívoco, método errado) |
| `perdido` | não sabe por onde começar, perdeu o fio, "estou confuso", pede para você fazer |

Anote também os **sinais de estado**, que pesam tanto quanto o acerto: respostas cada vez
mais curtas, erro em algo que ele já dominava, contestar a explicação ("não entendi de onde
saiu esse número"), defender uma resposta errada com insistência, "cansei". Contestar a
explicação quase sempre quer dizer que faltou um passo nela. Responda com a derivação que
faltou e confira se outros números da conversa também ficaram sem origem.

Antes de cada problema importante, **preveja** o resultado com base no modelo do aluno
(seção 4): `sozinho`, `dica` ou trava. Registre a previsão junto com o resultado. Quando
ela erra, isso é informação: ou você superestimou o domínio (o conceito isolado está no
nível 3, mas não combinado com outros), ou subestimou (ele pode pular etapas). É assim que
você aprende com o aluno.

## 3. Ler a janela e decidir o ritmo

Olhe as últimas 3 a 5 respostas, não só a última:

| Padrão | O que significa | O que fazer |
|---|---|---|
| 3 ou mais `sozinho` seguidos, rápidos | o apoio sobrou | **Acelerar:** tire o andaime, pule o exemplo resolvido, suba um nível de dificuldade ou misture conceitos. Quem já domina aprende menos com apoio demais. |
| Maioria `sozinho`/`dica` (~70–85% de acerto) | zona certa | **Manter.** É a dificuldade desejável. |
| O mesmo tipo de erro duas vezes | equívoco ou lacuna específica | **Mirar:** pare o problema e trate só aquilo, com um caso de contraste. Mais problemas iguais não desmontam o erro. |
| Erros de tipos diferentes em sequência, ou um `perdido` | sobrecarga: elementos demais | **Desacelerar:** pare o problema atual. Volte para um subproblema com 1 elemento novo, mostre um exemplo resolvido completo e só então um quase igual. Não dê mais uma dica no problema grande, porque toda dica nova também é um elemento a mais. |
| Sinais de cansaço ou frustração | a sessão passou do ponto | **Fechar bem:** um último problema que ele resolve sozinho, um resumo de uma linha do que ele fez bem e o próximo passo. Insistir aqui ensina que estudar é ruim. |

Quando desacelerar, diga isso ao aluno em uma frase, sem drama ("vamos quebrar isso em
partes menores"). Quando acelerar, diga também, para ele saber que avançou.

**Dicas que reduzem a carga.** Depois de uma dica sem progresso, a próxima não deve ser mais
uma pista no mesmo problema, e sim um problema menor com a mesma estrutura: "esquece o GC
por um momento: a memória está em 300 e sobe 2 MB/min; quanto falta para 400 e em quanto
tempo?". Uma dica que só aponta para algo ("olhe a tabela") pressupõe que ele saiba onde
olhar e o que procurar.

## 4. Mensagens que não sobrecarregam

- **Cada mensagem se sustenta sozinha.** Repita os dados e a tabela de que ele precisa para
  responder. No terminal, o aluno não rola para cima; "a tabela que você fez" sem a tabela
  na tela é um elemento a mais para lembrar.
- **Todo número tem origem.** Não escreva um valor (um "~330") sem mostrar de onde ele saiu.
  Número sem origem vira uma pergunta extra para o aluno.
- **Uma ideia nova por mensagem.** Correção, explicação nova e próxima pergunta, cada uma na
  sua vez quando o aluno estiver no nível 2 ou abaixo.
- **Admita o seu erro na hora,** em uma linha, e corrija o arquivo. Esconder o erro custa
  mais confiança do que o próprio erro.

## 5. Registrar e acompanhar a curva

```bash
python3 <skill>/scripts/arvore.py resposta <pasta> sozinho|dica|execucao|conceito|perdido \
    [--conceito SLUG] [--previsto sozinho|dica|trava] [--sinal cansaco|frustracao|contestou|confiante] \
    [--nota "texto curto"]
python3 <skill>/scripts/arvore.py curva <pasta>    # resumo por sessão + veredito de ritmo
```

`resposta` grava uma linha em `_respostas.md`, na seção do dia. `curva` mostra, por sessão, a
taxa de acerto sozinho, a mistura de erros, os sinais e o acerto das suas previsões, com um
**veredito de ritmo** para a próxima sessão: `ACELERAR`, `MANTER`, `DESACELERAR` ou
`MIRAR <tipo de erro>`. O veredito resume os números, mas o julgamento é seu: um cansaço no
fim de um dia de trabalho não é o mesmo que um conceito que não entra.

Registrar é silencioso, como o cronômetro: não narre. A tabela no chat é a dele; o registro
é seu.

## 6. Modelo do aluno: "Como este aluno aprende"

No `_perfil.md`, a seção **Como este aluno aprende** guarda o que você aprendeu sobre ele:

- **Funciona:** formatos e exemplos que deram certo (ex.: exemplos de dev, tabelas,
  transformar a conta em código).
- **Trava:** o que sobrecarrega (ex.: mais de 2 grandezas, perder a tabela de vista).
- **Ritmo:** quantos problemas novos cabem numa sessão e quanto tempo até cansar.
- **Sinais dele:** como ele mostra que está perdido ou cansado (cada aluno tem os seus).
- **Erros meus a evitar:** o que você fez que atrapalhou.

Leia essa seção ao retomar e use-a para planejar a sessão e prever os resultados. No fim de
cada sessão, atualize-a com o que a evidência mostrou: corrija o que estava errado e não só
acrescente. Mantenha a seção curta (no máximo 10 linhas), porque ela é uma ferramenta de
decisão, não um diário.
