#!/usr/bin/env python3
"""Ferramentas da árvore de conhecimento da skill professor (só biblioteca padrão).

Uso:
  arvore.py base [--definir CAMINHO]   # pasta base dos estudos (compartilhada com a TUI `estudando`)
  arvore.py relatorio <pasta-do-assunto> [--data AAAA-MM-DD]
  arvore.py revisoes  <pasta-do-assunto> [--data AAAA-MM-DD] [--dias N]
  arvore.py registrar <pasta-do-assunto> <conceito> acertou|parcial|errou [--nivel N] [--data AAAA-MM-DD]
  arvore.py sessao    <pasta-do-assunto> inicio [--plano "revisao=5,pratica=15"]
  arvore.py sessao    <pasta-do-assunto> etapa <nome> [--conceito SLUG] [--descontar MIN]
  arvore.py sessao    <pasta-do-assunto> fim [--descontar MIN]
  arvore.py progresso <pasta-do-assunto>   # refaz _progresso.md e diz se a expectativa precisa de conversa
  arvore.py resposta  <pasta-do-assunto> sozinho|dica|execucao|conceito|perdido [--conceito SLUG]
                      [--previsto sozinho|dica|trava] [--sinal cansaco ...] [--nota TEXTO]
  arvore.py curva     <pasta-do-assunto> [--sessoes N]   # curva de aprendizado e veredito de ritmo
  (sessao e progresso aceitam --data AAAA-MM-DD e --hora HH:MM)

Notas de conceito: arquivos .md com `tipo: conceito` no frontmatter ou dentro de conceitos/.
"""
import argparse
import datetime as dt
import math
import os
import re
import sys
import tempfile
from pathlib import Path

INTERVALOS = [1, 3, 7, 16, 35, 80, 180]
NIVEIS = {0: "não visto", 1: "reconhece", 2: "em construção", 3: "domina", 4: "transfere"}
LINK_RE = re.compile(r"\[\[([^\]|#]+)(?:[#|][^\]]*)?\]\]")
COMENTARIO_RE = re.compile(r"<!--.*?-->", re.S)
IGNORAR_DIRS = {".obsidian", ".trash", "pesquisa", "exercicios"}
SESSOES, PROGRESSO, RESPOSTAS = "_sessoes.md", "_progresso.md", "_respostas.md"
IGNORAR_ARQUIVOS = {SESSOES, PROGRESSO, RESPOSTAS}  # gerados; os links deles não contam na árvore

# Divisão padrão de uma sessão de estudo (% da meta da sessão), quando o plano não é dado.
PLANO_PADRAO = {"revisao": 15, "pre-teste": 10, "explicacao": 15, "autoexplicacao": 15, "pratica": 35, "nota": 10}
TOLERANCIA = 0.20  # diferença relativa abaixo da qual não vale falar de expectativa
FOLGA_MIN = 5  # ...e diferença absoluta (min) abaixo da qual também não
ABANDONO_MIN = 120  # etapa aberta há mais que isso: a sessão foi largada sem `fim`
HORA_RE = re.compile(r"^\d{1,2}:\d{2}$")
SESSAO_RE = re.compile(r"^##\s+(\d{4}-\d{2}-\d{2})\s+(\d{1,2}:\d{2})\s*$", re.M)


def ler_frontmatter(texto):
    """Devolve (meta, corpo). Entende `chave: valor`, listas inline e listas em bloco."""
    if not texto.startswith("---"):
        return {}, texto
    linhas = texto.split("\n")
    try:
        fim = next(i for i in range(1, len(linhas)) if linhas[i].strip() == "---")
    except StopIteration:
        return {}, texto
    meta, chave = {}, None
    for linha in linhas[1:fim]:
        if not linha.strip():
            continue
        if linha.lstrip().startswith("- ") and chave is not None:
            meta.setdefault(chave, [])
            if isinstance(meta[chave], list):
                meta[chave].append(linha.lstrip()[2:].strip().strip("\"'"))
            continue
        if ":" in linha and not linha.startswith((" ", "\t")):
            chave, valor = linha.split(":", 1)
            chave, valor = chave.strip(), valor.strip()
            if valor.startswith("[") and valor.endswith("]") and not valor.startswith("[["):
                itens = [v.strip().strip("\"'") for v in valor[1:-1].split(",")]
                meta[chave] = [v for v in itens if v]
            else:
                meta[chave] = valor.strip("\"'") if valor else []
    return meta, "\n".join(linhas[fim + 1:])


def alvos(valor):
    """Extrai slugs de links [[x]] de uma string ou lista."""
    if isinstance(valor, list):
        valor = " ".join(valor)
    return [normalizar(m) for m in LINK_RE.findall(valor or "")]


def normalizar(nome):
    return nome.strip().lower()


def data(valor):
    try:
        return dt.date.fromisoformat(str(valor)[:10])
    except (ValueError, TypeError):
        return None


def inteiro(valor):
    try:
        return int(str(valor).strip())
    except (ValueError, TypeError):
        return None


def secao(corpo, titulo):
    """Conteúdo de uma seção `## titulo`, sem comentários HTML."""
    m = re.search(rf"^##\s+{re.escape(titulo)}\s*$(.*?)(?=^##\s|\Z)", corpo, re.M | re.S | re.I)
    return COMENTARIO_RE.sub("", m.group(1)).strip() if m else None


def carregar(pasta):
    notas, todas = {}, {}
    for path in sorted(pasta.rglob("*.md")):
        rel = path.relative_to(pasta)
        if any(p in IGNORAR_DIRS for p in rel.parts[:-1]) or str(rel) in IGNORAR_ARQUIVOS:
            continue
        texto = path.read_text(encoding="utf-8")
        meta, corpo = ler_frontmatter(texto)
        slug = normalizar(path.stem)
        todas[slug] = {"path": path, "meta": meta, "corpo": corpo, "links": set(alvos(corpo)) | set(alvos(meta.get("prerequisitos")))}
        if meta.get("tipo") == "conceito" or rel.parts[0] == "conceitos":
            notas[slug] = todas[slug]
    return notas, todas


def cmd_relatorio(args):
    pasta = Path(args.pasta).expanduser()
    hoje = data(args.data) or dt.date.today()
    notas, todas = carregar(pasta)
    if not notas:
        print(f"Nenhuma nota de conceito em {pasta}")
        return
    print(f"# Relatório da árvore: {pasta.name} ({hoje})\n")
    print(f"Conceitos: {len(notas)}")
    contagem = {}
    sem_nivel = []
    for slug, n in notas.items():
        nv = inteiro(n["meta"].get("nivel"))
        if nv is None:
            sem_nivel.append(slug)
        else:
            contagem[nv] = contagem.get(nv, 0) + 1
    for nv in sorted(contagem):
        print(f"  nível {nv} ({NIVEIS.get(nv, '?')}): {contagem[nv]}")
    if sem_nivel:
        print(f"  sem nível: {', '.join(sem_nivel)}")

    print("\n## Conceitos")
    for slug, n in sorted(notas.items(), key=lambda kv: (inteiro(kv[1]["meta"].get("nivel")) or 0, kv[0])):
        m = n["meta"]
        prox = m.get("proxima_revisao") or "-"
        pre = ", ".join(alvos(m.get("prerequisitos"))) or "-"
        print(f"- {slug}: nível {m.get('nivel', '?')} · próx. revisão {prox} · pré-req.: {pre}")

    vencidas = [(s, data(n["meta"].get("proxima_revisao"))) for s, n in notas.items()]
    vencidas = sorted((d, s) for s, d in vencidas if d and d <= hoje)
    print(f"\n## Revisões vencidas ({len(vencidas)})")
    for d, s in vencidas:
        print(f"- {s} (desde {d}, {(hoje - d).days} dia(s) de atraso)")

    print("\n## Base frágil (pré-requisito com nível abaixo do conceito)")
    achou = False
    for slug, n in sorted(notas.items()):
        nv = inteiro(n["meta"].get("nivel"))
        if nv is None or nv < 2:
            continue
        for pre in alvos(n["meta"].get("prerequisitos")):
            pnv = inteiro(todas.get(pre, {}).get("meta", {}).get("nivel")) if pre in todas else None
            if pre not in todas:
                print(f"- {slug} (nível {nv}) depende de {pre}, que não tem nota")
                achou = True
            elif pnv is not None and pnv < min(nv, 3):
                print(f"- {slug} (nível {nv}) depende de {pre} (nível {pnv})")
                achou = True
    if not achou:
        print("- nenhuma")

    citados = {}
    for slug, n in todas.items():
        for alvo in n["links"]:
            if alvo not in todas:
                citados.setdefault(alvo, set()).add(slug)
    print(f"\n## Citados sem nota ({len(citados)})")
    for alvo, origens in sorted(citados.items()):
        print(f"- {alvo} ← {', '.join(sorted(origens))}")

    recebidos = {s: set() for s in todas}
    for slug, n in todas.items():
        for alvo in n["links"]:
            if alvo in recebidos and alvo != slug:
                recebidos[alvo].add(slug)
    orfas = [s for s, n in notas.items() if not recebidos[s] and not (n["links"] - {s})]
    print(f"\n## Órfãs (sem links de entrada nem de saída) ({len(orfas)})")
    for s in sorted(orfas):
        print(f"- {s}")
    sem_entrada = [s for s in notas if not recebidos[s] and s not in orfas]
    if sem_entrada:
        print(f"Sem links de entrada (nem no mapa): {', '.join(sorted(sem_entrada))}")

    print("\n## Notas sem 'Em minhas palavras' preenchido")
    vazias = [s for s, n in sorted(notas.items()) if not secao(n["corpo"], "Em minhas palavras")]
    print("\n".join(f"- {s}" for s in vazias) or "- nenhuma")


def cmd_revisoes(args):
    pasta = Path(args.pasta).expanduser()
    hoje = data(args.data) or dt.date.today()
    notas, _ = carregar(pasta)
    vencidas, proximas, sem_agenda = [], [], []
    for slug, n in notas.items():
        d = data(n["meta"].get("proxima_revisao"))
        nv = n["meta"].get("nivel", "?")
        if d is None:
            sem_agenda.append(slug)
        elif d <= hoje:
            vencidas.append((d, slug, nv))
        elif d <= hoje + dt.timedelta(days=args.dias):
            proximas.append((d, slug, nv))
    print(f"# Revisões: {pasta.name} ({hoje})")
    print(f"\nVencidas ({len(vencidas)}):")
    for d, s, nv in sorted(vencidas):
        print(f"- {s} · nível {nv} · desde {d}")
    print(f"\nPróximos {args.dias} dias ({len(proximas)}):")
    for d, s, nv in sorted(proximas):
        print(f"- {s} · nível {nv} · {d}")
    if sem_agenda:
        print(f"\nSem revisão agendada: {', '.join(sorted(sem_agenda))}")


def atualizar_chave(texto, chave, valor):
    linhas = texto.split("\n")
    fim = next(i for i in range(1, len(linhas)) if linhas[i].strip() == "---")
    for i in range(1, fim):
        if re.match(rf"^{re.escape(chave)}\s*:", linhas[i]):
            linhas[i] = f"{chave}: {valor}"
            # remove itens de lista em bloco que pertenciam a esta chave
            j = i + 1
            while j < fim and linhas[j].lstrip().startswith("- ") and linhas[j][:1] in (" ", "\t", "-"):
                del linhas[j]
                fim -= 1
            return "\n".join(linhas)
    linhas.insert(fim, f"{chave}: {valor}")
    return "\n".join(linhas)


def gravar_atomico(path, texto):
    """Grava num temporário na mesma pasta e troca de uma vez com os.replace: se algo
    interromper (processo morto, disco cheio), o arquivo fica inteiro na versão antiga."""
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(texto)
            f.flush()
            os.fsync(f.fileno())
        if path.exists():
            os.chmod(tmp, path.stat().st_mode & 0o777)  # mkstemp cria com 0600
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def cmd_registrar(args):
    pasta = Path(args.pasta).expanduser()
    hoje = data(args.data) or dt.date.today()
    notas, _ = carregar(pasta)
    slug = normalizar(Path(args.conceito).stem)
    if slug not in notas:
        sys.exit(f"Conceito '{slug}' não encontrado em {pasta}. Existentes: {', '.join(sorted(notas))}")
    n = notas[slug]
    texto = n["path"].read_text(encoding="utf-8")
    if not texto.startswith("---"):
        texto = f"---\ntipo: conceito\n---\n{texto}"

    atual = inteiro(n["meta"].get("intervalo")) or 0
    if args.resultado == "acertou":
        novo = next((i for i in INTERVALOS if i > atual), INTERVALOS[-1])
    elif args.resultado == "parcial":
        novo = max(1, atual)
    else:
        novo = 1
    proxima = hoje + dt.timedelta(days=novo)

    texto = atualizar_chave(texto, "intervalo", novo)
    texto = atualizar_chave(texto, "proxima_revisao", proxima.isoformat())
    texto = atualizar_chave(texto, "ultima_revisao", hoje.isoformat())
    nivel_txt = ""
    if args.nivel is not None:
        texto = atualizar_chave(texto, "nivel", args.nivel)
        nivel_txt = f", nível → {args.nivel}"

    entrada = f"- {hoje}: {args.resultado} (intervalo {novo}d{nivel_txt})"
    cab = re.search(r"^##\s+Revisões[ \t]*$", texto, re.M)
    if cab:
        prox = re.search(r"^##\s", texto[cab.end():], re.M)
        fim = cab.end() + prox.start() if prox else len(texto)
        bloco = texto[cab.end():fim].strip("\n")
        bloco = f"{bloco}\n{entrada}" if bloco else entrada
        texto = texto[:cab.end()] + "\n" + bloco + ("\n\n" + texto[fim:] if prox else "\n")
    else:
        texto = texto.rstrip("\n") + f"\n\n## Revisões\n{entrada}\n"

    # Uma nota que é link para fora da pasta (ex.: vault baixado de outra pessoa) não é gravada.
    destino = n["path"].resolve()
    if not destino.is_relative_to(pasta.resolve()):
        sys.exit(f"Recusado: {n['path']} aponta para {destino}, fora de {pasta}. Nada foi gravado.")
    gravar_atomico(destino, texto)
    print(f"{slug}: {args.resultado} → próxima revisão em {proxima} (intervalo {novo}d{nivel_txt})")


# ---------------------------------------------------------------------------
# Tempo de estudo: _sessoes.md (registro) e _progresso.md (gráficos, gerado)
# ---------------------------------------------------------------------------

def minutos_do_dia(hhmm):
    h, m = hhmm.split(":")
    return int(h) * 60 + int(m)


def fmt_min(m):
    m = int(round(m))
    return f"{m // 60} h {m % 60:02d} min" if m >= 60 else f"{m} min"


def metas(pasta):
    """Metas do frontmatter do _perfil.md: meta_semanal_min, meta_sessao_min, estimativa_min_conceito."""
    perfil = pasta / "_perfil.md"
    meta = ler_frontmatter(perfil.read_text(encoding="utf-8"))[0] if perfil.is_file() else {}
    semanal = inteiro(meta.get("meta_semanal_min"))
    sessao = inteiro(meta.get("meta_sessao_min")) or semanal
    conceito = inteiro(meta.get("estimativa_min_conceito")) or sessao
    return {"semanal": semanal, "sessao": sessao, "conceito": conceito}


def ler_sessoes(path):
    """(cabeçalho, sessões). Cada sessão guarda o texto original, para gravar sem tocar no que não mudou."""
    if not path.is_file():
        return None, []
    texto = path.read_text(encoding="utf-8")
    achados = list(SESSAO_RE.finditer(texto))
    cabecalho = texto[: achados[0].start()] if achados else texto
    sessoes = []
    for i, m in enumerate(achados):
        fim_bloco = achados[i + 1].start() if i + 1 < len(achados) else len(texto)
        s = {"data": data(m.group(1)), "hora": m.group(2), "plano": {}, "plano_min": None,
             "etapas": [], "fim": None, "bruto": texto[m.start():fim_bloco], "mudou": False}
        for linha in texto[m.end():fim_bloco].splitlines():
            l = linha.strip()
            if l.startswith("plano:"):
                partes = [p.strip() for p in l[6:].split("·")]
                s["plano_min"] = inteiro(partes[0].split()[0]) if partes[0] else None
                for p in partes[1:]:
                    nome, _, valor = p.rpartition(" ")
                    if nome and inteiro(valor) is not None:
                        s["plano"][nome] = inteiro(valor)
            elif l.startswith("fim:"):
                s["fim"] = l[4:].strip() or None
            elif l.startswith("|"):
                cels = [c.strip() for c in l.strip("|").split("|")]
                if len(cels) < 6 or cels[0] == "etapa" or set(cels[0]) <= set("-: "):
                    continue
                etapa, conceito, ini, fim, mins, plano = cels[:6]
                s["etapas"].append({"etapa": etapa, "conceito": None if conceito in ("", "-") else conceito,
                                    "inicio": ini, "fim": fim or None, "min": inteiro(mins), "plano": inteiro(plano)})
        sessoes.append(s)
    return cabecalho, sessoes


def duracao(e):
    """Minutos de uma etapa: a coluna min (que pode ter sido corrigida à mão) ou fim − início."""
    if e["min"] is not None:
        return e["min"]
    if e["fim"] and HORA_RE.match(e["fim"]) and HORA_RE.match(e["inicio"]):
        return (minutos_do_dia(e["fim"]) - minutos_do_dia(e["inicio"])) % 1440
    return 0


def total_sessao(s):
    return sum(duracao(e) for e in s["etapas"])


def render_sessao(s):
    plano = f"plano: {s['plano_min']} min" if s["plano_min"] else "plano: -"
    plano += "".join(f" · {k} {v}" for k, v in s["plano"].items())
    linhas = [f"## {s['data']} {s['hora']}", plano, "",
              "| etapa | conceito | início | fim | min | plano |", "|---|---|---|---|---|---|"]
    for e in s["etapas"]:
        mins = "" if e["min"] is None else str(e["min"])
        plan = "" if e["plano"] is None else str(e["plano"])
        linhas.append(f"| {e['etapa']} | {e['conceito'] or '-'} | {e['inicio']} | {e['fim'] or ''} | {mins} | {plan} |")
    if s["fim"]:
        linhas += ["", f"fim: {s['fim']}"]
    return "\n".join(linhas) + "\n\n"


def gravar_sessoes(path, pasta, cabecalho, sessoes):
    if cabecalho is None:
        cabecalho = (f"---\ntipo: sessoes\nassunto: {pasta.name}\n---\n# Sessões — {pasta.name}\n\n"
                     "Tempo de estudo, etapa a etapa (gravado por `arvore.py sessao`). Pode corrigir a "
                     "coluna **min** à mão, por exemplo para descontar uma pausa: os totais de "
                     "`_progresso.md` usam esse valor.\n\n")
    corpo = "".join(render_sessao(s) if s["mudou"] else s["bruto"] for s in sessoes)
    gravar_atomico(path, cabecalho + corpo)


def fechar_etapa(s, hora, descontar, avisos):
    if not s["etapas"] or s["etapas"][-1]["fim"]:
        return
    e = s["etapas"][-1]
    bruto = (minutos_do_dia(hora) - minutos_do_dia(e["inicio"])) % 1440
    e["fim"], e["min"] = hora, max(0, bruto - (descontar or 0))
    limite = max(30, 3 * (e["plano"] or 0))
    if e["min"] > limite:
        avisos.append(f"A etapa '{e['etapa']}' durou {e['min']} min (mais que {limite}). Pode ter havido "
                      "pausa: pergunte ao aluno e corrija a coluna min em _sessoes.md se for o caso.")
    s["mudou"] = True


def largar_sessao(s, avisos):
    """Sessão que ficou aberta (sem `fim`): a última etapa conta pelo plano, ou zero."""
    if s["etapas"] and not s["etapas"][-1]["fim"]:
        e = s["etapas"][-1]
        e["fim"], e["min"] = "?", e["plano"] or 0
    s["fim"] = "não registrado"
    s["mudou"] = True
    avisos.append(f"A sessão de {s['data']} {s['hora']} ficou sem `fim`; a última etapa entrou pelo "
                  "tempo planejado. Corrija a coluna min se souber o tempo real.")


def montar_plano(texto, meta_sessao):
    if texto:
        plano = {}
        for item in texto.split(","):
            nome, _, valor = item.partition("=")
            if not nome.strip() or inteiro(valor) is None:
                sys.exit(f"Plano inválido: '{item}'. Use etapa=minutos separados por vírgula.")
            plano[nome.strip()] = inteiro(valor)
        return plano, sum(plano.values())
    if meta_sessao:
        return {k: max(1, round(meta_sessao * p / 100)) for k, p in PLANO_PADRAO.items()}, meta_sessao
    return {}, None


def nova_sessao(sessoes, dia, hora, plano, plano_min):
    s = {"data": dia, "hora": hora, "plano": plano, "plano_min": plano_min,
         "etapas": [], "fim": None, "bruto": "", "mudou": True}
    sessoes.append(s)
    return s


def sessao_aberta(sessoes, dia, hora, avisos):
    """A sessão em andamento, ou None. Larga a que ficou aberta em outro dia ou parada há horas."""
    if not sessoes or sessoes[-1]["fim"]:
        return None
    s = sessoes[-1]
    ultima = s["etapas"][-1] if s["etapas"] else None
    parada = ultima and not ultima["fim"] and (minutos_do_dia(hora) - minutos_do_dia(ultima["inicio"])) % 1440 > ABANDONO_MIN
    if s["data"] != dia or parada:
        largar_sessao(s, avisos)
        return None
    return s


def cmd_sessao(args):
    pasta = Path(args.pasta).expanduser()
    if not pasta.is_dir():
        sys.exit(f"Pasta do assunto não encontrada: {pasta}")
    dia = data(args.data) or dt.date.today()
    hora = args.hora or dt.datetime.now().strftime("%H:%M")
    if not HORA_RE.match(hora):
        sys.exit(f"Hora inválida: {hora} (use HH:MM)")
    path = pasta / SESSOES
    if path.is_symlink() or (path.exists() and not path.resolve().is_relative_to(pasta.resolve())):
        sys.exit(f"Recusado: {path} aponta para fora de {pasta}. Nada foi gravado.")
    cabecalho, sessoes = ler_sessoes(path)
    m = metas(pasta)
    avisos = []
    s = sessao_aberta(sessoes, dia, hora, avisos)

    if args.acao == "inicio":
        if s:
            fechar_etapa(s, hora, 0, avisos)
            s["fim"] = hora
        plano, plano_min = montar_plano(args.plano, m["sessao"])
        historico = medias_por_etapa(sessoes)
        nova_sessao(sessoes, dia, hora, plano, plano_min)
        gravar_sessoes(path, pasta, cabecalho, sessoes)
        print(f"Sessão iniciada: {dia} {hora} · plano {fmt_min(plano_min) if plano_min else 'sem meta'}"
              + "".join(f" · {k} {v}" for k, v in plano.items()))
        if historico:
            print("Médias reais por etapa (últimas 5 sessões): "
                  + " · ".join(f"{k} {v:.0f}" for k, v in historico.items()))
        if not m["semanal"]:
            print("Sem meta: grave meta_semanal_min (e meta_sessao_min) no frontmatter do _perfil.md.")

    elif args.acao == "etapa":
        if not args.nome:
            sys.exit("Diga o nome da etapa: sessao <pasta> etapa <nome>")
        if s is None:
            plano, plano_min = montar_plano(None, m["sessao"])
            s = nova_sessao(sessoes, dia, hora, plano, plano_min)
            print(f"(sessão iniciada automaticamente às {hora})")
        anterior = s["etapas"][-1]["conceito"] if s["etapas"] else None
        fechar_etapa(s, hora, args.descontar, avisos)
        conceito = normalizar(args.conceito) if args.conceito else anterior
        s["etapas"].append({"etapa": args.nome, "conceito": conceito, "inicio": hora, "fim": None,
                            "min": None, "plano": s["plano"].get(args.nome)})
        s["mudou"] = True
        gravar_sessoes(path, pasta, cabecalho, sessoes)
        feito = sum(duracao(e) for e in s["etapas"][:-1])
        print(f"Etapa '{args.nome}' ({conceito or 'sem conceito'}) às {hora} · "
              f"plano da etapa: {s['plano'].get(args.nome, '-')} min · sessão até agora: {fmt_min(feito)}"
              + (f" de {fmt_min(s['plano_min'])}" if s["plano_min"] else ""))

    else:  # fim
        if s is None:
            print("Nenhuma sessão aberta hoje.")
            for a in avisos:
                print(f"Aviso: {a}")
            if avisos:
                gravar_sessoes(path, pasta, cabecalho, sessoes)
            return
        fechar_etapa(s, hora, args.descontar, avisos)
        s["fim"] = hora
        gravar_sessoes(path, pasta, cabecalho, sessoes)
        print(resumo_sessao(s))
        print()
        print(texto_expectativa(gerar_progresso(pasta, sessoes, m, dia)))
        _, dias, _ = ler_respostas(pasta / RESPOSTAS)
        grupos = [g for g in por_sessao(pasta, {str(dia): dias.get(str(dia), [])}) if g[0] == f"{dia} {s['hora']}"]
        if grupos and grupos[0][1]:
            print()
            print(texto_curva(grupos))
            print("→ atualize \"Como este aluno aprende\" no _perfil.md com o que esta sessão mostrou.")
    for a in avisos:
        print(f"Aviso: {a}")


def medias_por_etapa(sessoes, n=5):
    """Minutos médios de cada etapa nas últimas n sessões encerradas: base para um plano realista."""
    somas = {}
    for s in [s for s in sessoes if s["fim"]][-n:]:
        for e in s["etapas"]:
            somas.setdefault(e["etapa"], []).append(duracao(e))
    return {k: sum(v) / len(v) for k, v in somas.items()}


def semanas_txt(n):
    return "última semana fechada" if n == 1 else f"últimas {n} semanas fechadas"


def resumo_sessao(s):
    total = total_sessao(s)
    linhas = [f"# Sessão {s['data']} {s['hora']}–{s['fim']}: {fmt_min(total)}"
              + (f" (plano {fmt_min(s['plano_min'])})" if s["plano_min"] else "")]
    linhas.append("| etapa | conceito | min | plano |")
    for e in s["etapas"]:
        linhas.append(f"| {e['etapa']} | {e['conceito'] or '-'} | {duracao(e)} | {e['plano'] if e['plano'] is not None else '-'} |")
    if s["plano_min"]:
        dif = total - s["plano_min"]
        passou = sorted(((duracao(e) - e["plano"], e["etapa"]) for e in s["etapas"] if e["plano"] is not None), reverse=True)
        passou = [f"{n} ({d:+d})" for d, n in passou if d >= FOLGA_MIN]
        if abs(dif) <= max(FOLGA_MIN, TOLERANCIA * s["plano_min"]):
            linhas.append(f"Sessão: dentro do plano ({dif:+d} min) → não comentar."
                          + (f" Só para o próximo plano: {', '.join(passou)}." if passou else ""))
        else:
            sentido = "acima" if dif > 0 else "abaixo"
            linhas.append(f"Sessão: {abs(dif)} min {sentido} do plano ({dif / s['plano_min']:+.0%})."
                          + (f" Etapas que passaram: {', '.join(passou)}." if passou else "")
                          + " → ajuste o plano da próxima sessão (sem cobrar o aluno).")
    return "\n".join(linhas)


def segunda(d):
    return d - dt.timedelta(days=d.weekday())


def conceitos_do_mapa(pasta):
    """Slugs da trilha principal: links sob títulos `## Módulo...` do _mapa.md (ou todos, se não houver)."""
    mapa = pasta / "_mapa.md"
    if not mapa.is_file():
        return []
    corpo = ler_frontmatter(mapa.read_text(encoding="utf-8"))[1]
    blocos = re.findall(r"^##\s+M[óo]dulo.*?(?=^##\s|\Z)", corpo, re.M | re.S | re.I)
    vistos = []
    for slug in alvos("\n".join(blocos) if blocos else corpo):
        if slug not in vistos:
            vistos.append(slug)
    return vistos


def calcular(pasta, sessoes, m, hoje):
    """Números do progresso: semanas, etapas, conceitos, ritmo da trilha e o veredito da expectativa."""
    semanas = {}
    if sessoes:
        w, fim = segunda(min(s["data"] for s in sessoes)), segunda(hoje)
        while w <= fim:
            semanas[w] = 0
            w += dt.timedelta(days=7)
    etapas, por_conceito = {}, {}
    for s in sessoes:
        if s["data"] <= hoje:
            semanas[segunda(s["data"])] = semanas.get(segunda(s["data"]), 0) + total_sessao(s)
        for e in s["etapas"]:
            d = duracao(e)
            plan = etapas.setdefault(e["etapa"], {"min": 0, "plano": 0, "vezes": 0})
            plan["min"] += d
            plan["plano"] += e["plano"] or 0
            plan["vezes"] += 1
            c = por_conceito.setdefault(e["conceito"], {"min": 0, "novo": 0, "etapas": {}, "sessoes": set()})
            c["min"] += d
            c["novo"] += 0 if e["etapa"] == "revisao" else d
            c["etapas"][e["etapa"]] = c["etapas"].get(e["etapa"], 0) + d
            c["sessoes"].add((s["data"], s["hora"]))
    atual = semanas.get(segunda(hoje), 0)
    fechadas = [v for w, v in sorted(semanas.items()) if w < segunda(hoje)][-4:]
    media = sum(fechadas) / len(fechadas) if fechadas else None

    meta = m["semanal"]
    if not meta:
        veredito, motivo = "sem meta", "grave meta_semanal_min no frontmatter do _perfil.md"
    elif atual >= meta * (1 + TOLERANCIA) and atual - meta > FOLGA_MIN:
        veredito, motivo = "parabenizar", f"esta semana: {fmt_min(atual)} para meta de {fmt_min(meta)}"
    elif media is not None and len(fechadas) >= 2 and media >= meta * (1 + TOLERANCIA) and media - meta > FOLGA_MIN:
        veredito, motivo = "parabenizar", f"média de {fmt_min(media)}/semana para meta de {fmt_min(meta)}"
    elif media is not None and len(fechadas) >= 2 and media < meta * (1 - TOLERANCIA) and meta - media > FOLGA_MIN:
        veredito, motivo = "conversar", f"média de {fmt_min(media)}/semana nas últimas {len(fechadas)} semanas, meta de {fmt_min(meta)}"
    else:
        veredito = "não comentar"
        motivo = ("menos de 2 semanas fechadas" if len(fechadas) < 2 and atual < meta * (1 + TOLERANCIA)
                  else "dentro da tolerância")

    medidos = [c["novo"] for slug, c in por_conceito.items() if slug and c["novo"] > 0]
    min_conceito = sum(medidos) / len(medidos) if medidos else None
    niveis = {slug: inteiro(n["meta"].get("nivel")) for slug, n in carregar(pasta)[0].items()}
    trilha = conceitos_do_mapa(pasta)
    restantes = [c for c in trilha if (niveis.get(c) or 0) < 3]
    ritmo = None
    if min_conceito and (media or meta):
        por_semana = media if media else meta
        ritmo = {"restantes": len(restantes), "min_conceito": min_conceito, "medidos": len(medidos),
                 "semanas": math.ceil(len(restantes) * min_conceito / por_semana) if por_semana else None,
                 "semanas_plano": math.ceil(len(restantes) * m["conceito"] / meta) if meta and m["conceito"] else None}
        if (veredito == "não comentar" and m["conceito"] and len(medidos) >= 3
                and min_conceito > m["conceito"] * (1 + TOLERANCIA) and min_conceito - m["conceito"] > FOLGA_MIN):
            veredito = "conversar"
            motivo = (f"cada conceito novo leva {fmt_min(min_conceito)} em média, contra {fmt_min(m['conceito'])} "
                      f"estimados: a trilha vai levar ~{ritmo['semanas']} semanas, não ~{ritmo['semanas_plano']}")
    return {"semanas": semanas, "atual": atual, "fechadas": fechadas, "media": media, "meta": m,
            "veredito": veredito, "motivo": motivo, "etapas": etapas, "por_conceito": por_conceito,
            "niveis": niveis, "trilha": trilha, "ritmo": ritmo, "sessoes": sessoes, "hoje": hoje}


def texto_expectativa(r):
    linhas = ["# Expectativa"]
    meta = r["meta"]["semanal"]
    linhas.append(f"Esta semana: {fmt_min(r['atual'])}" + (f" de {fmt_min(meta)}" if meta else ""))
    if r["media"] is not None:
        linhas.append(f"Média semanal ({semanas_txt(len(r['fechadas']))}): {fmt_min(r['media'])}")
    if r["ritmo"]:
        rt = r["ritmo"]
        linhas.append(f"Ritmo: {fmt_min(rt['min_conceito'])} por conceito novo ({rt['medidos']} medidos) · "
                      f"{rt['restantes']} conceitos da trilha abaixo do nível 3 → ~{rt['semanas']} semanas"
                      + (f" (no plano: ~{rt['semanas_plano']})" if rt["semanas_plano"] else ""))
    linhas.append(f"Veredito: {r['veredito'].upper()} ({r['motivo']})")
    if r["veredito"] == "parabenizar":
        linhas.append("→ parabenize pelo tempo a mais, de forma específica, e NÃO aumente a meta.")
    elif r["veredito"] == "conversar":
        linhas.append("→ proponha um reajuste (meta, plano das sessões ou trilha) e deixe o aluno escolher.")
    return "\n".join(linhas)


def xychart(titulo, rotulos, barras, linha=None, eixo="min"):
    topo = max(barras + (linha or []) + [10])
    topo = int(math.ceil(topo * 1.15 / 10) * 10)
    rot = ", ".join(f'"{r}"' for r in rotulos)
    out = ["```mermaid", "xychart-beta", f'    title "{titulo}"', f"    x-axis [{rot}]",
           f'    y-axis "{eixo}" 0 --> {topo}', f"    bar [{', '.join(str(int(round(b))) for b in barras)}]"]
    if linha:
        out.append(f"    line [{', '.join(str(int(round(v))) for v in linha)}]")
    return out + ["```"]


def pizza(titulo, fatias):
    out = ["```mermaid", f"pie title {titulo}" if titulo else "pie"]
    out += [f'    "{nome}" : {int(round(v))}' for nome, v in fatias if v > 0]
    return out + ["```"]


def gerar_progresso(pasta, sessoes, m, hoje):
    """Refaz _progresso.md (gráficos mermaid para o Obsidian) e devolve os números calculados."""
    r = calcular(pasta, sessoes, m, hoje)
    if not sessoes:
        return r
    meta = m["semanal"]
    total = sum(total_sessao(s) for s in sessoes)
    planos = [s["plano_min"] for s in sessoes if s["plano_min"]]
    md = ["---", "tipo: progresso", f"assunto: {pasta.name}", f"atualizado: {hoje}", "---",
          f"# Progresso — {pasta.name}", "",
          "> [!info] Gerado por `arvore.py` a partir de `_sessoes.md` a cada fim de sessão. "
          "Não edite aqui: corrija os minutos em `_sessoes.md`.", "", "## Resumo", "", "| | |", "|---|---|",
          f"| Sessões | {len(sessoes)} |", f"| Tempo total | {fmt_min(total)} |",
          f"| Média por sessão | {fmt_min(total / len(sessoes))}"
          + (f" (plano {fmt_min(sum(planos) / len(planos))})" if planos else "") + " |",
          f"| Esta semana | {fmt_min(r['atual'])}" + (f" de {fmt_min(meta)}" if meta else "") + " |"]
    if r["media"] is not None:
        md.append(f"| Média semanal ({semanas_txt(len(r['fechadas']))}) | {fmt_min(r['media'])} |")
    if r["ritmo"]:
        rt = r["ritmo"]
        md.append(f"| Tempo por conceito novo | {fmt_min(rt['min_conceito'])}"
                  + (f" (estimado {fmt_min(m['conceito'])})" if m["conceito"] else "") + " |")
        md.append(f"| Previsão da trilha | {rt['restantes']} conceitos abaixo do nível 3 ≈ {rt['semanas']} semanas"
                  + (f" (no plano: {rt['semanas_plano']})" if rt["semanas_plano"] else "") + " |")
    if r["veredito"] == "parabenizar":
        md += ["", f"> [!success] Acima do combinado: {r['motivo']}. O tempo a mais é bônus: a meta continua a mesma."]
    elif r["veredito"] == "conversar":
        md += ["", f"> [!warning] Vale reajustar a expectativa: {r['motivo']}."]

    semanas = sorted(r["semanas"].items())[-12:]
    md += ["", "## Tempo por semana", ""]
    md += xychart("Minutos por semana" + (f" (linha = meta de {meta} min)" if meta else ""),
                  [w.strftime("%d/%m") for w, _ in semanas], [v for _, v in semanas],
                  [meta] * len(semanas) if meta else None)

    ultimas = sessoes[-15:]
    md += ["", "## Sessões", ""]
    md += xychart("Minutos por sessão (linha = plano)", [f"{s['data']:%d/%m} {s['hora']}" for s in ultimas],
                  [total_sessao(s) for s in ultimas],
                  [s["plano_min"] or 0 for s in ultimas] if any(s["plano_min"] for s in ultimas) else None)
    md += ["", "| sessão | conceitos | min | plano | etapas |", "|---|---|---|---|---|"]
    for s in reversed(sessoes):
        conceitos = []
        for e in s["etapas"]:
            if e["conceito"] and f"[[{e['conceito']}]]" not in conceitos:
                conceitos.append(f"[[{e['conceito']}]]")
        etapas = ", ".join(f"{e['etapa']} {duracao(e)}" for e in s["etapas"])
        md.append(f"| {s['data']} {s['hora']} | {', '.join(conceitos) or '-'} | {total_sessao(s)} | "
                  f"{s['plano_min'] or '-'} | {etapas or '-'} |")

    md += ["", "## Tempo por etapa", ""]
    md += pizza("Tempo total por etapa", [(k, v["min"]) for k, v in r["etapas"].items()])
    md += ["", "| etapa | vezes | média | plano médio | total |", "|---|---|---|---|---|"]
    for k, v in sorted(r["etapas"].items(), key=lambda kv: -kv[1]["min"]):
        plano_medio = fmt_min(v["plano"] / v["vezes"]) if v["plano"] else "-"
        md.append(f"| {k} | {v['vezes']} | {fmt_min(v['min'] / v['vezes'])} | {plano_medio} | {fmt_min(v['min'])} |")

    ordem = [c for c in r["trilha"] if c in r["por_conceito"]]
    ordem += sorted(c for c in r["por_conceito"] if c and c not in ordem)
    md += ["", "## Por tópico", "", "Clique num conceito para abrir o detalhe.", ""]
    if ordem:
        md += xychart("Minutos por conceito", ordem, [r["por_conceito"][c]["min"] for c in ordem])
    for c in ordem + ([None] if None in r["por_conceito"] else []):
        d = r["por_conceito"][c]
        nome = f"[[{c}]]" if c else "Sem conceito"
        nivel = f" · nível {r['niveis'][c]}" if c in r["niveis"] and r["niveis"][c] is not None else ""
        md += ["", f"> [!abstract]- {nome} · {fmt_min(d['min'])}{nivel} · {len(d['sessoes'])} sessão(ões)",
               "> | etapa | min |", "> |---|---|"]
        md += [f"> | {k} | {v} |" for k, v in sorted(d["etapas"].items(), key=lambda kv: -kv[1])]
        if len([v for v in d["etapas"].values() if v > 0]) > 1:
            md += [">"] + [f"> {l}" for l in pizza("", sorted(d["etapas"].items(), key=lambda kv: -kv[1]))]
    destino = pasta / PROGRESSO
    if destino.is_symlink():
        sys.exit(f"Recusado: {destino} é um link. Nada foi gravado.")
    gravar_atomico(destino, "\n".join(md) + "\n")
    return r


def cmd_progresso(args):
    pasta = Path(args.pasta).expanduser()
    _, sessoes = ler_sessoes(pasta / SESSOES)
    if not sessoes:
        print(f"Nenhuma sessão registrada em {pasta / SESSOES}.")
        return
    hoje = data(args.data) or dt.date.today()
    print(texto_expectativa(gerar_progresso(pasta, sessoes, metas(pasta), hoje)))
    print(f"\nGráficos em {pasta / PROGRESSO}")


# ---------------------------------------------------------------------------
# Curva de aprendizado: _respostas.md (cada resposta classificada) e veredito de ritmo
# ---------------------------------------------------------------------------

RESULTADOS = ["sozinho", "dica", "execucao", "conceito", "perdido"]
ERROS = {"execucao", "conceito", "perdido"}
PREVISOES = ["sozinho", "dica", "trava"]
SINAIS = ["cansaco", "frustracao", "contestou", "confiante", "rapido"]
JANELA = 5  # últimas respostas que decidem o ritmo
JANELA_SINAL = 2  # cansaço e frustração só contam se apareceram nas últimas respostas
DIA_RE = re.compile(r"^##\s+(\d{4}-\d{2}-\d{2})\s*$", re.M)
CABECALHO_RESPOSTAS = "| hora | conceito | resultado | previsto | sinal | nota |"


def ler_respostas(path):
    """(cabeçalho, {dia: [resposta, ...]}, {dia: texto original}) na ordem do arquivo."""
    if not path.is_file():
        return None, {}, {}
    texto = path.read_text(encoding="utf-8")
    achados = list(DIA_RE.finditer(texto))
    cabecalho = texto[: achados[0].start()] if achados else texto
    dias, brutos = {}, {}
    for i, m in enumerate(achados):
        fim_bloco = achados[i + 1].start() if i + 1 < len(achados) else len(texto)
        brutos[m.group(1)] = brutos.get(m.group(1), "") + texto[m.start():fim_bloco]
        linhas = dias.setdefault(m.group(1), [])
        for linha in texto[m.end():fim_bloco].splitlines():
            l = linha.strip()
            if not l.startswith("|"):
                continue
            cels = [c.strip() for c in l.strip("|").split("|")]
            if len(cels) < 6 or cels[0] == "hora" or set(cels[0]) <= set("-: "):
                continue
            hora, conceito, resultado, previsto, sinal, nota = cels[:6]
            linhas.append({"hora": hora, "conceito": None if conceito in ("", "-") else conceito,
                           "resultado": resultado, "previsto": None if previsto in ("", "-") else previsto,
                           "sinais": [x for x in re.split(r"[,\s]+", sinal) if x and x != "-"], "nota": nota})
    return cabecalho, dias, brutos


def gravar_respostas(path, pasta, cabecalho, dias, brutos, mudou):
    """Regrava só o dia `mudou`; os outros dias ficam com o texto original (notas à mão incluídas)."""
    if cabecalho is None:
        cabecalho = (f"---\ntipo: respostas\nassunto: {pasta.name}\n---\n# Respostas — {pasta.name}\n\n"
                     "Cada resposta do aluno a uma pergunta ou problema, classificada pelo professor "
                     "(gravado por `arvore.py resposta`; `arvore.py curva` resume). Resultados: sozinho, "
                     "dica, execucao (ideia certa, erro de conta ou unidade), conceito (ideia errada), "
                     "perdido (não sabe por onde começar).\n\n")
    corpo = ""
    for dia in sorted(dias):
        if dia != mudou and dia in brutos:
            corpo += brutos[dia] if brutos[dia].endswith("\n\n") else brutos[dia].rstrip("\n") + "\n\n"
            continue
        corpo += f"## {dia}\n\n{CABECALHO_RESPOSTAS}\n|---|---|---|---|---|---|\n"
        for r in dias[dia]:
            corpo += (f"| {r['hora']} | {r['conceito'] or '-'} | {r['resultado']} | {r['previsto'] or '-'} | "
                      f"{', '.join(r['sinais']) or '-'} | {r['nota']} |\n")
        corpo += "\n"
    gravar_atomico(path, cabecalho + corpo)


def caminho_respostas(pasta):
    path = pasta / RESPOSTAS
    if path.is_symlink() or (path.exists() and not path.resolve().is_relative_to(pasta.resolve())):
        sys.exit(f"Recusado: {path} aponta para fora de {pasta}. Nada foi gravado.")
    return path


def cmd_resposta(args):
    pasta = Path(args.pasta).expanduser()
    if not pasta.is_dir():
        sys.exit(f"Pasta do assunto não encontrada: {pasta}")
    dia = str(data(args.data) or dt.date.today())
    hora = args.hora or dt.datetime.now().strftime("%H:%M")
    if not HORA_RE.match(hora):
        sys.exit(f"Hora inválida: {hora} (use HH:MM)")
    path = caminho_respostas(pasta)
    cabecalho, dias, brutos = ler_respostas(path)
    conceito = normalizar(args.conceito) if args.conceito else None
    if conceito and not re.fullmatch(r"[\w.-]+", conceito):
        sys.exit(f"Conceito inválido: {conceito} (use o slug da nota, ex.: derivada-num-ponto)")
    if conceito is None:  # herda o conceito da etapa aberta do cronômetro, se houver
        _, sessoes = ler_sessoes(pasta / SESSOES)
        if sessoes and not sessoes[-1]["fim"] and str(sessoes[-1]["data"]) == dia and sessoes[-1]["etapas"]:
            conceito = sessoes[-1]["etapas"][-1]["conceito"]
    nota = " ".join((args.nota or "").replace("|", "/").split())
    dias.setdefault(dia, []).append({"hora": hora, "conceito": conceito, "resultado": args.resultado,
                                     "previsto": args.previsto, "sinais": args.sinal or [], "nota": nota})
    gravar_respostas(path, pasta, cabecalho, dias, brutos, dia)
    atual = por_sessao(pasta, {dia: dias[dia]})[-1][1]
    print(f"Resposta {len(atual)} desta sessão: {args.resultado}" + (f" ({conceito})" if conceito else ""))
    print(veredito_ritmo(atual)[1])


def por_sessao(pasta, dias):
    """[(rótulo, respostas)]: cada resposta vai para a sessão do mesmo dia que começou antes dela."""
    _, sessoes = ler_sessoes(pasta / SESSOES)
    inicios = {}
    for se in sessoes:
        if HORA_RE.match(se["hora"]):
            inicios.setdefault(str(se["data"]), []).append(se["hora"])
    grupos = []
    for dia in sorted(dias):
        hs = sorted(inicios.get(dia, []), key=minutos_do_dia)
        baldes = {}
        for r in dias[dia]:
            h = r["hora"] if HORA_RE.match(r["hora"]) else "00:00"
            antes = [x for x in hs if minutos_do_dia(x) <= minutos_do_dia(h)]
            baldes.setdefault(antes[-1] if antes else None, []).append(r)
        for ini in sorted(baldes, key=lambda x: -1 if x is None else minutos_do_dia(x)):
            grupos.append((f"{dia} {ini}" if ini else dia, baldes[ini]))
    return grupos


def ordem(r):
    """sozinho 0, dica 1, trava 2: para comparar previsão com resultado."""
    return {"sozinho": 0, "dica": 1}.get(r, 2)


def veredito_ritmo(respostas):
    """(código, texto) a partir das últimas JANELA respostas de uma sessão."""
    janela = respostas[-JANELA:]
    if len(janela) < 3:
        return "POUCOS DADOS", f"Ritmo: poucos dados ({len(janela)} resposta(s)); continue observando."
    erros = [r["resultado"] for r in janela if r["resultado"] in ERROS]
    sinais = {x for r in janela[-JANELA_SINAL:] for x in r["sinais"]}
    acertos = len(janela) - len(erros)
    sozinho = sum(r["resultado"] == "sozinho" for r in janela)
    seq = " → ".join(r["resultado"] for r in janela)
    if sinais & {"cansaco", "frustracao"}:
        return "FECHAR", (f"Ritmo: FECHAR ({seq}; sinais: {', '.join(sorted(sinais))}) → um último problema que ele "
                          "resolve sozinho, resumo do que fez bem e próximo passo menor.")
    if "perdido" in erros or (len(erros) >= 2 and len(set(erros)) >= 2):
        return "DESACELERAR", (f"Ritmo: DESACELERAR ({seq}) → erros de tipos diferentes ou aluno perdido = "
                               "sobrecarga. Pare o problema, volte a um subproblema de 1 elemento, exemplo "
                               "resolvido e depois um quase igual.")
    if len(erros) >= 2:
        return f"MIRAR {erros[0]}", (f"Ritmo: MIRAR {erros[0]} ({seq}) → o mesmo erro se repete. Trate só ele, com "
                                     "um caso de contraste, antes de seguir.")
    if acertos / len(janela) >= 0.85 and sozinho >= 3:
        return "ACELERAR", (f"Ritmo: ACELERAR ({seq}) → tire o andaime, pule o exemplo resolvido, suba a "
                            "dificuldade ou misture conceitos.")
    return "MANTER", f"Ritmo: MANTER ({seq}) → dificuldade na zona certa."


def resumo_previsoes(respostas):
    com = [r for r in respostas if r["previsto"] in PREVISOES]
    if not com:
        return None
    certas = sum(ordem(r["previsto"]) == ordem(r["resultado"]) for r in com)
    otimista = sum(ordem(r["previsto"]) < ordem(r["resultado"]) for r in com)
    pessimista = len(com) - certas - otimista
    texto = f"Previsões: {certas}/{len(com)} certas"
    if len(com) < 3:
        return texto
    if otimista > pessimista:
        texto += f" · {otimista} otimista(s) demais → você está superestimando o domínio; passos menores."
    elif pessimista > otimista:
        texto += f" · {pessimista} pessimista(s) demais → ele pode mais do que você supõe; tire apoio."
    return texto


def texto_curva(grupos, n=None):
    """grupos: [(rótulo, respostas)] de por_sessao, em ordem."""
    grupos = [(g, rs) for g, rs in grupos if rs]
    if not grupos:
        return "Nenhuma resposta registrada ainda (arvore.py resposta)."
    if n:
        grupos = grupos[-n:]
    linhas = ["# Curva de aprendizado", "",
              "| sessão | respostas | sozinho | dica | execucao | conceito | perdido | acerto | ritmo |",
              "|---|---|---|---|---|---|---|---|---|"]
    for rotulo, rs in grupos:
        cont = {k: sum(r["resultado"] == k for r in rs) for k in RESULTADOS}
        acerto = (cont["sozinho"] + cont["dica"]) / len(rs)
        linhas.append(f"| {rotulo} | {len(rs)} | " + " | ".join(str(cont[k]) for k in RESULTADOS)
                      + f" | {acerto:.0%} | {veredito_ritmo(rs)[0]} |")
    rotulo, ultimo = grupos[-1]
    linhas += ["", f"Última sessão ({rotulo}):", veredito_ritmo(ultimo)[1]]
    prev = resumo_previsoes(ultimo)
    if prev:
        linhas.append(prev)
    sinais = {}
    for r in ultimo:
        for x in r["sinais"]:
            sinais[x] = sinais.get(x, 0) + 1
    if sinais:
        linhas.append("Sinais: " + ", ".join(f"{k} ×{v}" for k, v in sorted(sinais.items())))
    erros_conceito = {}
    for _, rs in grupos:
        for r in rs:
            if r["resultado"] in ("conceito", "perdido") and r["conceito"]:
                erros_conceito[r["conceito"]] = erros_conceito.get(r["conceito"], 0) + 1
    if erros_conceito:
        linhas.append("Onde trava (conceito/perdido): "
                      + ", ".join(f"{k} ×{v}" for k, v in sorted(erros_conceito.items(), key=lambda kv: -kv[1])))
    linhas.append("Meta da prática guiada: ~80% de acerto (sozinho + dica). Bem acima disso, acelere; bem abaixo, "
                  "passos menores.")
    return "\n".join(linhas)


def cmd_curva(args):
    pasta = Path(args.pasta).expanduser()
    _, dias, _ = ler_respostas(pasta / RESPOSTAS)
    print(texto_curva(por_sessao(pasta, dias), args.sessoes))


def arquivo_config():
    """Mesmo arquivo que a TUI `estudando` lê: $XDG_CONFIG_HOME/estudando/config.toml."""
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    return Path(base) / "estudando" / "config.toml"


def ler_base():
    """(pasta, origem) a partir de $ESTUDANDO_DIR ou do config; (None, None) se não houver."""
    env = os.environ.get("ESTUDANDO_DIR", "").strip()
    if env:
        return Path(env).expanduser(), "$ESTUDANDO_DIR"
    cfg = arquivo_config()
    if cfg.is_file():
        for linha in cfg.read_text(encoding="utf-8").splitlines():
            chave, sep, valor = linha.partition("=")
            valor = valor_toml(valor)
            if sep and chave.strip() == "pasta" and valor:
                return Path(valor).expanduser(), str(cfg)
    return None, None


def valor_toml(bruto):
    """Valor TOML simples: "básica" (com escapes), 'literal' ou sem aspas até o comentário.
    Mesma regra de `src/config.rs` da TUI."""
    v = bruto.strip()
    if v.startswith('"'):
        out, i = [], 1
        while i < len(v):
            c = v[i]
            if c == '"':
                return "".join(out)
            if c == "\\" and i + 1 < len(v):
                i += 1
                c = {"n": "\n", "t": "\t"}.get(v[i], v[i])
            out.append(c)
            i += 1
        return None
    if v.startswith("'"):
        fim = v.find("'", 1)
        return v[1:fim] if fim > 0 else None
    return v.split("#", 1)[0].strip()


def string_toml(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def sugestao_base():
    """<pasta Documentos do sistema>/Aprendizado (Documentos, Documents... conforme o idioma)."""
    casa = Path.home()
    dirs = Path(os.environ.get("XDG_CONFIG_HOME") or casa / ".config") / "user-dirs.dirs"
    if dirs.is_file():
        for linha in dirs.read_text(encoding="utf-8").splitlines():
            if linha.startswith("XDG_DOCUMENTS_DIR="):
                docs = linha.split("=", 1)[1].strip().strip('"').replace("$HOME", str(casa))
                if docs and Path(docs) != casa:
                    return Path(docs) / "Aprendizado"
    for nome in ("Documents", "Documentos"):
        if (casa / nome).is_dir():
            return casa / nome / "Aprendizado"
    return casa / "Aprendizado"


CABECALHO_CONFIG = "# Pasta dos estudos: a skill professor escreve aqui e a TUI `estudando` lê daqui.\n"
OPCOES_SESSAO = """
# Sessão (`estudando --sessao`): comandos dos painéis ao lado da TUI. "" desliga o painel.
# ia = "claude"         # o professor: claude, gemini, codex...
# caderno = "python3"   # rascunho e contas: python3, node, qalc, "nvim rascunho.md"...

# Tamanhos da sessão, em %: largura da TUI e proporção professor/caderno à direita.
# largura_tui = 55
# altura_ia = 60
# altura_caderno = 40
"""


def com_pasta(atual, valor):
    """O config com a linha `pasta = ...` trocada, sem mexer nas outras chaves (`ia`,
    `caderno`...). Sem config, cria um novo. Mesma regra de `src/config.rs` da TUI."""
    nova = f"pasta = {valor}"
    if not atual.strip():
        return f"{CABECALHO_CONFIG}{nova}\n{OPCOES_SESSAO}"
    linhas, trocou = [], False
    for linha in atual.splitlines():
        chave, sep, _ = linha.partition("=")
        if sep and chave.strip() == "pasta" and not trocou:
            linhas.append(nova)
            trocou = True
        else:
            linhas.append(linha)
    if not trocou:
        linhas.insert(0, nova)
    return "\n".join(linhas) + "\n"


def com_til(path):
    try:
        return "~/" + str(path.relative_to(Path.home()))
    except ValueError:
        return str(path)


def cmd_base(args):
    if args.definir:
        if any(ord(c) < 32 for c in args.definir):
            sys.exit("o caminho tem caracteres de controle")
        nova = Path(args.definir).expanduser().resolve()
        nova.mkdir(parents=True, exist_ok=True)
        cfg = arquivo_config()
        cfg.parent.mkdir(parents=True, exist_ok=True)
        atual = cfg.read_text(encoding="utf-8") if cfg.is_file() else ""
        gravar_atomico(cfg, com_pasta(atual, string_toml(com_til(nova))))
        print(f"Pasta de estudos: {nova}\nGravado em {cfg}")
        if os.environ.get("ESTUDANDO_DIR", "").strip():
            print("Atenção: $ESTUDANDO_DIR está definida e tem prioridade sobre o config.")
        return
    pasta, origem = ler_base()
    if pasta is None:
        print(
            "Pasta de estudos não configurada.\n"
            f"Sugestão: {sugestao_base()}\n"
            f"Pergunte ao aluno onde guardar e grave com: arvore.py base --definir <caminho>"
        )
        sys.exit(2)
    print(pasta)
    print(f"(de {origem}{'' if pasta.is_dir() else '; a pasta ainda não existe'})", file=sys.stderr)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("base")
    b.add_argument("--definir", metavar="CAMINHO")
    b.set_defaults(func=cmd_base)
    r = sub.add_parser("relatorio")
    r.add_argument("pasta")
    r.add_argument("--data")
    r.set_defaults(func=cmd_relatorio)
    v = sub.add_parser("revisoes")
    v.add_argument("pasta")
    v.add_argument("--data")
    v.add_argument("--dias", type=int, default=7)
    v.set_defaults(func=cmd_revisoes)
    g = sub.add_parser("registrar")
    g.add_argument("pasta")
    g.add_argument("conceito")
    g.add_argument("resultado", choices=["acertou", "parcial", "errou"])
    g.add_argument("--nivel", type=int, choices=range(5))
    g.add_argument("--data")
    g.set_defaults(func=cmd_registrar)
    s = sub.add_parser("sessao")
    s.add_argument("pasta")
    s.add_argument("acao", choices=["inicio", "etapa", "fim"])
    s.add_argument("nome", nargs="?", help="nome da etapa (revisao, pre-teste, explicacao, autoexplicacao, pratica, nota...)")
    s.add_argument("--conceito")
    s.add_argument("--plano", help='minutos por etapa: "revisao=5,pratica=15"')
    s.add_argument("--descontar", type=int, default=0, metavar="MIN", help="minutos de pausa a tirar da etapa que fecha")
    s.add_argument("--data")
    s.add_argument("--hora")
    s.set_defaults(func=cmd_sessao)
    pr = sub.add_parser("progresso")
    pr.add_argument("pasta")
    pr.add_argument("--data")
    pr.set_defaults(func=cmd_progresso)
    rs = sub.add_parser("resposta", help="registra uma resposta do aluno, classificada")
    rs.add_argument("pasta")
    rs.add_argument("resultado", choices=RESULTADOS)
    rs.add_argument("--conceito")
    rs.add_argument("--previsto", choices=PREVISOES, help="o que você previu antes da resposta")
    rs.add_argument("--sinal", action="append", choices=SINAIS, help="sinal de estado (pode repetir)")
    rs.add_argument("--nota")
    rs.add_argument("--data")
    rs.add_argument("--hora")
    rs.set_defaults(func=cmd_resposta)
    cu = sub.add_parser("curva", help="curva de aprendizado por dia e veredito de ritmo")
    cu.add_argument("pasta")
    cu.add_argument("--sessoes", type=int, help="só as últimas N sessões com respostas")
    cu.set_defaults(func=cmd_curva)
    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
