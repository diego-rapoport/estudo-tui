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

/// `pasta = ...` do config; ignora comentários e outras chaves.
fn ler_arquivo(cfg: &Path) -> Option<PathBuf> {
    let texto = fs::read_to_string(cfg).ok()?;
    texto.lines().find_map(|linha| {
        let (chave, valor) = linha.split_once('=')?;
        let valor = valor_toml(valor).filter(|v| !v.is_empty())?;
        (chave.trim() == "pasta").then(|| expandir(&valor))
    })
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
    fs::write(
        &cfg,
        format!(
            "# Pasta dos estudos: a skill professor escreve aqui e a TUI `estudando` lê daqui.\npasta = {}\n",
            string_toml(&com_til(&pasta))
        ),
    )?;
    Ok((pasta, cfg))
}

#[cfg(test)]
mod testes {
    use super::{string_toml, valor_toml};

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
}
