# Árvore de conhecimento: estrutura e templates

A árvore é um conjunto de notas Markdown, uma por conceito, ligadas por `[[links]]`. Abre
direto no Obsidian (a pasta `Aprendizado` pode virar um vault, e o Graph View mostra a
árvore), mas é Markdown puro, legível em qualquer editor.

## Nomes de arquivos

- Pasta do assunto e arquivos de conceito: minúsculas, sem acento, com hífens
  (`regra-da-cadeia.md`, `funcao-composta.md`). O título com acento fica no `# Título`.
- Os links usam o nome do arquivo: `[[regra-da-cadeia]]`. Para mostrar outro texto:
  `[[regra-da-cadeia|regra da cadeia]]`.
- Arquivos do assunto começam com `_` (`_perfil.md`, `_mapa.md`, `_referencias.md`) para
  ficarem no topo da listagem.

## Nota de conceito: `conceitos/<slug>.md`

```markdown
---
tipo: conceito
assunto: calculo
nivel: 1
prerequisitos: ["[[funcao]]", "[[taxa-de-variacao]]"]
intervalo: 1
proxima_revisao: 2026-10-30
ultima_revisao:
criado: 2026-10-29
---
# Limite

## Em minhas palavras
<!-- Escreva você, como se explicasse para um colega que nunca ouviu falar. -->

## Exemplo que eu entendo
<!-- Um exemplo concreto, de preferência inventado por você. -->

## Por que importa
<!-- Para que serve? O que fica possível com isso? -->

## Conexões
- Depende de: [[funcao]], [[taxa-de-variacao]]
- Leva a: [[continuidade]], [[derivada]]

## Dúvidas abertas

## Equívocos que já corrigi

## Observações do professor

## Revisões
```

Regras:

- **Frontmatter**: chaves simples (`chave: valor`). Os pré-requisitos ficam numa lista de
  links entre aspas (o formato de *properties* do Obsidian). Se o Obsidian reescrever a
  lista em várias linhas (`- "[[funcao]]"`), está tudo bem: o script entende os dois
  formatos.
- `nivel` segue a escala 0–4 do SKILL.md. Atualize sempre com base em evidência (resposta,
  exercício, nota escrita), nunca porque "já estudamos isso".
- `intervalo` e `proxima_revisao` são gerenciados por `arvore.py registrar`. Na criação,
  depois da primeira sessão sobre o conceito: `intervalo: 1`, próxima revisão no dia
  seguinte. Para conceitos que só foram diagnosticados (nível 3+ no diagnóstico), comece
  com `intervalo: 7`.
- As seções "Em minhas palavras", "Exemplo que eu entendo", "Por que importa" e "Dúvidas
  abertas" são **do aluno**. Você só escreve em "Observações do professor" (com data),
  "Conexões" (sugestões, quando ele concordar) e "Equívocos que já corrigi" (quando um
  equívoco for desmontado na sessão, com as palavras dele, se possível).
- "Revisões" é preenchida pelo script.

## `_perfil.md`

```markdown
---
tipo: perfil
assunto: calculo
criado: 2026-10-29
meta_semanal_min: 60
meta_sessao_min: 30
---
# Perfil — Cálculo

## Objetivo
Para quê, prazo (se houver), como ele vai saber que chegou lá.

## Profundidade escolhida
Ex.: "cálculo universitário padrão (Cálculo I), com ênfase em intuição". Registre também
as opções que foram oferecidas, para ele poder mudar.

## Disponibilidade
Horas por semana; preferências (sessões curtas, gosta de exemplos visuais...).

## Diagnóstico (2026-10-29)
| Conceito | Nível | Evidência | Tipo |
|---|---|---|---|

Equívocos encontrados: ...

## Diário
- **2026-10-29**: diagnóstico; ponto de partida = funções (nível 2). Próximo: composição de funções.
```

O diário é a memória entre sessões: uma ou duas linhas por sessão, sempre com o "Próximo"
e os minutos da sessão (real/plano).

As metas do frontmatter (`meta_semanal_min`, `meta_sessao_min` e, opcional,
`estimativa_min_conceito`) são o que `arvore.py sessao`/`progresso` usam para comparar o
tempo real com o combinado. Mude-as só quando o aluno decidir, nunca porque ele estudou a
mais (ver `references/tempo.md`).

## `_sessoes.md` e `_progresso.md`

Os dois são escritos pelo script (`arvore.py sessao`, `arvore.py progresso`), não à mão:

- `_sessoes.md`: uma seção `## AAAA-MM-DD HH:MM` por sessão, com uma linha `plano:` e uma
  tabela `| etapa | conceito | início | fim | min | plano |`. A coluna `min` pode ser
  corrigida à mão (pausas), e os totais usam esse valor.
- `_progresso.md`: gerado de novo a cada `sessao fim`. Traz um resumo, gráficos mermaid
  (`xychart-beta` por semana e por sessão, `pie` por etapa) e a seção **Por tópico**, com
  um gráfico de minutos por conceito e um callout recolhido por conceito.

Os links desses dois arquivos não contam no `relatorio` (órfãs, citados sem nota).

## `_mapa.md`

```markdown
---
tipo: mapa
assunto: calculo
---
# Mapa — Cálculo

Objetivo: ... · Profundidade: ...

## Módulo 0 — Base que falta
1. [[funcao]]
2. [[funcao-composta]]
3. [[exponencial-e-logaritmo]]

## Módulo 1 — Limites e continuidade
4. [[limite]]
5. [[limites-laterais]]
6. [[continuidade]]

## Módulo 2 — Derivada
...
```

O mapa guarda **a ordem**, não o status: o nível de cada conceito fica na nota (veja com
`arvore.py relatorio`). Assim não há duas fontes de verdade. Conceitos de módulos futuros
podem aparecer como links para notas que ainda não existem; o relatório os lista como
"citados sem nota", e isso é esperado.

## `_referencias.md`

```markdown
# Referências — Cálculo

## 1. Primeiro contato
- **Essence of Calculus** — 3Blue1Brown (vídeos, EN, legendas PT) · episódios 1–4
  Por quê: intuição visual de derivada e integral antes de qualquer fórmula.

## 2. Base (texto principal)
- ...

## 3. Aprofundamento
- ...

## 4. Referência densa
- ...

## Prática
- Listas e fontes de exercícios.

_Verificado em 2026-10-29. Itens marcados com (não verificado) precisam de confirmação._
```

Para cada item: título, autor, formato, idioma, **o que ler** (capítulos ou seções) e
**por quê**. Links só se tiverem sido verificados.

## Analisando a árvore: o que procurar

| Sinal | Como detectar | O que fazer |
|---|---|---|
| Equívoco | ler o texto do aluno | prioridade 1; pergunta que exponha a contradição |
| Base frágil | `relatorio` → "base frágil" | revisar o pré-requisito antes de seguir |
| Nota vazia ou só com a definição copiada | `relatorio` + leitura | pedir que ele explique com as próprias palavras; exercício de "explique" |
| Nível maior do que a nota justifica | leitura (nível 3 com explicação vaga) | exercício de verificação; ajustar o nível conforme o resultado |
| Conexão faltando | conceitos relacionados sem link entre si | perguntar a relação ("como derivada e limite se ligam?") e deixar ele escrever |
| Órfã | `relatorio` → "órfãs" | ligar ao mapa ou ao conceito pai |
| Citado sem nota | `relatorio` → "citados sem nota" | normal se for de um módulo futuro; lacuna se for pré-requisito de algo já estudado |
| Revisão vencida | `revisoes` | exercícios de revisão |
