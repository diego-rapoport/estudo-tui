# Tempo de estudo: medir, comparar, reajustar

O aluno combina uma disponibilidade no início ("30 min por semana") e a trilha é estimada
em cima dela. Se a estimativa não bate com a realidade, ele chega à sessão 6 sentindo que
"está atrasado", e isso derruba a motivação mais rápido do que qualquer conteúdo difícil.
Medir o tempo de cada etapa permite trocar essa sensação por números e ajustar o plano
cedo, sem drama.

## O que fica onde

| Arquivo | Quem escreve | O que tem |
|---|---|---|
| `_perfil.md` (frontmatter) | professor | `meta_semanal_min`, `meta_sessao_min`, `estimativa_min_conceito` (opcional; o padrão é a meta da sessão) |
| `_sessoes.md` | `arvore.py sessao` | uma seção por sessão: plano e tabela de etapas (início, fim, min, plano) |
| `_progresso.md` | `arvore.py` (gerado, nunca edite à mão) | resumo, gráficos mermaid por semana, por sessão e por etapa, e a seção **Por tópico** |

A coluna `min` de `_sessoes.md` pode ser corrigida à mão (pausa, sessão que ficou sem
`fim`). Os totais sempre usam esse valor.

## Durante a sessão

O script usa o relógio do sistema. Chame-o nos momentos de transição, que você já percebe
pela conversa:

```bash
python3 <skill>/scripts/arvore.py sessao <pasta> inicio --plano "revisao=5,pre-teste=3,explicacao=5,autoexplicacao=4,pratica=10,nota=3"
python3 <skill>/scripts/arvore.py sessao <pasta> etapa revisao --conceito exponencial-e-logaritmo
python3 <skill>/scripts/arvore.py sessao <pasta> etapa pre-teste --conceito taxa-media-de-variacao
python3 <skill>/scripts/arvore.py sessao <pasta> etapa pratica      # herda o conceito da etapa anterior
python3 <skill>/scripts/arvore.py sessao <pasta> fim
```

- **Nomes de etapa**: use sempre os mesmos, para as médias fazerem sentido: `revisao`,
  `pre-teste`, `explicacao`, `autoexplicacao`, `pratica`, `nota`; fora da sessão de
  estudo, use `diagnostico`, `exercicios`, `correcao`, `analise`. O tempo de `revisao` não
  conta no "tempo por conceito novo", porque revisão é manutenção.
- **Plano**: monte-o no `inicio` a partir do que vai acontecer hoje (sem revisões
  vencidas, sem etapa `revisao`) e da meta da sessão. O `inicio` mostra as **médias
  reais por etapa** das últimas sessões. Use-as: se a prática leva 20 min em média,
  planejar 10 é se enganar. Sem `--plano`, o script divide a meta da sessão em
  proporções padrão.
- **Chame `etapa` no momento em que a etapa começa**, de preferência na mesma resposta em
  que você faz a primeira pergunta dela. Uma chamada atrasada joga o tempo na etapa
  anterior.
- **`fim`** quando a sessão fecha (o resumo final, junto com o diário). Se o aluno sumir
  sem fechar, a próxima chamada encerra a sessão antiga sozinha e avisa. Nesse caso, a
  última etapa entra pelo tempo planejado.
- **Pausas**: se o script avisar que uma etapa durou muito, pergunte ao aluno, em uma
  linha, se ele parou no meio, e corrija a coluna `min`. Se você já sabe da pausa ("volto
  em 10 min"), passe `--descontar 10` na chamada que fecha a etapa.
- Não narre o cronômetro para o aluno ("iniciando a etapa 3..."). O registro é invisível;
  o que ele vê é o resultado, quando valer a pena.

## Lendo o resultado

`sessao fim` imprime duas partes, e `progresso` imprime só a segunda:

1. **Sessão**: total × plano, etapa a etapa. "dentro do plano → não comentar" vale
   quando a diferença fica em até 20% ou 5 min. Mesmo assim, a linha "só para o próximo
   plano" lista as etapas que estouraram: use-as para planejar a próxima sessão, sem
   falar disso.
2. **Expectativa**: semana atual, média das últimas 4 semanas fechadas, ritmo da trilha e
   um **veredito**:

| Veredito | Quando | O que fazer |
|---|---|---|
| `NÃO COMENTAR` | diferença pequena, ou menos de 2 semanas de dados | nada. Não mencione tempo nem meta. |
| `PARABENIZAR` | semana atual ou média acima da meta em mais de 20% | parabenize, sem aumentar a meta (ver abaixo) |
| `CONVERSAR` | média abaixo da meta em mais de 20% (2+ semanas), ou cada conceito leva bem mais que o estimado (3+ conceitos) | proponha um reajuste e deixe o aluno escolher |
| `SEM META` | o `_perfil.md` não tem `meta_semanal_min` | grave a meta a partir de "Disponibilidade" e confirme com o aluno |

Por que o silêncio quando a diferença é pequena? Semanas de estudo são irregulares, e
comentar cada desvio de 3 minutos transforma o professor num fiscal de ponto. A conversa
sobre expectativa só ajuda quando muda alguma decisão.

## Parabenizar sem subir a meta

Quando o aluno estuda mais que o combinado, o tempo extra é **bônus**, não o novo
normal. Se a meta sobe toda vez que ele passa dela, o esforço extra vira punição: a
próxima semana normal parece um fracasso. Por isso:

- Elogie de forma específica e uma vez por semana: o que o tempo extra rendeu ("com os
  45 min desta semana você fechou taxa média e ainda adiantou a prática da derivada").
  Para não repetir o elogio em toda sessão da mesma semana, confira o diário.
- Não altere `meta_semanal_min` nem a estimativa da trilha por causa disso. A previsão da
  trilha em `_progresso.md` já mostra o efeito (menos semanas), e isso já é recompensa.
- Só mude a meta se o próprio aluno pedir ("agora consigo 1 h por semana").

## Conversar sobre reajuste

Traga os números em uma ou duas linhas e ofereça 2 ou 3 caminhos concretos. Não culpe o
aluno: a estimativa é que estava errada.

> Nas últimas 4 semanas a média ficou em 18 min (a meta era 30), e cada conceito tem
> levado uns 40 min. Nesse ritmo, a trilha leva ~25 semanas, não 12. Quer (a) manter tudo
> e aceitar o prazo maior, (b) encurtar a trilha tirando o módulo 3, ou (c) sessões de
> 20 min só com prática?

Depois da escolha, atualize o `_perfil.md` (metas e "Disponibilidade") e o `_mapa.md`, se
for o caso, e registre a decisão no diário.

## No diário

Inclua o tempo na entrada da sessão: `**2026-11-01 (sessão 3, 34 min / plano 30)**: ...`.
Se houve parabéns ou reajuste, diga em meia linha. É assim que a próxima sessão sabe que o
elogio da semana já foi feito.
