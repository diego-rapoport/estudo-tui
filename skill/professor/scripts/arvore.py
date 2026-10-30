#!/usr/bin/env python3
"""Ferramentas da árvore de conhecimento da skill professor (só biblioteca padrão).

Uso:
  arvore.py base [--definir CAMINHO]   # pasta base dos estudos (compartilhada com a TUI `estudando`)
  arvore.py relatorio <pasta-do-assunto> [--data AAAA-MM-DD]
  arvore.py revisoes  <pasta-do-assunto> [--data AAAA-MM-DD] [--dias N]
  arvore.py registrar <pasta-do-assunto> <conceito> acertou|parcial|errou [--nivel N] [--data AAAA-MM-DD]

Notas de conceito: arquivos .md com `tipo: conceito` no frontmatter ou dentro de conceitos/.
"""
import argparse
import datetime as dt
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
        if any(p in IGNORAR_DIRS for p in rel.parts[:-1]):
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
        gravar_atomico(
            cfg,
            "# Pasta dos estudos: a skill professor escreve aqui e a TUI `estudando` lê daqui.\n"
            f"pasta = {string_toml(com_til(nova))}\n",
        )
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
    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
