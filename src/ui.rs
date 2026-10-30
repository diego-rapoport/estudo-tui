//! Desenho das telas.

use chrono::{Datelike, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Cell, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap};

use crate::analise::{
    LinhaTrilha, Situacao, TipoPasso, hoje, itens_trilha, listas_do_conceito, pendencias, proximos_passos, resumo,
    revisoes_ordenadas, situacao, trilha,
};
use crate::app::{Aba, App, Detalhe, Revelar};
use crate::model::{Assunto, Exercicios, NIVEIS, sem_comentarios};

const ACENTO: Color = Color::Cyan;
const APAGADO: Color = Color::DarkGray;
const SELECAO: Color = Color::Indexed(236);

const DIAS: [&str; 7] = ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"];
const MESES: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

pub fn draw(f: &mut Frame, app: &mut App) {
    let hoje = hoje();
    let [topo, meio, rodape] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)]).areas(f.area());
    desenhar_topo(f, app, topo, hoje);

    let principal = if meio.width >= 96 {
        let [esq, dir] = Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).areas(meio);
        desenhar_lateral(f, app, esq, hoje);
        dir
    } else {
        meio
    };

    match app.aba {
        Aba::Trilha => desenhar_trilha(f, app, principal, hoje),
        Aba::Revisoes => desenhar_revisoes(f, app, principal, hoje),
        Aba::Exercicios => desenhar_exercicios(f, app, principal),
        Aba::Pesquisa => desenhar_pesquisa(f, app, principal),
        Aba::Pendencias => {
            let linhas = linhas_pendencias(&app.assuntos[app.idx], hoje);
            texto_rolavel(f, principal, "Pendências", linhas, &mut app.scroll);
        }
        Aba::Perfil => {
            let linhas = documento(app.assuntos[app.idx].perfil.as_ref().map(|p| p.1.as_str()), "_perfil.md");
            texto_rolavel(f, principal, "Perfil e diário", linhas, &mut app.scroll);
        }
        Aba::Referencias => {
            let linhas =
                documento(app.assuntos[app.idx].referencias.as_ref().map(|p| p.1.as_str()), "_referencias.md");
            texto_rolavel(f, principal, "Referências", linhas, &mut app.scroll);
        }
    }

    if let Some(d) = &app.detalhe {
        let a = &app.assuntos[app.idx];
        let (titulo, linhas) = match d {
            Detalhe::Conceito(slug) => (a.titulo_de(slug), linhas_detalhe(a, slug, hoje)),
            Detalhe::Arquivo(path) => linhas_arquivo(a, path, app.revelar),
        };
        f.render_widget(Clear, principal);
        texto_rolavel(f, principal, &titulo, linhas, &mut app.scroll_detalhe);
    }

    desenhar_rodape(f, app, rodape);
    if app.ajuda {
        desenhar_ajuda(f);
    }
}

fn bloco(titulo: &str) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(APAGADO))
        .title(Span::styled(format!(" {titulo} "), Style::new().fg(ACENTO).add_modifier(Modifier::BOLD)))
}

fn negrito(cor: Color) -> Style {
    Style::new().fg(cor).add_modifier(Modifier::BOLD)
}

fn cor_situacao(s: Situacao) -> Color {
    match s {
        Situacao::Vencida(_) => Color::Red,
        Situacao::Hoje => Color::Yellow,
        Situacao::Futura(_) => Color::Green,
        Situacao::SemData => Color::Magenta,
        Situacao::Planejado => APAGADO,
    }
}

fn cor_nivel(n: Option<u8>) -> Color {
    match n {
        Some(1) => Color::Red,
        Some(2) => Color::Yellow,
        Some(3) => Color::Green,
        Some(4) => ACENTO,
        _ => APAGADO,
    }
}

fn barra(n: Option<u8>) -> String {
    let n = n.unwrap_or(0).min(4) as usize;
    format!("{}{}", "█".repeat(n), "░".repeat(4 - n))
}

fn nivel_spans(n: Option<u8>) -> Vec<Span<'static>> {
    vec![
        Span::styled(barra(n), Style::new().fg(cor_nivel(n))),
        Span::raw(format!(" {}", n.map_or("–".to_string(), |n| n.to_string()))),
    ]
}

fn data_curta(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

fn data_semana(d: NaiveDate) -> String {
    format!("{} {}", DIAS[d.weekday().num_days_from_monday() as usize], d.format("%d/%m"))
}

fn data_extenso(d: NaiveDate) -> String {
    format!(" {}, {} de {} ", DIAS[d.weekday().num_days_from_monday() as usize], d.day(), MESES[d.month0() as usize])
}

/// Corta com "…" ou completa com espaços até `w` colunas.
fn ajustar(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n > w {
        s.chars().take(w.saturating_sub(1)).chain(['…']).collect()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

/// Markdown inline: **negrito**, *itálico*, `código` e [[links]].
fn inline(s: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let (mut b, mut it, mut code) = (false, false, false);
    let estilo = |b: bool, it: bool, code: bool| {
        let mut st = base;
        if b {
            st = st.add_modifier(Modifier::BOLD);
        }
        if it {
            st = st.add_modifier(Modifier::ITALIC);
        }
        if code {
            st = st.fg(Color::Yellow);
        }
        st
    };
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), estilo(b, it, code)));
            }
        };
    }
    while i < cs.len() {
        let c = cs[i];
        let prox = cs.get(i + 1).copied();
        if c == '`' {
            flush!();
            code = !code;
            i += 1;
            continue;
        }
        if !code {
            if c == '*' && prox == Some('*') {
                flush!();
                b = !b;
                i += 2;
                continue;
            }
            if c == '*' {
                let antes = if i == 0 { ' ' } else { cs[i - 1] };
                let abre = !it && (antes.is_whitespace() || antes == '(') && prox.is_some_and(|p| !p.is_whitespace());
                let fecha = it && !antes.is_whitespace();
                if abre || fecha {
                    flush!();
                    it = !it;
                    i += 1;
                    continue;
                }
            }
            // $math$ (regra do pandoc: sem espaço logo depois de abrir nem antes de fechar).
            if c == '$'
                && prox.is_some_and(|p| !p.is_whitespace() && p != '$')
                && let Some(fim) = (i + 2..cs.len()).find(|&j| cs[j] == '$' && !cs[j - 1].is_whitespace())
            {
                flush!();
                let formula: String = cs[i + 1..fim].iter().collect();
                spans.push(Span::styled(latex(&formula), estilo(b, it, code).add_modifier(Modifier::ITALIC)));
                i = fim + 1;
                continue;
            }
            if c == '['
                && prox == Some('[')
                && let Some(fim) = (i + 2..cs.len().saturating_sub(1)).find(|&j| cs[j] == ']' && cs[j + 1] == ']')
            {
                flush!();
                let dentro: String = cs[i + 2..fim].iter().collect();
                let texto = dentro.split_once('|').map_or(dentro.as_str(), |(_, t)| t).to_string();
                spans.push(Span::styled(texto, estilo(b, it, code).fg(ACENTO).add_modifier(Modifier::UNDERLINED)));
                i = fim + 2;
                continue;
            }
        }
        buf.push(c);
        i += 1;
    }
    flush!();
    spans
}

const SIMBOLOS: [(&str, &str); 33] = [
    ("\\left", ""),
    ("\\right", ""),
    ("\\cdot", "·"),
    ("\\times", "×"),
    ("\\to", "→"),
    ("\\infty", "∞"),
    ("\\approx", "≈"),
    ("\\neq", "≠"),
    ("\\leq", "≤"),
    ("\\geq", "≥"),
    ("\\le", "≤"),
    ("\\ge", "≥"),
    ("\\int", "∫"),
    ("\\sum", "Σ"),
    ("\\Delta", "Δ"),
    ("\\delta", "δ"),
    ("\\varepsilon", "ε"),
    ("\\epsilon", "ε"),
    ("\\theta", "θ"),
    ("\\lambda", "λ"),
    ("\\alpha", "α"),
    ("\\mu", "μ"),
    ("\\sigma", "σ"),
    ("\\pi", "π"),
    ("\\prime", "′"),
    ("\\lim", "lim"),
    ("\\log", "log"),
    ("\\ln", "ln"),
    ("\\sin", "sin"),
    ("\\cos", "cos"),
    ("\\exp", "exp"),
    ("\\,", " "),
    ("\\;", " "),
];

/// LaTeX das notas em Unicode legível: `\log_2 x` → `log₂ x`, `\sqrt{x}` → `√x`, `x^2` → `x²`.
fn latex(s: &str) -> String {
    let mut t = s.to_string();
    for (de, para) in SIMBOLOS {
        // Em TeX o espaço depois de `\Delta` só termina o comando: `\Delta x` → `Δx`.
        if para.chars().count() == 1 && para.chars().all(char::is_alphabetic) {
            t = t.replace(&format!("{de} "), para);
        }
        t = t.replace(de, para);
    }
    t = comando(&t, "\\sqrt", 1, |a| {
        if a[0].chars().all(char::is_alphanumeric) { format!("√{}", a[0]) } else { format!("√({})", a[0]) }
    });
    t = comando(&t, "\\frac", 2, |a| format!("{}/{}", agrupar(&a[0]), agrupar(&a[1])));

    let cs: Vec<char> = t.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if (c == '^' || c == '_') && i + 1 < cs.len() {
            let (arg, prox) = if cs[i + 1] == '{' {
                match fecha_chave(&cs, i + 1) {
                    Some(j) => (cs[i + 2..j].iter().collect::<String>(), j + 1),
                    None => (String::new(), i + 1),
                }
            } else {
                (cs[i + 1].to_string(), i + 2)
            };
            let mapa = if c == '^' { sobrescrito } else { subscrito };
            match arg.chars().map(mapa).collect::<Option<String>>() {
                Some(m) if !arg.is_empty() => out.push_str(&m),
                _ => {
                    out.push(c);
                    out.push_str(&agrupar(&arg));
                }
            }
            i = prox;
            continue;
        }
        if !matches!(c, '{' | '}' | '\\') {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Troca `\nome{a}{b}...` (com `n` argumentos) pelo resultado de `f`.
fn comando(t: &str, nome: &str, n: usize, f: impl Fn(&[String]) -> String) -> String {
    let cs: Vec<char> = t.chars().collect();
    let nm: Vec<char> = nome.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    'fora: while i < cs.len() {
        if cs[i..].starts_with(&nm) {
            let mut j = i + nm.len();
            let mut args = Vec::new();
            for _ in 0..n {
                while cs.get(j) == Some(&' ') {
                    j += 1;
                }
                let Some(k) = (cs.get(j) == Some(&'{')).then(|| fecha_chave(&cs, j)).flatten() else {
                    out.push(cs[i]);
                    i += 1;
                    continue 'fora;
                };
                args.push(cs[j + 1..k].iter().collect());
                j = k + 1;
            }
            out.push_str(&f(&args));
            i = j;
            continue;
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

fn fecha_chave(cs: &[char], abre: usize) -> Option<usize> {
    let mut nivel = 0;
    for (k, c) in cs.iter().enumerate().skip(abre) {
        match c {
            '{' => nivel += 1,
            '}' => {
                nivel -= 1;
                if nivel == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

fn agrupar(s: &str) -> String {
    if s.chars().count() > 1 && s.contains([' ', '+', '-', '/']) { format!("({s})") } else { s.to_string() }
}

fn sobrescrito(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c as usize - '0' as usize)?,
        '+' => '⁺',
        '-' => '⁻',
        '(' => '⁽',
        ')' => '⁾',
        'n' => 'ⁿ',
        'x' => 'ˣ',
        'i' => 'ⁱ',
        _ => return None,
    })
}

fn subscrito(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => "₀₁₂₃₄₅₆₇₈₉".chars().nth(c as usize - '0' as usize)?,
        '+' => '₊',
        '-' => '₋',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'x' => 'ₓ',
        _ => return None,
    })
}

fn linha_inline(prefixo: &str, s: &str, base: Style) -> Line<'static> {
    let mut spans = vec![Span::styled(prefixo.to_string(), base)];
    spans.extend(inline(s, base));
    Line::from(spans)
}

/// Callout do Obsidian (`> [!tip]- Dica`): rótulo, cor e se está oculto.
fn callout(cabecalho: &str, revelar: Revelar) -> Option<(String, Color, Option<char>)> {
    let resto = cabecalho.strip_prefix("[!")?;
    let (tipo, depois) = resto.split_once(']')?;
    let tipo = tipo.trim().to_lowercase();
    let recolhido = depois.starts_with('-');
    let titulo = depois.trim_start_matches(['-', '+']).trim();
    let (padrao, cor, tecla, visivel) = match tipo.as_str() {
        "tip" | "hint" | "dica" => ("Dica", Color::Yellow, 'd', revelar.dicas),
        "check" | "success" | "done" | "answer" | "gabarito" => ("Gabarito", Color::Green, 'g', revelar.gabarito),
        _ => ("Nota", ACENTO, ' ', true),
    };
    let rotulo = if titulo.is_empty() { padrao.to_string() } else { titulo.to_string() };
    Some((rotulo, cor, (recolhido && !visivel).then_some(tecla)))
}

/// Markdown de bloco simples: títulos, listas, tabelas, citações, callouts e parágrafos.
/// Callouts recolhidos de dica e gabarito só aparecem se `revelar` deixar.
fn markdown(corpo: &str, revelar: Revelar) -> Vec<Line<'static>> {
    let corpo = sem_comentarios(corpo);
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut branco = true;
    // Dentro de um callout: (cor da barra, oculto).
    let mut em_callout: Option<(Color, bool)> = None;
    for linha in corpo.lines() {
        let t = linha.trim_end();
        if let Some(q) = t.trim_start().strip_prefix('>') {
            let q = q.strip_prefix(' ').unwrap_or(q);
            if let Some((rotulo, cor, oculto)) = callout(q, revelar) {
                match oculto {
                    Some(tecla) => out.push(Line::from(vec![
                        Span::styled(format!("▸ {rotulo}"), Style::new().fg(cor)),
                        Span::styled(format!("  ({tecla} mostra)"), Style::new().fg(APAGADO)),
                    ])),
                    None => out.push(Line::from(Span::styled(format!("▾ {rotulo}"), negrito(cor)))),
                }
                em_callout = Some((cor, oculto.is_some()));
            } else {
                match em_callout {
                    Some((_, true)) => {}
                    Some((cor, false)) => out.push(linha_inline_barra(q, cor)),
                    None => out.push(linha_inline_barra(q, APAGADO)),
                }
            }
            branco = false;
            continue;
        }
        em_callout = None;
        if t.trim().is_empty() {
            if !branco {
                out.push(Line::default());
                branco = true;
            }
            continue;
        }
        if t.trim() == "---" || t.trim() == "***" {
            out.push(Line::from(Span::styled("─".repeat(40), Style::new().fg(APAGADO))));
        } else if let Some(h) = t.strip_prefix("#### ") {
            out.push(Line::from(Span::styled(h.to_string(), Style::new().add_modifier(Modifier::ITALIC))));
        } else if let Some(h) = t.strip_prefix("# ") {
            out.push(Line::from(Span::styled(
                h.to_string(),
                negrito(ACENTO).add_modifier(Modifier::UNDERLINED),
            )));
        } else if let Some(h) = t.strip_prefix("## ") {
            if !branco {
                out.push(Line::default());
            }
            out.push(linha_inline("▍", h, negrito(ACENTO)));
        } else if let Some(h) = t.strip_prefix("### ") {
            out.push(Line::from(Span::styled(h.to_string(), Style::new().add_modifier(Modifier::BOLD))));
        } else if t.starts_with('|') {
            if t.chars().all(|c| "|-: ".contains(c)) {
                continue;
            }
            let celulas: Vec<&str> = t.trim().trim_matches('|').split('|').map(str::trim).collect();
            let mut spans = vec![Span::raw("  ")];
            for (k, c) in celulas.iter().enumerate() {
                if k > 0 {
                    spans.push(Span::styled(" │ ", Style::new().fg(APAGADO)));
                }
                spans.extend(inline(c, Style::new()));
            }
            out.push(Line::from(spans));
        } else {
            let recuo = t.len() - t.trim_start().len();
            let s = t.trim_start();
            match s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")) {
                Some(r) => out.push(linha_inline(&format!("{}• ", " ".repeat(recuo)), r, Style::new())),
                None => out.push(linha_inline(&" ".repeat(recuo), s, Style::new())),
            }
        }
        branco = false;
    }
    while out.last().is_some_and(|l| l.width() == 0) {
        out.pop();
    }
    out
}

fn linha_inline_barra(s: &str, cor: Color) -> Line<'static> {
    let mut spans = vec![Span::styled("│ ", Style::new().fg(cor))];
    spans.extend(inline(s, Style::new()));
    Line::from(spans)
}

fn documento(corpo: Option<&str>, arquivo: &str) -> Vec<Line<'static>> {
    match corpo {
        Some(c) => markdown(c, Revelar::TUDO),
        None => vec![Line::from(Span::styled(format!("Este assunto não tem {arquivo}."), Style::new().fg(APAGADO)))],
    }
}

/// Altura aproximada depois da quebra de linha (com folga, já que a quebra é por palavra).
fn altura_visual(linhas: &[Line], largura: u16) -> usize {
    let w = largura.max(1) as usize;
    linhas
        .iter()
        .map(|l| {
            let n = l.width();
            if n <= w { 1 } else { (n * 6 / 5).div_ceil(w) }
        })
        .sum()
}

fn texto_rolavel(f: &mut Frame, area: Rect, titulo: &str, linhas: Vec<Line<'static>>, scroll: &mut u16) {
    let mut b = bloco(titulo);
    let interno = b.inner(area);
    let max = altura_visual(&linhas, interno.width).saturating_sub(interno.height as usize);
    *scroll = (*scroll).min(max.min(u16::MAX as usize) as u16);
    if max > 0 {
        b = b.title_bottom(
            Line::from(Span::styled(format!(" {}% ", *scroll as usize * 100 / max), Style::new().fg(APAGADO)))
                .right_aligned(),
        );
    }
    f.render_widget(Paragraph::new(linhas).wrap(Wrap { trim: false }).scroll((*scroll, 0)).block(b), area);
}

fn desenhar_topo(f: &mut Frame, app: &App, area: Rect, hoje: NaiveDate) {
    let a = &app.assuntos[app.idx];
    // Em terminal estreito, as 7 abas só cabem sem o espaço em volta.
    let estreito = area.width < 110;
    let titulos: Vec<Line> = Aba::TODAS
        .iter()
        .enumerate()
        .map(|(i, aba)| {
            let nome = if estreito { aba.titulo().chars().take(5).collect() } else { aba.titulo().to_string() };
            Line::from(format!("{} {nome}", i + 1))
        })
        .collect();
    let bloco = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(APAGADO))
        .title(Line::from(vec![
            Span::styled(" estudando ", negrito(ACENTO)),
            Span::styled("· ", Style::new().fg(APAGADO)),
            Span::styled(format!("{} ", a.titulo), Style::new().add_modifier(Modifier::BOLD)),
        ]))
        .title_top(Line::from(Span::styled(data_extenso(hoje), Style::new().fg(APAGADO))).right_aligned());
    let tabs = Tabs::new(titulos)
        .select(app.aba.indice())
        .style(Style::new().fg(APAGADO))
        .highlight_style(Style::new().fg(Color::Black).bg(ACENTO).add_modifier(Modifier::BOLD))
        .divider(Span::styled("│", Style::new().fg(APAGADO)))
        .padding(if estreito { "" } else { " " }, if estreito { "" } else { " " })
        .block(bloco);
    f.render_widget(tabs, area);
}

fn desenhar_lateral(f: &mut Frame, app: &App, area: Rect, hoje: NaiveDate) {
    let a = &app.assuntos[app.idx];
    let r = resumo(a, hoje);
    let n = app.assuntos.len() as u16;
    let [assuntos, dia, niveis, _] = Layout::vertical([
        Constraint::Length(n + 2),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Fill(1),
    ])
    .areas(area);

    let itens: Vec<ListItem> = app
        .assuntos
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == app.idx {
                ListItem::new(Span::styled(format!("▶ {}", s.titulo), negrito(ACENTO)))
            } else {
                ListItem::new(format!("  {}", s.titulo))
            }
        })
        .collect();
    f.render_widget(List::new(itens).block(bloco("Assuntos")), assuntos);

    let contador = |rotulo: &str, v: usize, cor: Color| {
        Line::from(vec![
            Span::raw(format!("{rotulo:<14}")),
            Span::styled(v.to_string(), if v > 0 { negrito(cor) } else { Style::new().fg(APAGADO) }),
        ])
    };
    let sessao = match r.ultima_sessao.map(|d| (hoje - d).num_days()) {
        None => Span::styled("nenhuma ainda", Style::new().fg(APAGADO)),
        Some(0) => Span::styled("hoje", negrito(Color::Green)),
        Some(1) => Span::styled("ontem", negrito(Color::Green)),
        Some(d) => Span::styled(format!("há {d} dias"), negrito(if d > 7 { Color::Red } else { Color::Yellow })),
    };
    let linhas = vec![
        contador("Vencidas", r.vencidas, Color::Red),
        contador("Para hoje", r.hoje, Color::Yellow),
        contador("Próx. 7 dias", r.semana, Color::Green),
        contador("Sem data", r.sem_data, Color::Magenta),
        contador("Listas pend.", r.exercicios_pendentes, Color::Blue),
        Line::from(vec![Span::raw(format!("{:<14}", "Últ. sessão")), sessao]),
    ];
    f.render_widget(Paragraph::new(linhas).block(bloco("Hoje")), dia);

    let largura = 20usize;
    let cheio = if r.na_trilha == 0 { 0 } else { r.dominados * largura / r.na_trilha };
    let mut linhas = vec![
        Line::from(vec![
            Span::styled("█".repeat(cheio), Style::new().fg(Color::Green)),
            Span::styled("░".repeat(largura - cheio), Style::new().fg(APAGADO)),
        ]),
        Line::from(format!("{}/{} dominados", r.dominados, r.na_trilha)),
        Line::default(),
    ];
    for nv in (0..5).rev() {
        linhas.push(Line::from(vec![
            Span::styled(format!("{nv} "), negrito(cor_nivel(Some(nv as u8)))),
            Span::raw(format!("{:<14}", NIVEIS[nv])),
            Span::raw(r.por_nivel[nv].to_string()),
        ]));
    }
    linhas.push(Line::from(vec![
        Span::styled(format!("  {:<14}", "planejados"), Style::new().fg(APAGADO)),
        Span::styled(r.planejados.to_string(), Style::new().fg(APAGADO)),
    ]));
    f.render_widget(Paragraph::new(linhas).block(bloco("Progresso")), niveis);
}

fn linha_item(a: &Assunto, slug: &str, numero: Option<u32>, hoje: NaiveDate, largura: u16) -> Line<'static> {
    let c = a.conceitos.get(slug);
    let sit = situacao(c, hoje);
    let nivel = c.and_then(|c| c.nivel);
    // borda + símbolo de seleção + marca + número + barra/nível + situação
    let fixo = 2 + 2 + 2 + 4 + 2 + 6 + 2 + 15;
    let w_titulo = (largura as usize).saturating_sub(fixo).max(10);
    let estilo_titulo = if c.is_some() { Style::new() } else { Style::new().fg(APAGADO) };
    let mut spans = vec![
        Span::styled(if c.is_some() { "● " } else { "○ " }, Style::new().fg(cor_situacao(sit))),
        Span::styled(numero.map_or("   ".to_string(), |n| format!("{n:>2}.")), Style::new().fg(APAGADO)),
        Span::raw(" "),
        Span::styled(ajustar(&a.titulo_de(slug), w_titulo), estilo_titulo),
        Span::raw("  "),
    ];
    if c.is_some() {
        spans.extend(nivel_spans(nivel));
    } else {
        spans.push(Span::raw("      "));
    }
    spans.push(Span::raw("  "));
    spans.push(Span::styled(sit.texto(), Style::new().fg(cor_situacao(sit))));
    Line::from(spans)
}

fn info_conceito(a: &Assunto, slug: &str, hoje: NaiveDate, com_palavras: bool) -> Vec<Line<'static>> {
    let mut linhas = Vec::new();
    let mapa = a.item_mapa(slug);
    let mut titulo = vec![Span::styled(a.titulo_de(slug), Style::new().add_modifier(Modifier::BOLD))];
    if let Some((_, item)) = mapa.filter(|(_, i)| !i.descricao.is_empty()) {
        titulo.push(Span::styled(" — ", Style::new().fg(APAGADO)));
        titulo.extend(inline(&item.descricao, Style::new().fg(APAGADO)));
    }
    linhas.push(Line::from(titulo));

    let Some(c) = a.conceitos.get(slug) else {
        let modulo = mapa.map_or("o mapa".to_string(), |(m, _)| m.titulo.clone());
        linhas.push(Line::from(Span::styled(
            format!("Ainda sem nota: planejado em {modulo}."),
            Style::new().fg(APAGADO),
        )));
        return linhas;
    };

    let sit = situacao(Some(c), hoje);
    let nivel = c.nivel.map_or("sem nível".to_string(), |n| format!("nível {n} ({})", NIVEIS[n.min(4) as usize]));
    linhas.push(Line::from(vec![
        Span::styled(nivel, Style::new().fg(cor_nivel(c.nivel))),
        Span::styled(" · última ", Style::new().fg(APAGADO)),
        Span::raw(c.ultima.map_or("—".to_string(), data_curta)),
        Span::styled(" · próxima ", Style::new().fg(APAGADO)),
        Span::styled(
            c.proxima.map_or("—".to_string(), data_semana) + &format!(" ({})", sit.texto()),
            Style::new().fg(cor_situacao(sit)),
        ),
    ]));

    if !c.prerequisitos.is_empty() {
        let mut spans = vec![Span::styled("pré-requisitos: ", Style::new().fg(APAGADO))];
        for (k, p) in c.prerequisitos.iter().enumerate() {
            if k > 0 {
                spans.push(Span::raw(", "));
            }
            let pn = a.conceitos.get(p).and_then(|c| c.nivel);
            spans.push(Span::styled(a.titulo_de(p), Style::new().fg(cor_nivel(pn))));
        }
        linhas.push(Line::from(spans));
    }

    if !com_palavras {
        return linhas;
    }
    match c.secao("Em minhas palavras").filter(|s| !s.is_empty()) {
        Some(s) => linhas.push(linha_inline("“", &format!("{}”", s.replace('\n', " ")), Style::new().add_modifier(Modifier::ITALIC))),
        None => linhas.push(Line::from(Span::styled(
            "Você ainda não explicou com suas palavras.",
            Style::new().fg(Color::Magenta),
        ))),
    }
    linhas
}

fn desenhar_trilha(f: &mut Frame, app: &mut App, area: Rect, hoje: NaiveDate) {
    let a = &app.assuntos[app.idx];

    let passos: Vec<Line> = proximos_passos(a, hoje)
        .iter()
        .map(|p| {
            let (rotulo, cor) = match p.tipo {
                TipoPasso::Revisar => ("revisar", Color::Red),
                TipoPasso::Exercitar => ("exercícios", Color::Blue),
                TipoPasso::Combinado => ("combinado", ACENTO),
                TipoPasso::Reforcar => ("reforçar", Color::Yellow),
                TipoPasso::Novo => ("novo", Color::Green),
                TipoPasso::Agendar => ("agendar", Color::Magenta),
            };
            let mut spans = vec![Span::styled(format!("{rotulo:<11}"), negrito(cor))];
            spans.extend(inline(&p.texto, Style::new()));
            Line::from(spans)
        })
        .collect();
    let alt_passos = (altura_visual(&passos, area.width.saturating_sub(2)) as u16 + 2).clamp(3, (area.height / 2).max(3));
    let [lista, info, prox] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(6), Constraint::Length(alt_passos)]).areas(area);

    let mut itens: Vec<ListItem> =
        a.cabecalho.iter().map(|l| ListItem::new(linha_inline("", l, Style::new().fg(APAGADO)))).collect();
    if !itens.is_empty() {
        itens.push(ListItem::new(""));
    }
    let mut linha_sel = None;
    let mut k = 0;
    for l in trilha(a) {
        match l {
            LinhaTrilha::Modulo(t) => itens.push(ListItem::new(Span::styled(t.to_string(), negrito(ACENTO)))),
            LinhaTrilha::Texto(t) => itens.push(ListItem::new(linha_inline("    ", t, Style::new().fg(APAGADO)))),
            LinhaTrilha::Item { slug, numero } => {
                if k == app.sel_trilha {
                    linha_sel = Some(itens.len());
                }
                k += 1;
                itens.push(ListItem::new(linha_item(a, slug, numero, hoje, lista.width)));
            }
        }
    }
    app.estado_trilha.select(linha_sel);
    let list = List::new(itens)
        .block(bloco("Trilha"))
        .highlight_style(Style::new().bg(SELECAO).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, lista, &mut app.estado_trilha);

    let sel = itens_trilha(a).get(app.sel_trilha).copied();
    let linhas_info = sel.map(|s| info_conceito(a, s, hoje, true)).unwrap_or_default();
    f.render_widget(Paragraph::new(linhas_info).wrap(Wrap { trim: true }).block(bloco("Selecionado")), info);
    f.render_widget(Paragraph::new(passos).wrap(Wrap { trim: true }).block(bloco("O que estudar agora")), prox);
}

fn desenhar_revisoes(f: &mut Frame, app: &mut App, area: Rect, hoje: NaiveDate) {
    let a = &app.assuntos[app.idx];
    let lista = revisoes_ordenadas(a, hoje);
    let [tabela, historico] = Layout::vertical([Constraint::Fill(1), Constraint::Length(9)]).areas(area);

    let cabecalho = Row::new(["Conceito", "Nível", "Última", "Próxima", "Interv.", "Situação"]).style(negrito(ACENTO));
    let linhas: Vec<Row> = lista
        .iter()
        .map(|c| {
            let s = situacao(Some(c), hoje);
            Row::new(vec![
                Cell::from(c.titulo.clone()),
                Cell::from(Line::from(nivel_spans(c.nivel))),
                Cell::from(c.ultima.map_or("—".to_string(), data_curta)),
                Cell::from(c.proxima.map_or("—".to_string(), data_semana)),
                Cell::from(c.intervalo.map_or("—".to_string(), |i| format!("{i}d"))),
                Cell::from(Span::styled(s.texto(), Style::new().fg(cor_situacao(s)))),
            ])
        })
        .collect();
    let larguras = [
        Constraint::Fill(1),
        Constraint::Length(7),
        Constraint::Length(11),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Length(15),
    ];
    app.estado_revisoes.select((!lista.is_empty()).then_some(app.sel_revisoes));
    let tabela_w = Table::new(linhas, larguras)
        .header(cabecalho)
        .row_highlight_style(Style::new().bg(SELECAO).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ")
        .block(bloco("Revisões espaçadas"));
    f.render_stateful_widget(tabela_w, tabela, &mut app.estado_revisoes);

    let linhas_hist = match lista.get(app.sel_revisoes) {
        None => vec![Line::from(Span::styled("Nenhum conceito com nota ainda.", Style::new().fg(APAGADO)))],
        Some(c) => {
            let itens = c.itens("Revisões");
            if itens.is_empty() {
                vec![Line::from(Span::styled("Nenhuma revisão registrada.", Style::new().fg(APAGADO)))]
            } else {
                itens.iter().rev().map(|i| linha_inline("• ", i, Style::new())).collect()
            }
        }
    };
    let titulo = lista.get(app.sel_revisoes).map_or("Histórico".to_string(), |c| format!("Histórico: {}", c.titulo));
    f.render_widget(Paragraph::new(linhas_hist).wrap(Wrap { trim: true }).block(bloco(&titulo)), historico);
}

fn linhas_pendencias(a: &Assunto, hoje: NaiveDate) -> Vec<Line<'static>> {
    let p = pendencias(a, hoje);
    let grupos: [(&str, Color, &Vec<String>); 8] = [
        ("Revisões vencidas", Color::Red, &p.vencidas),
        ("Listas de exercícios pendentes", Color::Blue, &p.exercicios),
        ("Equívocos encontrados no diagnóstico", Color::Red, &p.equivocos),
        ("Perguntas que o professor deixou para você", ACENTO, &p.perguntas),
        ("Dúvidas abertas", Color::Yellow, &p.duvidas),
        ("Base frágil", Color::Yellow, &p.base_fragil),
        ("Notas com seções suas em branco", Color::Magenta, &p.incompletas),
        ("Sem data de revisão", Color::Magenta, &p.sem_data),
    ];
    let mut out = Vec::new();
    for (titulo, cor, itens) in grupos {
        let mut cab = vec![Span::styled(format!("▍{titulo} "), negrito(cor))];
        if itens.is_empty() {
            cab.push(Span::styled("✓ nada", Style::new().fg(Color::Green)));
        } else {
            cab.push(Span::styled(format!("({})", itens.len()), Style::new().fg(APAGADO)));
        }
        out.push(Line::from(cab));
        out.extend(itens.iter().map(|i| linha_inline("  • ", i, Style::new())));
        out.push(Line::default());
    }
    out.pop();
    out
}

fn linhas_detalhe(a: &Assunto, slug: &str, hoje: NaiveDate) -> Vec<Line<'static>> {
    let mut linhas = info_conceito(a, slug, hoje, false);
    if let Some((m, _)) = a.item_mapa(slug) {
        linhas.push(Line::from(vec![
            Span::styled("no mapa: ", Style::new().fg(APAGADO)),
            Span::raw(m.titulo.clone()),
        ]));
    }
    if let Some(c) = a.conceitos.get(slug) {
        if let Some(i) = c.intervalo {
            linhas.push(Line::from(vec![
                Span::styled("intervalo atual: ", Style::new().fg(APAGADO)),
                Span::raw(format!("{i} dia(s)")),
            ]));
        }
    }
    let listas = listas_do_conceito(a, slug);
    if !listas.is_empty() {
        let mut spans = vec![Span::styled("exercícios: ", Style::new().fg(APAGADO))];
        for (k, e) in listas.iter().enumerate() {
            if k > 0 {
                spans.push(Span::raw(", "));
            }
            spans.push(Span::raw(e.data.map_or(e.titulo.clone(), data_curta)));
            spans.push(Span::styled(format!(" ({})", e.status), Style::new().fg(cor_status(e))));
        }
        linhas.push(Line::from(spans));
    }
    if let Some(c) = a.conceitos.get(slug) {
        linhas.push(Line::from(Span::styled("─".repeat(40), Style::new().fg(APAGADO))));
        // O título já está na borda.
        let corpo: String = c.corpo.lines().filter(|l| !l.starts_with("# ")).collect::<Vec<_>>().join("\n");
        linhas.extend(markdown(&corpo, Revelar::TUDO));
    }
    linhas
}

fn cor_status(e: &Exercicios) -> Color {
    if e.corrigida() { Color::Green } else { Color::Yellow }
}

/// Lista de exercícios ou relatório de pesquisa em tela cheia.
fn linhas_arquivo(a: &Assunto, path: &std::path::Path, revelar: Revelar) -> (String, Vec<Line<'static>>) {
    if let Some(e) = a.exercicios.iter().find(|e| e.path == path) {
        let mut linhas = vec![
            Line::from(vec![
                Span::styled(e.status.clone(), negrito(cor_status(e))),
                Span::styled(format!(" · {}/{} respondidas no arquivo", e.respondidas(), e.questoes.len()), Style::new().fg(APAGADO)),
                Span::styled(
                    format!(
                        " · dicas {} · gabarito {}",
                        if revelar.dicas { "visíveis" } else { "ocultas" },
                        if revelar.gabarito { "visível" } else { "oculto" }
                    ),
                    Style::new().fg(APAGADO),
                ),
            ]),
            Line::from(Span::styled("─".repeat(40), Style::new().fg(APAGADO))),
        ];
        let corpo: String = e.corpo.lines().filter(|l| !l.starts_with("# ")).collect::<Vec<_>>().join("\n");
        linhas.extend(markdown(&com_titulos(a, &corpo), revelar));
        return (e.titulo.clone(), linhas);
    }
    if let Some(p) = a.pesquisas.iter().find(|p| p.path == path) {
        let corpo: String = p.corpo.lines().filter(|l| !l.starts_with("# ")).collect::<Vec<_>>().join("\n");
        return (p.titulo.clone(), markdown(&corpo, Revelar::TUDO));
    }
    ("Arquivo".into(), vec![Line::from(Span::styled("O arquivo não existe mais.", Style::new().fg(APAGADO)))])
}

/// `[[slug]]` → `[[slug|Título]]` para conceitos que já têm nota.
fn com_titulos(a: &Assunto, s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(i) = resto.find("[[") {
        out.push_str(&resto[..i]);
        let depois = &resto[i + 2..];
        let Some(j) = depois.find("]]") else {
            resto = &resto[i..];
            break;
        };
        let alvo = &depois[..j];
        match a.conceitos.get(&alvo.trim().to_lowercase()) {
            Some(c) if !alvo.contains(['|', '#']) => out.push_str(&format!("[[{alvo}|{}]]", c.titulo)),
            _ => out.push_str(&format!("[[{alvo}]]")),
        }
        resto = &depois[j + 2..];
    }
    out.push_str(resto);
    out
}

/// O título sem o "Exercícios — " do template; nada se o que sobra for só a data.
fn tema(e: &Exercicios) -> Option<String> {
    let resto = e.titulo.split_once('—').map_or(e.titulo.as_str(), |(_, r)| r).trim();
    let so_data = resto.chars().all(|c| c.is_ascii_digit() || c == '/' || c == '-');
    (!resto.is_empty() && !so_data).then(|| resto.to_string())
}

fn vazio(f: &mut Frame, area: Rect, titulo: &str, texto: &str) {
    let linhas = vec![Line::default(), Line::from(Span::styled(texto.to_string(), Style::new().fg(APAGADO)))];
    f.render_widget(Paragraph::new(linhas).wrap(Wrap { trim: true }).centered().block(bloco(titulo)), area);
}

fn desenhar_exercicios(f: &mut Frame, app: &mut App, area: Rect) {
    let a = &app.assuntos[app.idx];
    if a.exercicios.is_empty() {
        vazio(
            f,
            area,
            "Exercícios",
            "Nenhuma lista ainda. Peça ao professor uma lista de exercícios; ela aparece aqui sozinha.",
        );
        return;
    }
    let sel = app.sel_exercicios.min(a.exercicios.len() - 1);
    let n_questoes = a.exercicios[sel].questoes.len() as u16;
    let [lista, questoes] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length((n_questoes + 2).clamp(3, (area.height / 2).max(3)))])
            .areas(area);

    let itens: Vec<ListItem> = a
        .exercicios
        .iter()
        .map(|e| {
            let cor = cor_status(e);
            let mut spans = vec![
                Span::styled("● ", Style::new().fg(cor)),
                Span::styled(e.data.map_or("—".to_string(), data_curta), Style::new().fg(APAGADO)),
                Span::raw("  "),
                Span::styled(format!("{:<10}", e.status), Style::new().fg(cor)),
                Span::styled(
                    format!("{:>3}/{:<2} respondidas  ", e.respondidas(), e.questoes.len()),
                    Style::new().fg(APAGADO),
                ),
            ];
            if let Some(tema) = tema(e) {
                spans.push(Span::styled(tema, Style::new().add_modifier(Modifier::BOLD)));
                spans.push(Span::styled(" · ", Style::new().fg(APAGADO)));
            }
            let nomes: Vec<String> = e.conceitos.iter().map(|c| a.titulo_de(c)).collect();
            spans.push(Span::styled(nomes.join(", "), Style::new().fg(ACENTO)));
            ListItem::new(Line::from(spans))
        })
        .collect();
    app.estado_exercicios.select(Some(sel));
    let list = List::new(itens)
        .block(bloco("Listas de exercícios"))
        .highlight_style(Style::new().bg(SELECAO).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, lista, &mut app.estado_exercicios);

    let e = &a.exercicios[sel];
    let linhas: Vec<Line> = if e.questoes.is_empty() {
        vec![Line::from(Span::styled("Nenhuma questão no formato \"## 1. ...\".", Style::new().fg(APAGADO)))]
    } else {
        e.questoes
            .iter()
            .map(|q| {
                let mut spans = vec![
                    if q.respondida {
                        Span::styled("✓ ", Style::new().fg(Color::Green))
                    } else {
                        Span::styled("· ", Style::new().fg(APAGADO))
                    },
                    Span::styled(q.numero.map_or(String::new(), |n| format!("{n:>2}. ")), Style::new().fg(APAGADO)),
                ];
                spans.extend(inline(&com_titulos(a, &q.titulo), Style::new()));
                Line::from(spans)
            })
            .collect()
    };
    f.render_widget(Paragraph::new(linhas).block(bloco("Questões")), questoes);
}

fn desenhar_pesquisa(f: &mut Frame, app: &mut App, area: Rect) {
    let a = &app.assuntos[app.idx];
    if a.pesquisas.is_empty() {
        vazio(
            f,
            area,
            "Pesquisa",
            "Nenhum relatório ainda. Os agentes pesquisador e de atualizações salvam em pesquisa/.",
        );
        return;
    }
    let sel = app.sel_pesquisa.min(a.pesquisas.len() - 1);
    let n_secoes = a.pesquisas[sel].secoes.len() as u16;
    let [lista, secoes] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length((n_secoes + 2).clamp(3, (area.height / 2).max(3)))])
            .areas(area);

    let itens: Vec<ListItem> = a
        .pesquisas
        .iter()
        .map(|p| {
            let (tipo, cor) = if p.atualizacoes { ("atualizações", Color::Magenta) } else { ("pesquisa", ACENTO) };
            ListItem::new(Line::from(vec![
                Span::styled(p.data.map_or("—".to_string(), data_curta), Style::new().fg(APAGADO)),
                Span::raw("  "),
                Span::styled(format!("{tipo:<13}"), Style::new().fg(cor)),
                Span::raw(p.titulo.clone()),
            ]))
        })
        .collect();
    app.estado_pesquisa.select(Some(sel));
    let list = List::new(itens)
        .block(bloco("Relatórios de pesquisa"))
        .highlight_style(Style::new().bg(SELECAO).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, lista, &mut app.estado_pesquisa);

    let linhas: Vec<Line> =
        a.pesquisas[sel].secoes.iter().map(|s| linha_inline("• ", s, Style::new())).collect();
    f.render_widget(Paragraph::new(linhas).block(bloco("Seções")), secoes);
}

fn desenhar_rodape(f: &mut Frame, app: &App, area: Rect) {
    let linha = if let Some(m) = &app.mensagem {
        Line::from(Span::styled(format!(" {m}"), Style::new().fg(Color::Yellow)))
    } else {
        let dicas: &[(&str, &str)] = if app.lista_aberta() {
            &[("j/k", "rolar"), ("d", "dicas"), ("g", "gabarito"), ("o", "responder no editor"), ("Esc", "voltar")]
        } else if app.detalhe.is_some() {
            &[("j/k", "rolar"), ("o", "editar"), ("Esc", "voltar"), ("q", "sair")]
        } else {
            match app.aba {
                Aba::Trilha | Aba::Revisoes | Aba::Exercicios | Aba::Pesquisa => &[
                    ("j/k", "mover"),
                    ("Enter", "abrir"),
                    ("o", "editar"),
                    ("Tab", "aba"),
                    ("[ ]", "assunto"),
                    ("?", "ajuda"),
                    ("q", "sair"),
                ],
                _ => &[("j/k", "rolar"), ("o", "editar"), ("Tab", "aba"), ("[ ]", "assunto"), ("?", "ajuda"), ("q", "sair")],
            }
        };
        let mut spans = vec![Span::raw(" ")];
        for (tecla, acao) in dicas {
            spans.push(Span::styled(*tecla, negrito(ACENTO)));
            spans.push(Span::styled(format!(" {acao}  "), Style::new().fg(APAGADO)));
        }
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(linha), area);
}

fn desenhar_ajuda(f: &mut Frame) {
    let area = f.area();
    let (w, h) = (62.min(area.width), 26.min(area.height));
    let caixa = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    let tecla = |t: &str, d: &str| {
        Line::from(vec![Span::styled(format!("  {t:<14}"), negrito(ACENTO)), Span::raw(d.to_string())])
    };
    let legenda = |simbolo: &str, cor: Color, d: &str| {
        Line::from(vec![Span::styled(format!("  {simbolo:<14}"), Style::new().fg(cor)), Span::raw(d.to_string())])
    };
    let linhas = vec![
        tecla("j k ↓ ↑", "mover / rolar"),
        tecla("g G", "início / fim"),
        tecla("PgDn PgUp", "pular 10"),
        tecla("Enter", "abrir conceito, lista ou relatório"),
        tecla("o", "abrir o arquivo no $EDITOR"),
        tecla("d / g", "na lista: mostrar dicas / gabarito"),
        tecla("Tab 1-7", "trocar de aba"),
        tecla("[ ] h l", "trocar de assunto"),
        tecla("r", "recarregar (também é automático)"),
        tecla("Esc", "fechar"),
        tecla("q", "sair"),
        Line::default(),
        legenda("● vencida", Color::Red, "revisão atrasada"),
        legenda("● hoje", Color::Yellow, "revisar hoje"),
        legenda("● em dia", Color::Green, "próxima revisão no futuro"),
        legenda("● sem data", Color::Magenta, "fora do ciclo de revisão"),
        legenda("○ planejado", APAGADO, "no mapa, ainda sem nota"),
        Line::default(),
        Line::from(Span::styled(
            "  Níveis: 0 não visto · 1 reconhece · 2 em construção · 3 domina · 4 transfere",
            Style::new().fg(APAGADO),
        )),
    ];
    f.render_widget(Clear, caixa);
    f.render_widget(Paragraph::new(linhas).wrap(Wrap { trim: false }).block(bloco("Ajuda")), caixa);
}

#[cfg(test)]
mod testes {
    use super::latex;

    #[test]
    fn converte_latex_comum() {
        assert_eq!(latex(r"\log_2 x"), "log₂ x");
        assert_eq!(latex(r"\sqrt{x}"), "√x");
        assert_eq!(latex(r"\sqrt{16} = 4"), "√16 = 4");
        assert_eq!(latex(r"\sqrt{x+1}"), "√(x+1)");
        assert_eq!(latex(r"2^8 = 256"), "2⁸ = 256");
        assert_eq!(latex(r"x^{10}"), "x¹⁰");
        assert_eq!(latex(r"\frac{f(b) - f(a)}{b - a}"), "(f(b) - f(a))/(b - a)");
        assert_eq!(latex(r"\Delta y / \Delta x"), "Δy / Δx");
        assert_eq!(latex(r"e^{-kt}"), "e^(-kt)");
    }
}
