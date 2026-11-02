# estudando

Duas peças que trabalham juntas:

- **`skill/professor/`**: skill do Claude Code que faz papel de professor particular. Ela
  diagnostica o que você sabe, monta uma trilha, escreve notas em Markdown (compatíveis com
  o Obsidian), passa exercícios com repetição espaçada e dispara pesquisas.
- **`estudando`**: painel no terminal (Rust + ratatui) que lê essas notas e mostra progresso,
  revisões, exercícios, pendências e o que estudar a seguir. Ele só lê; quem escreve é a
  skill (ou você, pelo editor).

![Aba Trilha do estudando: módulos do mapa com nível e situação da revisão de cada conceito, painel lateral com o resumo do dia e o progresso, e o quadro "O que estudar agora"](docs/screenshots/trilha.png)

## Instalar

Requer Rust (cargo) e, para a skill, Python 3 e o Claude Code. A sessão com painéis
(`estudando --sessao`) usa o tmux.

```sh
./instalar.sh              # TUI + skill + pergunta onde guardar os estudos
./instalar.sh --sem-skill  # só a TUI
```

O script:

1. roda `cargo install --path . --locked` (binário em `~/.cargo/bin/estudando`);
2. cria o link `~/.claude/skills/professor -> <repo>/skill/professor`, para que editar a
   skill no repositório já valha no Claude Code;
3. pergunta onde guardar os estudos, se isso ainda não estiver configurado.

Depois de mudar o código, rode `./instalar.sh` de novo.

## Onde ficam os estudos

A skill e a TUI usam a mesma pasta base, resolvida nesta ordem:

1. `$ESTUDANDO_DIR`, se estiver definida;
2. `pasta = "..."` em `~/.config/estudando/config.toml` (respeita `$XDG_CONFIG_HOME`).

Se nenhuma das duas existir, nada é criado às cegas. A skill pergunta onde guardar na
primeira sessão e grava a resposta. A TUI mostra como configurar. A sugestão é
`<pasta de documentos do sistema>/Aprendizado`, que vira `~/Documentos/Aprendizado` ou
`~/Documents/Aprendizado` conforme o idioma.

```sh
estudando --pasta                         # qual pasta está em uso e de onde veio
estudando --definir-pasta ~/Estudos       # grava (e cria) a pasta
python3 skill/professor/scripts/arvore.py base [--definir CAMINHO]   # o mesmo, pelo lado da skill
```

Dentro da pasta base fica uma pasta por assunto (`calculo/`, com `_mapa.md`, `_perfil.md`,
`conceitos/`, `exercicios/`, `pesquisa/`). Ela pode ser aberta como vault do Obsidian.

O professor cronometra cada sessão, etapa a etapa, em `_sessoes.md` e gera
`_progresso.md`, com gráficos mermaid do tempo por semana, por sessão, por etapa e por
tópico, comparados com a meta do `_perfil.md`. Abra no Obsidian para ver os gráficos.

## Usar

```sh
estudando            # abre o primeiro assunto
estudando calculo    # abre direto num assunto (nome da pasta ou prefixo)
estudando --dir ~/outra/pasta   # outra pasta só nesta execução
```

Para começar um assunto, peça ao Claude Code: "quero aprender cálculo". A skill professor
assume a partir daí.

| Aba | O que mostra |
|---|---|
| 1 Trilha | o `_mapa.md` na ordem, com nível e situação da revisão de cada conceito; detalhes do selecionado; "O que estudar agora" |
| 2 Revisões | todos os conceitos ordenados por urgência (vencidas, hoje, futuras, sem data) e o histórico de revisões |
| 3 Exercícios | as listas de `exercicios/`, com status, questões respondidas no arquivo e conceitos cobertos |
| 4 Pendências | revisões vencidas, listas pendentes, equívocos do diagnóstico, perguntas do professor, dúvidas abertas, base frágil, seções suas em branco |
| 5 Pesquisa | os relatórios de `pesquisa/` (pesquisador e atualizações), com as seções de cada um |
| 6 Perfil | o `_perfil.md` (objetivo, diagnóstico, diário) |
| 7 Referências | o `_referencias.md` |

Atalhos: `j/k` mover, `Enter` abrir conceito, lista ou relatório, `o` abrir o arquivo no
`$VISUAL`/`$EDITOR`, `Tab`/`1-7` abas, `[`/`]` assunto, `r` recarregar, `?` ajuda, `q` sair.
As notas são recarregadas sozinhas quando mudam no disco.

Numa lista de exercícios aberta, dicas e gabarito (callouts recolhidos `> [!tip]-` e
`> [!check]-`) ficam **ocultos**: `d` mostra as dicas e `g` mostra o gabarito. Uma questão
conta como respondida quando há texto depois de `**Sua resposta:**`. Responda pelo `o`, no
editor, ou no chat com o professor.

## Sessão: professor e caderno ao lado

```sh
estudando --sessao calculo                      # TUI + professor (claude) + caderno (python3)
estudando -s --ia gemini --caderno qalc         # outros programas só nesta vez
estudando -s --caderno "nvim rascunho.md"       # caderno de texto em vez de REPL
estudando -s --ia ""                            # sem professor (só TUI e caderno)
estudando --painel caderno                      # só o caderno; o professor abre com i
```

Não precisa decidir antes: no `estudando` normal, apertar `i`, `c` ou `p` leva a TUI para
uma sessão no mesmo assunto e aba, já com o painel pedido.

Abre tudo no tmux, visível ao mesmo tempo: a TUI à esquerda e, à direita, o **professor**
em cima (uma IA de terminal; com o `claude`, a skill professor) e o **caderno** embaixo
(qualquer programa de terminal: REPL, calculadora, editor). Os dois rodam na pasta de
estudos, com `$ESTUDANDO_DIR` e `$ESTUDANDO_ASSUNTO` definidas, e o que o professor gravar
aparece na TUI na hora.

| Tecla (na TUI) | O que faz |
|---|---|
| `p` | começa uma pergunta ao professor sobre o conceito, lista ou relatório em foco (com o caminho da nota) e leva você até lá para completar |
| `i` / `c` | vai para o professor / caderno; se o painel não existe ou o programa foi encerrado, abre |
| `b` | recolhe a TUI numa faixa estreita (com o número de revisões pendentes) para o professor e o caderno ocuparem a largura; `b` de novo ou `Alt+b` de qualquer painel expande |
| `q` | sai da TUI e fecha os dois painéis |

Para trocar de painel: `Alt`+setas ou clique. `Alt+z` amplia o painel atual (de novo para
voltar), e `Ctrl-b d` sai sem fechar; rodar `estudando -s` outra vez volta para a mesma
sessão, com a conversa intacta. Arrastar a borda redimensiona.

Os comandos e os tamanhos ficam no config (`""` desliga o painel):

```toml
pasta = "~/Documentos/Aprendizado"
ia = "claude"          # ou "gemini", "codex"...
caderno = "python3"    # ou "node", "qalc", "nvim rascunho.md"...
largura_tui = 55       # % da largura para a TUI
altura_ia = 60         # proporção professor/caderno na coluna da direita
altura_caderno = 40
```

Fora de um tmux, a sessão roda num servidor tmux próprio (`tmux -L estudando`), com mouse,
rótulos nos painéis e os atalhos acima, sem mexer no seu `~/.tmux.conf`. Se você já está
num tmux, ela divide a janela atual e usa os seus atalhos (`Ctrl-b` setas; para expandir a
TUI recolhida, vá até ela e aperte `b`).

## "O que estudar agora"

Em ordem de prioridade:

1. **revisar**: conceitos com `proxima_revisao` vencida ou para hoje;
2. **exercícios**: listas sem `status: corrigido`, da mais antiga para a mais nova;
3. **combinado**: o "Próximo:" da entrada mais recente do diário no `_perfil.md`;
4. **reforçar**: conceitos da trilha com nota e nível abaixo de 3;
5. **novo**: o primeiro conceito da trilha principal (módulos não opcionais) que ainda não
   tem nota, avisando se algum anterior está abaixo do nível 2;
6. **agendar**: conceitos sem `proxima_revisao`, fora do ciclo de revisão.

## Formato esperado

O formato é o descrito em `~/.claude/skills/professor/references/arvore-conhecimento.md`.
As regras de leitura (frontmatter, pastas ignoradas, "base frágil") são as mesmas do
`arvore.py` da skill.

## Licença

MIT. Veja [LICENSE](LICENSE).
