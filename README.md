# estudando

Painel no terminal (ratatui) para a árvore de conhecimento da skill **professor**:
progresso por conceito, revisões espaçadas, pendências e o que estudar a seguir.

Só lê as notas. Quem escreve é a skill professor (ou você, pelo editor).

## Instalar

```sh
cargo install --path . --locked -j4
```

O binário vai para `~/.cargo/bin/estudando`. Depois de mudar o código, rode o mesmo comando
de novo.

## Usar

```sh
estudando            # abre o primeiro assunto
estudando calculo    # abre direto num assunto (nome da pasta ou prefixo)
estudando --dir ~/outra/pasta
```

A pasta raiz padrão é `~/Documentos/Aprendizado`. Para mudar, use `$ESTUDANDO_DIR` ou `--dir`.

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
