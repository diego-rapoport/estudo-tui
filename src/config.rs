//! Onde ficam os estudos. A mesma regra vale para a skill professor (`arvore.py base`):
//! `$ESTUDANDO_DIR`, senão `pasta = "..."` em `$XDG_CONFIG_HOME/estudando/config.toml`.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub enum Origem {
    Argumento,
    Variavel,
    Arquivo(PathBuf),
}

impl Origem {
    pub fn descricao(&self) -> String {
        match self {
            Origem::Argumento => "--dir".into(),
            Origem::Variavel => "$ESTUDANDO_DIR".into(),
            Origem::Arquivo(p) => p.display().to_string(),
        }
    }
}

fn casa() -> PathBuf {
    env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| casa().join(".config"))
}

pub fn arquivo_config() -> PathBuf {
    config_home().join("estudando").join("config.toml")
}

fn expandir(s: &str) -> PathBuf {
    match s.strip_prefix("~/") {
        Some(resto) => casa().join(resto),
        None if s == "~" => casa(),
        None => PathBuf::from(s),
    }
}

fn com_til(p: &Path) -> String {
    match p.strip_prefix(casa()) {
        Ok(resto) if !casa().as_os_str().is_empty() => format!("~/{}", resto.display()),
        _ => p.display().to_string(),
    }
}

/// Valor TOML simples: "básica" (com escapes), 'literal' ou sem aspas até o comentário.
fn valor_toml(bruto: &str) -> Option<String> {
    let v = bruto.trim();
    if let Some(resto) = v.strip_prefix('"') {
        let mut out = String::new();
        let mut cs = resto.chars();
        while let Some(c) = cs.next() {
            match c {
                '"' => return Some(out),
                '\\' => match cs.next()? {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    outro => out.push(outro),
                },
                _ => out.push(c),
            }
        }
        None
    } else if let Some(resto) = v.strip_prefix('\'') {
        resto.split_once('\'').map(|(s, _)| s.to_string())
    } else {
        Some(v.split('#').next()?.trim().to_string())
    }
}

/// `[tabela]` de uma linha de cabeçalho, ou `None` se a linha não é cabeçalho.
fn cabecalho(linha: &str) -> Option<&str> {
    let l = linha.trim();
    l.strip_prefix('[')?.split_once(']').map(|(t, _)| t.trim())
}

/// (tabela, chave, valor bruto) de cada `chave = valor` do texto, na ordem; a tabela do topo
/// (antes do primeiro cabeçalho) é `""`. Pula comentários; a chave pode vir entre aspas.
fn pares(texto: &str) -> impl Iterator<Item = (&str, &str, &str)> {
    let mut atual = "";
    texto.lines().filter_map(move |linha| {
        if let Some(t) = cabecalho(linha) {
            atual = t;
            return None;
        }
        if linha.trim_start().starts_with('#') {
            return None;
        }
        let (k, valor) = linha.split_once('=')?;
        let k = k.trim();
        Some((atual, k.strip_prefix('"').and_then(|k| k.strip_suffix('"')).unwrap_or(k), valor))
    })
}

/// Valor de `chave = ...` dentro da `tabela` (`""` é o topo).
fn valor_em(texto: &str, tabela: &str, chave: &str) -> Option<String> {
    pares(texto).find_map(|(t, k, valor)| (t == tabela && k == chave).then(|| valor_toml(valor)).flatten())
}

/// Erro de config: chaves de `[cadernos]` que não são o nome da pasta de nenhum assunto (o
/// caderno desse assunto cairia no `caderno` geral sem aviso). Sugere o único assunto que tem
/// a chave como palavra do nome (`c` → `programacao-c`, e não `calculo`) ou, sem esse, o único
/// que a contém.
fn erro_cadernos(texto: &str, assuntos: &[&str]) -> Option<String> {
    let unico = |filtro: &dyn Fn(&str) -> bool| match assuntos.iter().filter(|a| filtro(a)).collect::<Vec<_>>()[..] {
        [a] => Some(*a),
        _ => None,
    };
    let erros: Vec<String> = pares(texto)
        .filter(|&(t, k, _)| t == "cadernos" && !assuntos.contains(&k))
        .map(|(_, k, _)| {
            let parecido = unico(&|a| a.split(['-', '_']).any(|p| p == k)).or_else(|| unico(&|a| a.contains(k)));
            match parecido {
                Some(a) => format!("[cadernos].{k} não é um assunto (seria {a}?)"),
                None => format!("[cadernos].{k} não é um assunto"),
            }
        })
        .collect();
    (!erros.is_empty()).then(|| {
        format!("erro no config: {}; assuntos são nomes de pasta: {}", erros.join("; "), assuntos.join(", "))
    })
}

/// Valor de `chave = ...` no topo do config.
fn valor_da_chave(texto: &str, chave: &str) -> Option<String> {
    valor_em(texto, "", chave)
}

/// `pasta = ...` do config.
fn ler_arquivo(cfg: &Path) -> Option<PathBuf> {
    let texto = fs::read_to_string(cfg).ok()?;
    valor_da_chave(&texto, "pasta").filter(|v| !v.is_empty()).map(|v| expandir(&v))
}

/// Uma chave qualquer do config (`ia`, `caderno`...). `Some("")` é diferente de ausente:
/// quem chama decide o que o vazio significa.
pub fn chave(nome: &str) -> Option<String> {
    valor_da_chave(&fs::read_to_string(arquivo_config()).ok()?, nome)
}

/// Uma chave dentro de uma tabela do config (`[cadernos]`...).
pub fn chave_em(tabela: &str, nome: &str) -> Option<String> {
    valor_em(&fs::read_to_string(arquivo_config()).ok()?, tabela, nome)
}

/// Erro de config sobre os assuntos que existem (`[cadernos]` com assunto que não existe).
pub fn erro_com_assuntos(assuntos: &[&str]) -> Option<String> {
    erro_cadernos(&fs::read_to_string(arquivo_config()).ok()?, assuntos)
}

/// O editor de texto do usuário: `$VISUAL`, senão `$EDITOR`, senão `nvim`. Pode ter
/// argumentos (`code -w`).
pub fn editor() -> String {
    ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|v| env::var(v).ok())
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
        .unwrap_or_else(|| "nvim".into())
}

fn string_toml(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn resolver(argumento: Option<PathBuf>) -> Option<(PathBuf, Origem)> {
    if let Some(p) = argumento {
        return Some((p, Origem::Argumento));
    }
    if let Some(v) = env::var("ESTUDANDO_DIR").ok().filter(|v| !v.trim().is_empty()) {
        return Some((expandir(v.trim()), Origem::Variavel));
    }
    let cfg = arquivo_config();
    ler_arquivo(&cfg).map(|p| (p, Origem::Arquivo(cfg)))
}

/// `<pasta Documentos do sistema>/Aprendizado` (Documentos, Documents... conforme o idioma).
pub fn sugestao() -> PathBuf {
    let casa = casa();
    if let Ok(texto) = fs::read_to_string(config_home().join("user-dirs.dirs")) {
        for linha in texto.lines() {
            if let Some(v) = linha.strip_prefix("XDG_DOCUMENTS_DIR=") {
                let docs = PathBuf::from(v.trim().trim_matches('"').replace("$HOME", &casa.to_string_lossy()));
                if docs != casa && !docs.as_os_str().is_empty() {
                    return docs.join("Aprendizado");
                }
            }
        }
    }
    ["Documents", "Documentos"]
        .iter()
        .map(|d| casa.join(d))
        .find(|d| d.is_dir())
        .map_or_else(|| casa.join("Aprendizado"), |d| d.join("Aprendizado"))
}

/// Cria a pasta e grava o config. Devolve (pasta absoluta, arquivo de config).
pub fn definir(caminho: &str) -> io::Result<(PathBuf, PathBuf)> {
    if caminho.chars().any(char::is_control) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "o caminho tem caracteres de controle"));
    }
    let pasta = expandir(caminho);
    fs::create_dir_all(&pasta)?;
    let pasta = fs::canonicalize(&pasta)?;
    let cfg = arquivo_config();
    if let Some(dir) = cfg.parent() {
        fs::create_dir_all(dir)?;
    }
    let atual = fs::read_to_string(&cfg).unwrap_or_default();
    fs::write(&cfg, com_pasta(&atual, &string_toml(&com_til(&pasta))))?;
    Ok((pasta, cfg))
}

/// O config com a linha `pasta = ...` trocada, sem mexer nas outras chaves. Sem config, cria
/// um novo com as opções da sessão comentadas. Mesma regra de `arvore.py base --definir`.
fn com_pasta(atual: &str, valor: &str) -> String {
    let nova = format!("pasta = {valor}");
    if atual.trim().is_empty() {
        return format!("{CABECALHO}{nova}\n{OPCOES_SESSAO}");
    }
    let mut trocou = false;
    let mut no_topo = true;
    let mut linhas: Vec<String> = atual
        .lines()
        .map(|l| match l.split_once('=') {
            _ if cabecalho(l).is_some() => {
                no_topo = false;
                l.to_string()
            }
            Some((k, _)) if no_topo && k.trim() == "pasta" && !trocou => {
                trocou = true;
                nova.clone()
            }
            _ => l.to_string(),
        })
        .collect();
    if !trocou {
        linhas.insert(0, nova);
    }
    linhas.join("\n") + "\n"
}

const CABECALHO: &str = "# Pasta dos estudos: a skill professor escreve aqui e a TUI `estudando` lê daqui.\n";
const OPCOES_SESSAO: &str = "
# Sessão (`estudando --sessao`): comandos dos painéis ao lado da TUI. \"\" desliga o painel.
# ia = \"claude\"         # o professor: claude, gemini, codex...
# caderno = \"python3\"   # rascunho e contas: python3, node, qalc, \"editor rascunho.md\"...

# Tamanhos da sessão, em %: largura da TUI e proporção professor/caderno à direita.
# largura_tui = 55
# altura_ia = 60
# altura_caderno = 40

# Caderno de cada assunto (nome da pasta), no lugar do `caderno` acima. `editor` é o seu
# $VISUAL/$EDITOR (nvim se não houver). Se o assunto tem a pasta codigo/, o caderno abre nela.
# [cadernos]
# calculo = \"node\"
# programacao-c = \"editor\"
";

#[cfg(test)]
mod testes {
    use super::{com_pasta, erro_cadernos, string_toml, valor_da_chave, valor_em, valor_toml};

    #[test]
    fn valor_toml_respeita_aspas_e_comentarios() {
        assert_eq!(valor_toml(r#" "~/Estudos" # comentário"#).as_deref(), Some("~/Estudos"));
        assert_eq!(valor_toml(r#" "/a/C# e \"aspas\"" "#).as_deref(), Some(r#"/a/C# e "aspas""#));
        assert_eq!(valor_toml(" '/literal/#1' ").as_deref(), Some("/literal/#1"));
        assert_eq!(valor_toml(" /sem/aspas # x").as_deref(), Some("/sem/aspas"));
        assert_eq!(valor_toml(r#" "aberta"#), None);
    }

    #[test]
    fn string_toml_ida_e_volta() {
        for s in [r"C:\x", r#"tem "aspas""#, "acentuação/#1"] {
            assert_eq!(valor_toml(&string_toml(s)).as_deref(), Some(s));
        }
    }

    #[test]
    fn definir_pasta_preserva_as_outras_chaves() {
        let novo = com_pasta("", "\"~/A\"");
        assert_eq!(valor_da_chave(&novo, "pasta").as_deref(), Some("~/A"));
        assert_eq!(valor_da_chave(&novo, "ia"), None, "as opções da sessão vêm comentadas");

        let atual = "# meu\npasta = \"~/A\"\nia = \"gemini\"\ncaderno = \"\"\n";
        let trocado = com_pasta(atual, "\"~/B\"");
        assert_eq!(trocado, "# meu\npasta = \"~/B\"\nia = \"gemini\"\ncaderno = \"\"\n");
        assert_eq!(valor_da_chave(&trocado, "caderno").as_deref(), Some(""));

        let sem_pasta = com_pasta("ia = \"codex\"", "\"~/C\"");
        assert_eq!(sem_pasta, "pasta = \"~/C\"\nia = \"codex\"\n");

        // Um assunto chamado `pasta` em [cadernos] não é a pasta de estudos.
        let tabela = com_pasta("[cadernos]\npasta = \"node\"\n", "\"~/D\"");
        assert_eq!(tabela, "pasta = \"~/D\"\n[cadernos]\npasta = \"node\"\n");
    }

    #[test]
    fn chaves_de_tabela_nao_se_misturam_com_as_do_topo() {
        let texto = "caderno = \"python3\"\n\n[cadernos]\ncalculo = \"node\" # contas\n\"c\" = 'editor ola.c'\nia = \"x\"\n";
        assert_eq!(valor_da_chave(texto, "caderno").as_deref(), Some("python3"));
        assert_eq!(valor_da_chave(texto, "ia"), None);
        assert_eq!(valor_em(texto, "cadernos", "calculo").as_deref(), Some("node"));
        assert_eq!(valor_em(texto, "cadernos", "c").as_deref(), Some("editor ola.c"));
        assert_eq!(valor_em(texto, "cadernos", "fisica"), None);
        assert_eq!(valor_em(texto, "outra", "calculo"), None);
    }

    #[test]
    fn cadernos_de_assunto_que_nao_existe_sao_erro() {
        let assuntos = ["calculo", "programacao-c"];
        let ok = "caderno = \"node\"\n[cadernos]\ncalculo = \"node\"\n# c = \"editor\"\nprogramacao-c = \"editor\"\n";
        assert_eq!(erro_cadernos(ok, &assuntos), None);
        assert_eq!(erro_cadernos("c = \"x\"\n", &assuntos), None, "fora de [cadernos] não conta");

        let errado = "[cadernos]\nc = \"editor\"\nfisica = \"python3\"\n";
        let erro = erro_cadernos(errado, &assuntos).unwrap();
        assert!(erro.contains("[cadernos].c não é um assunto (seria programacao-c?)"), "{erro}");
        assert!(erro.contains("[cadernos].fisica não é um assunto;"), "{erro}");
        assert!(erro.ends_with("calculo, programacao-c"), "{erro}");
        let sem_palavra = erro_cadernos("[cadernos]\ncalc = \"node\"\n", &assuntos).unwrap();
        assert!(sem_palavra.contains("(seria calculo?)"), "{sem_palavra}");
    }
}
