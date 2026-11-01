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

/// Valor de `chave = ...` no texto do config; ignora comentários e outras chaves.
fn valor_da_chave(texto: &str, chave: &str) -> Option<String> {
    texto.lines().find_map(|linha| {
        let (k, valor) = linha.split_once('=')?;
        if k.trim() != chave {
            return None;
        }
        valor_toml(valor)
    })
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
    let mut linhas: Vec<String> = atual
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, _)) if k.trim() == "pasta" && !trocou => {
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
# caderno = \"python3\"   # rascunho e contas: python3, node, qalc, \"nvim rascunho.md\"...

# Tamanhos da sessão, em %: largura da TUI e proporção professor/caderno à direita.
# largura_tui = 55
# altura_ia = 60
# altura_caderno = 40
";

#[cfg(test)]
mod testes {
    use super::{com_pasta, string_toml, valor_da_chave, valor_toml};

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
    }
}
