//! `estudando`: painel no terminal para a árvore de conhecimento da skill professor.

mod analise;
mod app;
mod model;
mod ui;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use app::App;
use model::carregar_assuntos;

const AJUDA: &str = "\
estudando: progresso, revisões e próximo passo dos seus estudos

Uso: estudando [ASSUNTO] [--dir PASTA]

  ASSUNTO       abre direto nesse assunto (nome da pasta, ex.: calculo)
  -d, --dir     pasta raiz dos estudos (padrão: $ESTUDANDO_DIR ou ~/Documentos/Aprendizado)
  -h, --help    mostra esta ajuda
  -V, --version mostra a versão

Dentro da TUI, aperte ? para ver os atalhos.
";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut dir: Option<PathBuf> = None;
    let mut assunto: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{AJUDA}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("estudando {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "-d" | "--dir" => match args.next() {
                Some(d) => dir = Some(PathBuf::from(d)),
                None => {
                    eprintln!("--dir precisa de uma pasta");
                    return ExitCode::from(2);
                }
            },
            _ if a.starts_with('-') => {
                eprintln!("opção desconhecida: {a}\n\n{AJUDA}");
                return ExitCode::from(2);
            }
            _ => assunto = Some(a),
        }
    }

    let raiz = dir
        .or_else(|| env::var_os("ESTUDANDO_DIR").map(PathBuf::from))
        .unwrap_or_else(|| {
            env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Documentos/Aprendizado")
        });
    let assuntos = carregar_assuntos(&raiz);
    if assuntos.is_empty() {
        eprintln!(
            "Nenhum assunto encontrado em {}.\nCada assunto é uma pasta com _mapa.md, _perfil.md ou conceitos/.",
            raiz.display()
        );
        return ExitCode::FAILURE;
    }

    let idx = match &assunto {
        None => 0,
        Some(nome) => {
            let nome = nome.to_lowercase();
            let achado = assuntos
                .iter()
                .position(|a| a.nome.to_lowercase() == nome || a.titulo.to_lowercase() == nome)
                .or_else(|| assuntos.iter().position(|a| a.nome.to_lowercase().starts_with(&nome)));
            match achado {
                Some(i) => i,
                None => {
                    let nomes: Vec<&str> = assuntos.iter().map(|a| a.nome.as_str()).collect();
                    eprintln!("Assunto \"{nome}\" não encontrado. Disponíveis: {}", nomes.join(", "));
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    let mut terminal = ratatui::init();
    let resultado = App::new(raiz, assuntos, idx).run(&mut terminal);
    ratatui::restore();
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
