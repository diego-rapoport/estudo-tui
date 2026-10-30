#!/usr/bin/env bash
# Instala a TUI `estudando` e liga a skill professor ao Claude Code.
#
#   ./instalar.sh                 # instala tudo e pergunta onde guardar os estudos
#   ./instalar.sh --sem-skill     # só a TUI
#
# Variáveis: JOBS (paralelismo do cargo, padrão 4), CLAUDE_SKILLS_DIR (padrão ~/.claude/skills).
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
com_skill=1
[[ "${1:-}" == "--sem-skill" ]] && com_skill=0

echo "==> Compilando e instalando a TUI (cargo install)"
cargo install --path "$raiz" --locked -j "${JOBS:-4}"
bin="${CARGO_HOME:-$HOME/.cargo}/bin/estudando"

if (( com_skill )); then
    destino="${CLAUDE_SKILLS_DIR:-$HOME/.claude/skills}/professor"
    origem="$raiz/skill/professor"
    echo "==> Ligando a skill: $destino -> $origem"
    mkdir -p "$(dirname "$destino")"
    if [[ -L "$destino" ]]; then
        ln -sfn "$origem" "$destino"
    elif [[ -e "$destino" ]]; then
        echo "    $destino já existe e não é um link. Mova ou apague e rode de novo." >&2
        exit 1
    else
        ln -s "$origem" "$destino"
    fi
fi

echo "==> Pasta de estudos"
if pasta="$("$bin" --pasta 2>/dev/null)"; then
    echo "    já configurada: $pasta"
elif [[ -t 0 ]]; then
    # Sem caminho, --definir-pasta sai com erro mas imprime a sugestão.
    sugestao="$("$bin" --definir-pasta 2>&1 | sed -n 's/.*(sugestão: \(.*\))$/\1/p' || true)"
    read -r -p "    Onde guardar os estudos? [$sugestao] " escolha
    "$bin" --definir-pasta "${escolha:-$sugestao}"
else
    echo "    não configurada. Rode: estudando --definir-pasta <caminho>"
    echo "    (ou deixe a skill professor perguntar na primeira sessão)"
fi

if ! command -v estudando >/dev/null; then
    echo "==> Atenção: $(dirname "$bin") não está no PATH. Adicione-o para usar \`estudando\`."
fi
echo "==> Pronto. Rode: estudando"
