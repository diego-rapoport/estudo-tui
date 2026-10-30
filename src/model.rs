//! Leitura das notas da skill professor: frontmatter, seções, mapa e perfil.
//!
//! O formato é o descrito em `references/arvore-conhecimento.md` da skill. As regras de
//! leitura seguem as do `arvore.py` para que os dois enxerguem a mesma árvore.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

pub const NIVEIS: [&str; 5] = ["não visto", "reconhece", "em construção", "domina", "transfere"];
const IGNORAR_DIRS: [&str; 4] = [".obsidian", ".trash", "pesquisa", "exercicios"];

pub struct Conceito {
    pub slug: String,
    pub titulo: String,
    pub path: PathBuf,
    pub nivel: Option<u8>,
    pub intervalo: Option<u32>,
    pub proxima: Option<NaiveDate>,
    pub ultima: Option<NaiveDate>,
    pub prerequisitos: Vec<String>,
    pub corpo: String,
}

impl Conceito {
    pub fn secao(&self, titulo: &str) -> Option<String> {
        secao(&self.corpo, titulo)
    }

    /// Itens de lista (`- ...`) de uma seção, sem o marcador.
    pub fn itens(&self, titulo: &str) -> Vec<String> {
        self.secao(titulo).map(|s| itens_lista(&s)).unwrap_or_default()
    }
}

pub struct ItemMapa {
    pub slug: String,
    pub numero: Option<u32>,
    pub descricao: String,
}

pub enum LinhaMapa {
    Item(ItemMapa),
    Texto(String),
}

pub struct Modulo {
    pub titulo: String,
    pub linhas: Vec<LinhaMapa>,
}

impl Modulo {
    /// Módulos fora da trilha principal ("opcional", "depois").
    pub fn opcional(&self) -> bool {
        let t = self.titulo.to_lowercase();
        t.contains("opcional") || t.starts_with("depois")
    }

    pub fn itens(&self) -> impl Iterator<Item = &ItemMapa> {
        self.linhas.iter().filter_map(|l| match l {
            LinhaMapa::Item(i) => Some(i),
            LinhaMapa::Texto(_) => None,
        })
    }
}

pub struct EntradaDiario {
    pub data: Option<NaiveDate>,
    pub rotulo: String,
    pub proximo: Option<String>,
}

pub struct Assunto {
    pub nome: String,
    pub titulo: String,
    pub cabecalho: Vec<String>,
    pub modulos: Vec<Modulo>,
    pub conceitos: BTreeMap<String, Conceito>,
    pub diario: Vec<EntradaDiario>,
    pub equivocos: Vec<String>,
    /// (caminho, corpo sem frontmatter)
    pub perfil: Option<(PathBuf, String)>,
    pub referencias: Option<(PathBuf, String)>,
    /// Listas de `exercicios/`, da mais recente para a mais antiga.
    pub exercicios: Vec<Exercicios>,
    /// Relatórios de `pesquisa/`, do mais recente para o mais antigo.
    pub pesquisas: Vec<Pesquisa>,
}

pub struct Questao {
    pub numero: Option<u32>,
    /// Texto do título depois do número, ex.: "Lembrar · [[limite]]".
    pub titulo: String,
    pub respondida: bool,
}

pub struct Exercicios {
    pub path: PathBuf,
    pub titulo: String,
    pub data: Option<NaiveDate>,
    pub conceitos: Vec<String>,
    pub status: String,
    pub questoes: Vec<Questao>,
    pub corpo: String,
}

impl Exercicios {
    pub fn corrigida(&self) -> bool {
        self.status == "corrigido"
    }

    pub fn respondidas(&self) -> usize {
        self.questoes.iter().filter(|q| q.respondida).count()
    }
}

pub struct Pesquisa {
    pub path: PathBuf,
    pub titulo: String,
    pub data: Option<NaiveDate>,
    /// Relatório do agente de atualizações (`atualizacoes-*.md`).
    pub atualizacoes: bool,
    pub secoes: Vec<String>,
    pub corpo: String,
}

impl Assunto {
    pub fn item_mapa(&self, slug: &str) -> Option<(&Modulo, &ItemMapa)> {
        self.modulos
            .iter()
            .find_map(|m| m.itens().find(|i| i.slug == slug).map(|i| (m, i)))
    }

    pub fn titulo_de(&self, slug: &str) -> String {
        self.conceitos
            .get(slug)
            .map(|c| c.titulo.clone())
            .unwrap_or_else(|| {
                let mut cs = slug.chars();
                cs.next().map(|c| c.to_uppercase().chain(cs).collect::<String>()).unwrap_or_default().replace('-', " ")
            })
    }
}

enum Valor {
    Texto(String),
    Lista(Vec<String>),
}

impl Valor {
    fn texto(&self) -> &str {
        match self {
            Valor::Texto(s) => s,
            Valor::Lista(_) => "",
        }
    }
}

/// Devolve (meta, corpo). Entende `chave: valor`, listas inline e listas em bloco.
fn ler_frontmatter(texto: &str) -> (HashMap<String, Valor>, String) {
    let mut meta = HashMap::new();
    if !texto.starts_with("---") {
        return (meta, texto.to_string());
    }
    let linhas: Vec<&str> = texto.lines().collect();
    let Some(fim) = (1..linhas.len()).find(|&i| linhas[i].trim() == "---") else {
        return (meta, texto.to_string());
    };
    let mut chave: Option<String> = None;
    for linha in &linhas[1..fim] {
        if linha.trim().is_empty() {
            continue;
        }
        if let (Some(item), Some(k)) = (linha.trim_start().strip_prefix("- "), &chave) {
            let item = sem_aspas(item);
            match meta.get_mut(k) {
                Some(Valor::Lista(v)) => v.push(item),
                Some(Valor::Texto(_)) => {}
                None => {
                    meta.insert(k.clone(), Valor::Lista(vec![item]));
                }
            }
            continue;
        }
        if linha.starts_with([' ', '\t']) {
            continue;
        }
        if let Some((k, v)) = linha.split_once(':') {
            let (k, v) = (k.trim().to_string(), v.trim());
            let valor = if v.starts_with('[') && v.ends_with(']') && !v.starts_with("[[") {
                Valor::Lista(
                    v[1..v.len() - 1]
                        .split(',')
                        .map(sem_aspas)
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            } else if v.is_empty() {
                Valor::Lista(vec![])
            } else {
                Valor::Texto(sem_aspas(v))
            };
            meta.insert(k.clone(), valor);
            chave = Some(k);
        }
    }
    (meta, linhas[fim + 1..].join("\n"))
}

fn sem_aspas(s: &str) -> String {
    s.trim().trim_matches(['"', '\'']).to_string()
}

pub fn corpo_sem_frontmatter(texto: &str) -> String {
    ler_frontmatter(texto).1
}

pub fn sem_comentarios(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(i) = resto.find("<!--") {
        out.push_str(&resto[..i]);
        resto = match resto[i..].find("-->") {
            Some(j) => &resto[i + j + 3..],
            None => "",
        };
    }
    out.push_str(resto);
    out
}

/// Slugs dos links `[[alvo]]`, `[[alvo|texto]]` e `[[alvo#seção]]`.
pub fn links(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut resto = s;
    while let Some(i) = resto.find("[[") {
        let depois = &resto[i + 2..];
        let Some(j) = depois.find("]]") else { break };
        let alvo = depois[..j].split(['|', '#']).next().unwrap_or("").trim().to_lowercase();
        if !alvo.is_empty() {
            v.push(alvo);
        }
        resto = &depois[j + 2..];
    }
    v
}

pub fn data(s: &str) -> Option<NaiveDate> {
    s.get(..10).and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// Conteúdo de uma seção `## titulo`, sem comentários HTML.
pub fn secao(corpo: &str, titulo: &str) -> Option<String> {
    let alvo = titulo.to_lowercase();
    let mut dentro = false;
    let mut out = Vec::new();
    for linha in corpo.lines() {
        if let Some(h) = linha.strip_prefix("## ") {
            if dentro {
                break;
            }
            dentro = h.trim().to_lowercase() == alvo;
            continue;
        }
        if dentro {
            out.push(linha);
        }
    }
    dentro.then(|| sem_comentarios(&out.join("\n")).trim().to_string())
}

/// Itens `- ...` de um trecho; linhas de continuação são juntadas ao item anterior.
pub fn itens_lista(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for linha in s.lines() {
        let t = linha.trim();
        if let Some(r) = t.strip_prefix("- ") {
            out.push(r.trim().to_string());
        } else if !t.is_empty()
            && let Some(u) = out.last_mut()
        {
            u.push(' ');
            u.push_str(t);
        }
    }
    out
}

fn nome(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn coletar_md(dir: &Path, ignorar: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entradas) = fs::read_dir(dir) else { return };
    let mut caminhos: Vec<PathBuf> = entradas.flatten().map(|e| e.path()).collect();
    caminhos.sort();
    for p in caminhos {
        if p.is_dir() {
            if !ignorar.contains(&nome(&p).as_str()) {
                coletar_md(&p, ignorar, out);
            }
        } else if p.extension().is_some_and(|e| e == "md") {
            out.push(p);
        }
    }
}

/// Os .md de uma pasta, sem descer em subpastas.
fn md_em(dir: &Path) -> Vec<PathBuf> {
    let Ok(entradas) = fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<PathBuf> = entradas
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    v.sort();
    v
}

/// Muda sempre que algum .md da raiz (inclusive exercícios e pesquisas) é criado, apagado ou alterado.
pub fn assinatura(raiz: &Path) -> u64 {
    let mut arquivos = Vec::new();
    coletar_md(raiz, &[".obsidian", ".trash"], &mut arquivos);
    let mut h = DefaultHasher::new();
    for p in arquivos {
        p.hash(&mut h);
        if let Ok(m) = fs::metadata(&p) {
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    }
    h.finish()
}

pub fn carregar_assuntos(raiz: &Path) -> Vec<Assunto> {
    let Ok(entradas) = fs::read_dir(raiz) else { return vec![] };
    let mut dirs: Vec<PathBuf> = entradas
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !nome(p).starts_with('.'))
        .filter(|p| {
            p.join("_mapa.md").exists() || p.join("_perfil.md").exists() || p.join("conceitos").is_dir()
        })
        .collect();
    dirs.sort();
    dirs.iter().map(|d| carregar_assunto(d)).collect()
}

fn carregar_assunto(dir: &Path) -> Assunto {
    let mut arquivos = Vec::new();
    coletar_md(dir, &IGNORAR_DIRS, &mut arquivos);

    let mut conceitos = BTreeMap::new();
    for path in arquivos {
        let Ok(texto) = fs::read_to_string(&path) else { continue };
        let (meta, corpo) = ler_frontmatter(&texto);
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        let em_conceitos = rel.components().count() > 1
            && rel.components().next().is_some_and(|c| c.as_os_str() == "conceitos");
        let tipo = meta.get("tipo").map(Valor::texto).unwrap_or("");
        if tipo != "conceito" && !em_conceitos {
            continue;
        }
        let slug = path.file_stem().map(|s| s.to_string_lossy().trim().to_lowercase()).unwrap_or_default();
        let titulo = corpo
            .lines()
            .find_map(|l| l.strip_prefix("# "))
            .map(|t| t.trim().to_string())
            .unwrap_or_else(|| slug.clone());
        let campo = |k: &str| meta.get(k).map(Valor::texto).unwrap_or("").trim().to_string();
        let prerequisitos = match meta.get("prerequisitos") {
            Some(Valor::Lista(l)) => links(&l.join(" ")),
            Some(Valor::Texto(t)) => links(t),
            None => vec![],
        };
        conceitos.insert(
            slug.clone(),
            Conceito {
                nivel: campo("nivel").parse().ok(),
                intervalo: campo("intervalo").parse().ok(),
                proxima: data(&campo("proxima_revisao")),
                ultima: data(&campo("ultima_revisao")),
                prerequisitos,
                slug,
                titulo,
                path,
                corpo,
            },
        );
    }

    let ler = |arquivo: &str| {
        let p = dir.join(arquivo);
        fs::read_to_string(&p).ok().map(|t| (p, corpo_sem_frontmatter(&t)))
    };
    let mapa = ler("_mapa.md");
    let perfil = ler("_perfil.md");
    let referencias = ler("_referencias.md");

    let (titulo_mapa, cabecalho, modulos) = mapa.as_ref().map(|(_, c)| ler_mapa(c)).unwrap_or_default();
    let titulo = titulo_mapa
        .or_else(|| perfil.as_ref().and_then(|(_, c)| titulo_do_documento(c)))
        .unwrap_or_else(|| nome(dir));
    let (diario, equivocos) = perfil
        .as_ref()
        .map(|(_, c)| (ler_diario(c), ler_equivocos(c)))
        .unwrap_or_default();

    Assunto {
        nome: nome(dir),
        titulo,
        cabecalho,
        modulos,
        conceitos,
        diario,
        equivocos,
        perfil,
        referencias,
        exercicios: carregar_varios(&dir.join("exercicios"), ler_exercicios, |e| e.data),
        pesquisas: carregar_varios(&dir.join("pesquisa"), ler_pesquisa, |p| p.data),
    }
}

/// Lê os .md de uma pasta, do mais recente para o mais antigo.
fn carregar_varios<T>(
    dir: &Path,
    ler: fn(&Path) -> Option<T>,
    data: fn(&T) -> Option<NaiveDate>,
) -> Vec<T> {
    let mut v: Vec<(PathBuf, T)> = md_em(dir).into_iter().filter_map(|p| ler(&p).map(|t| (p, t))).collect();
    v.sort_by(|(pa, a), (pb, b)| data(b).cmp(&data(a)).then(pb.cmp(pa)));
    v.into_iter().map(|(_, t)| t).collect()
}

fn titulo_h1(corpo: &str) -> Option<String> {
    corpo.lines().find_map(|l| l.strip_prefix("# ")).map(|t| t.trim().to_string())
}

fn stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Primeira data AAAA-MM-DD que aparece no nome do arquivo.
fn data_no_nome(nome: &str) -> Option<NaiveDate> {
    nome.char_indices().find_map(|(i, _)| nome.get(i..).and_then(data))
}

/// Seções `## ` na ordem: (título, conteúdo).
fn secoes(corpo: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for linha in corpo.lines() {
        if let Some(h) = linha.strip_prefix("## ") {
            out.push((h.trim().to_string(), String::new()));
        } else if let Some((_, c)) = out.last_mut() {
            c.push_str(linha);
            c.push('\n');
        }
    }
    out
}

fn ler_exercicios(path: &Path) -> Option<Exercicios> {
    let texto = fs::read_to_string(path).ok()?;
    let (meta, corpo) = ler_frontmatter(&texto);
    let mut status = meta.get("status").map(Valor::texto).unwrap_or("").trim().to_lowercase();
    let mut questoes = Vec::new();
    for (h, conteudo) in secoes(&corpo) {
        if h.to_lowercase().starts_with("correção") || h.to_lowercase().starts_with("correcao") {
            // A skill troca para `corrigido` ao corrigir; a seção vale mesmo se esqueceu.
            if status.is_empty() || status == "pendente" {
                status = "corrigido".into();
            }
            continue;
        }
        if let (numero @ Some(_), titulo) = marcador(&h) {
            questoes.push(Questao { numero, titulo: titulo.to_string(), respondida: tem_resposta(&conteudo) });
        }
    }
    if status.is_empty() {
        status = "pendente".into();
    }
    let mut conceitos = match meta.get("conceitos") {
        Some(Valor::Lista(l)) => links(&l.join(" ")),
        Some(Valor::Texto(t)) => links(t),
        None => vec![],
    };
    if conceitos.is_empty() {
        for l in questoes.iter().flat_map(|q| links(&q.titulo)) {
            if !conceitos.contains(&l) {
                conceitos.push(l);
            }
        }
    }
    let nome = stem(path);
    Some(Exercicios {
        path: path.to_path_buf(),
        titulo: titulo_h1(&corpo).unwrap_or_else(|| nome.clone()),
        data: data(meta.get("data").map(Valor::texto).unwrap_or("")).or_else(|| data_no_nome(&nome)),
        conceitos,
        status,
        questoes,
        corpo,
    })
}

/// Há texto depois de "**Sua resposta:**" (fora dos callouts de dica e gabarito)?
fn tem_resposta(secao: &str) -> bool {
    let s = sem_comentarios(secao);
    let Some(i) = s.find("Sua resposta").or_else(|| s.find("sua resposta")) else { return false };
    s[i + "Sua resposta".len()..].lines().any(|l| {
        let t = l.trim().trim_matches(['*', ':']).trim();
        !t.is_empty() && !t.starts_with('>')
    })
}

fn ler_pesquisa(path: &Path) -> Option<Pesquisa> {
    let texto = fs::read_to_string(path).ok()?;
    let (meta, corpo) = ler_frontmatter(&texto);
    let nome = stem(path);
    let titulo = titulo_h1(&corpo).unwrap_or_else(|| nome.clone());
    let data_no_texto = corpo
        .lines()
        .find_map(|l| l.split_once("**Data:**").map(|(_, d)| d.trim().to_string()))
        .and_then(|d| data(&d));
    Some(Pesquisa {
        path: path.to_path_buf(),
        titulo: titulo.strip_prefix("Pesquisa:").unwrap_or(&titulo).trim().to_string(),
        data: data(meta.get("data").map(Valor::texto).unwrap_or(""))
            .or_else(|| data_no_nome(&nome))
            .or(data_no_texto),
        atualizacoes: nome.starts_with("atualizacoes"),
        secoes: secoes(&corpo).into_iter().map(|(h, _)| h).collect(),
        corpo,
    })
}

/// "# Mapa — Cálculo" → "Cálculo".
fn titulo_do_documento(corpo: &str) -> Option<String> {
    corpo.lines().find_map(|l| l.strip_prefix("# ")).map(|h| {
        h.split_once('—').map_or(h, |(_, d)| d).trim().to_string()
    })
}

fn ler_mapa(corpo: &str) -> (Option<String>, Vec<String>, Vec<Modulo>) {
    let corpo = sem_comentarios(corpo);
    let titulo = titulo_do_documento(&corpo);
    let mut cabecalho = Vec::new();
    let mut modulos: Vec<Modulo> = Vec::new();
    for linha in corpo.lines() {
        let t = linha.trim();
        if t.is_empty() || t.starts_with("# ") {
            continue;
        }
        if let Some(h) = t.strip_prefix("## ") {
            modulos.push(Modulo { titulo: h.trim().to_string(), linhas: vec![] });
            continue;
        }
        let Some(m) = modulos.last_mut() else {
            cabecalho.push(t.to_string());
            continue;
        };
        let (numero, resto) = marcador(t);
        let alvos = links(resto);
        if alvos.is_empty() {
            m.linhas.push(LinhaMapa::Texto(resto.to_string()));
        } else if alvos.len() == 1 && resto.starts_with("[[") {
            let descricao = resto
                .split_once("]]")
                .map_or("", |(_, d)| d)
                .trim()
                .trim_start_matches(['—', '–', '-', ':'])
                .trim()
                .to_string();
            m.linhas.push(LinhaMapa::Item(ItemMapa { slug: alvos[0].clone(), numero, descricao }));
        } else {
            // "- ML: [[a]], [[b]]" vira um item por link, com o rótulo como descrição.
            let rotulo = resto.split("[[").next().unwrap_or("").trim().trim_end_matches(':').trim();
            for slug in alvos {
                m.linhas.push(LinhaMapa::Item(ItemMapa { slug, numero: None, descricao: rotulo.to_string() }));
            }
        }
    }
    (titulo, cabecalho, modulos)
}

/// Separa o marcador de lista ("- ", "* " ou "12. ") do resto da linha.
fn marcador(t: &str) -> (Option<u32>, &str) {
    if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        return (None, r.trim());
    }
    let digitos = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digitos > 0
        && let Some(r) = t[digitos..].strip_prefix(". ")
    {
        return (t[..digitos].parse().ok(), r.trim());
    }
    (None, t)
}

fn ler_diario(corpo: &str) -> Vec<EntradaDiario> {
    let Some(s) = secao(corpo, "Diário") else { return vec![] };
    itens_lista(&s)
        .into_iter()
        .map(|item| {
            let (rotulo, texto) = match item.strip_prefix("**").and_then(|r| r.split_once("**")) {
                Some((r, t)) => (r.trim().to_string(), t.trim_start_matches(':').trim().to_string()),
                None => (String::new(), item.clone()),
            };
            let proximo = texto.split_once("Próximo:").map(|(_, p)| p.trim().to_string());
            EntradaDiario { data: data(&rotulo), rotulo, proximo }
        })
        .collect()
}

/// Lista logo abaixo de "Equívocos encontrados:" no perfil.
fn ler_equivocos(corpo: &str) -> Vec<String> {
    let mut linhas = corpo.lines().skip_while(|l| {
        !l.trim().to_lowercase().starts_with("equívocos encontrados")
    });
    if linhas.next().is_none() {
        return vec![];
    }
    let mut trecho = Vec::new();
    for linha in linhas {
        let t = linha.trim();
        if t.is_empty() {
            if trecho.is_empty() {
                continue;
            }
            break;
        }
        if !t.starts_with("- ") && !linha.starts_with([' ', '\t']) {
            break;
        }
        trecho.push(linha);
    }
    itens_lista(&trecho.join("\n"))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn frontmatter_listas_inline_e_em_bloco() {
        let (m, corpo) = ler_frontmatter("---\nnivel: 2\nprerequisitos: [\"[[funcao]]\", \"[[taxa]]\"]\nproxima_revisao:\n---\n# T\n");
        assert_eq!(m.get("nivel").map(Valor::texto), Some("2"));
        assert!(matches!(m.get("proxima_revisao"), Some(Valor::Lista(v)) if v.is_empty()));
        assert!(matches!(m.get("prerequisitos"), Some(Valor::Lista(v)) if v.len() == 2));
        assert_eq!(corpo, "# T");

        let (m, _) = ler_frontmatter("---\nprerequisitos:\n  - \"[[funcao]]\"\n  - \"[[limite]]\"\n---\n");
        let Some(Valor::Lista(v)) = m.get("prerequisitos") else { panic!() };
        assert_eq!(links(&v.join(" ")), ["funcao", "limite"]);
    }

    #[test]
    fn mapa_com_numeros_descricoes_e_rotulos() {
        let (titulo, cab, mods) = ler_mapa(
            "# Mapa — Cálculo\n\nObjetivo: x\n\n## Módulo 0\n1. [[a]] — desc A\n2. [[b|B]]\n\n## Extra (opcional)\n- ML: [[c]], [[d]]\nsó texto\n",
        );
        assert_eq!(titulo.as_deref(), Some("Cálculo"));
        assert_eq!(cab, ["Objetivo: x"]);
        let m0: Vec<_> = mods[0].itens().map(|i| (i.slug.as_str(), i.numero, i.descricao.as_str())).collect();
        assert_eq!(m0, [("a", Some(1), "desc A"), ("b", Some(2), "")]);
        assert!(mods[1].opcional());
        let m1: Vec<_> = mods[1].itens().map(|i| (i.slug.as_str(), i.descricao.as_str())).collect();
        assert_eq!(m1, [("c", "ML"), ("d", "ML")]);
    }

    #[test]
    fn resposta_ignora_callouts_e_marcadores() {
        assert!(!tem_resposta("Enunciado\n\n> [!check]- Gabarito\n> x\n\n**Sua resposta:**\n\n"));
        assert!(tem_resposta("Enunciado\n\n**Sua resposta:**\nÉ a taxa.\n"));
        assert!(!tem_resposta("Enunciado sem campo de resposta\n"));
    }

    #[test]
    fn diario_extrai_proximo() {
        let d = ler_diario("## Diário\n- **2026-10-30 (sessão 1)**: fez X. Próximo: estudar Y.\n");
        assert_eq!(d[0].data, NaiveDate::from_ymd_opt(2026, 10, 30));
        assert_eq!(d[0].rotulo, "2026-10-30 (sessão 1)");
        assert_eq!(d[0].proximo.as_deref(), Some("estudar Y."));
    }
}
