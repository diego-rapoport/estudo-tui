# Exercícios e revisão

## Seleção

1. `arvore.py revisoes <pasta>`: os conceitos vencidos entram primeiro.
2. Complete com o conceito atual da trilha e com um ou dois anteriores relacionados
   (intercalação).
3. Uma lista típica tem de 5 a 8 questões, ou cerca de 20–30 minutos. Para revisão rápida,
   de 3 a 4 questões. Mais do que isso desanima e quebra o hábito.

## Tipos de questão

Varie os tipos dentro da lista. Cada tipo testa uma coisa diferente:

| Tipo | Testa | Exemplo |
|---|---|---|
| **Lembrar** (sem consulta) | recuperação da definição ou da ideia | "Sem olhar nada: o que significa lim x→a f(x) = L?" |
| **Aplicar** | procedimento | "Derive f(x) = sen(x²)." |
| **Explicar o porquê** | compreensão | "Por que a regra da cadeia multiplica as derivadas em vez de somar?" |
| **Achar o erro** | equívocos e atenção | "Um aluno escreveu d/dx(eˣ²) = eˣ². Onde está o erro?" |
| **Prever / estimar** | intuição | "Sem calcular, a derivada de f em x = 2 é positiva ou negativa? Como você sabe pelo gráfico?" |
| **Transferir** | uso em contexto novo | problema aplicado de outra área (física, economia, biologia) |
| **Escolher a ferramenta** | discriminação (intercalação) | problemas misturados sem dizer qual técnica usar |

Regras:

- Pelo menos metade das questões precisa exigir explicação ou raciocínio, não só a resposta
  final.
- Ordem: de mais fácil a mais difícil dentro da lista, com os conceitos misturados.
- Inclua pelo menos uma questão de "achar o erro" com um equívoco que ele já teve (se o
  `_perfil.md` ou as notas registrarem algum): é a melhor forma de confirmar que foi
  superado.
- Dificuldade calibrada pelo nível: nível 1–2 → lembrar e aplicar, com uma questão de
  porquê; nível 3 → porquê, achar o erro e transferir; nível 4 → transferir e problemas
  abertos.
- Uma questão de **transferir** só entra quando os conceitos de que ela depende estão no
  nível 3. Problemas com muitas grandezas (limite, taxa, ciclo, piso...) são ótimos, mas
  para quem está no nível 2 eles viram uma sequência de erros diferentes. Nesse caso,
  quebre a questão em partes de 1 ou 2 elementos (teste de carga em
  `references/curva-de-aprendizado.md`).
- **Gabarito conferido por dois caminhos** (fórmula e tabela, ou fórmula e uma simulação
  curta) e com as unidades checadas. Errar o gabarito ensina o erro.
- **Números que discriminam:** escolha valores em que os erros comuns dão resultados
  diferentes do certo. Evite taxa 1, zeros e coincidências (√16 = log₂16, 1000 − 500 = 500):
  uma conta errada que acerta por acaso vira um equívoco confiante.
- Exercício pendente de outra sessão: antes de retomá-lo, confira se ele ainda cabe no
  nível atual do aluno.

## Arquivo: `exercicios/AAAA-MM-DD-<tema>.md`

```markdown
---
tipo: exercicios
assunto: calculo
data: 2026-10-29
conceitos: ["[[limite]]", "[[derivada]]", "[[regra-da-cadeia]]"]
status: pendente
---
# Exercícios — 29/10/2026

Responda sem consultar notas ou livros. "Não sei" vale: escreva o que você lembra e onde
travou. Abra as dicas só depois de tentar.

## 1. Lembrar · [[limite]]
Explique com suas palavras o que significa $\lim_{x \to 2} f(x) = 5$. O valor de $f(2)$
precisa ser 5?

> [!tip]- Dica
> Pense numa função com um "buraco" no gráfico em x = 2.

> [!check]- Gabarito
> Significa que f(x) fica tão perto de 5 quanto quisermos quando x está suficientemente
> perto de 2 (sem ser 2). f(2) pode ser outro valor, ou nem existir: o limite fala da
> vizinhança, não do ponto.

**Sua resposta:**


## 2. ...
```

- As dicas e o gabarito ficam em callouts recolhidos (`> [!tip]-`, `> [!check]-`). No
  Obsidian eles aparecem fechados; o aluno abre se quiser.
- O aluno pode responder no arquivo (em "Sua resposta") ou no chat. Os dois funcionam.
- No chat, ao apresentar a lista, mostre só os enunciados (e diga onde está o arquivo).
  Nunca mostre o gabarito no chat antes de ele responder.

## Correção

Para cada questão, no máximo 2–3 linhas:

- ✅ / 🟡 / ❌ e o que está certo **no raciocínio**, não só no resultado.
- O que falta ou o erro, e **por quê** (a regra ou a intuição por trás).
- Se errou: um mini-problema parecido para refazer na hora, ou deixe para a próxima lista.
- Se ele resolver no chat com você, classifique cada resposta (`arvore.py resposta`) e olhe
  a janela das últimas 3 a 5. Erros de tipos diferentes em sequência indicam sobrecarga:
  pare a lista e reduza o passo em vez de seguir para a próxima questão.

Depois da correção:

1. Acrescente no arquivo uma seção `## Correção (data)` com o resumo e mude
   `status: corrigido`.
2. Para cada conceito avaliado, rode
   `arvore.py registrar <pasta> <conceito> acertou|parcial|errou [--nivel N]`.
   - `acertou`: correto com o raciocínio certo, sem ajuda.
   - `parcial`: ideia certa com erro de execução, ou acerto só depois de uma dica.
   - `errou`: errado, ou não sabia.
   - Use `--nivel` quando a evidência mudar o nível (por exemplo, acertou um problema de
     transferência → 4; errou de novo a ideia central → baixa).
3. Se aparecer um equívoco novo, registre na nota (em "Observações do professor") e no
   diário do `_perfil.md`.
4. Termine com uma linha de resumo: o que está sólido, o que volta na próxima revisão e
   quando é a próxima revisão.

## Manter-se atualizado

"Manter atualizado" tem dois sentidos, e os dois importam:

- **Não esquecer**: as revisões espaçadas. Ao retomar, sempre verifique o que está vencido.
- **Novidades da área**: para assuntos que mudam (tecnologia, ciência ativa), sugira de vez
  em quando rodar o agente de atualizações e transforme o que for relevante em notas ou
  exercícios. Para assuntos estáveis (cálculo clássico), novidades são raras; o que muda são
  os materiais (novos cursos, novas edições).
