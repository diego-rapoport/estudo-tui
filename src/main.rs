//! `estudando`: painel no terminal para a árvore de conhecimento da skill professor.

mod analise;
mod app;
mod config;
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
     estudando --pasta
     estudando --definir-pasta CAMINHO

  ASSUNTO                abre direto nesse assunto (nome da pasta, ex.: calculo)
  -d, --dir PASTA        usa esta pasta só nesta execução
      --pasta            mostra qual pasta de estudos está em uso e de onde ela veio
      --definir-pasta C  grava C como pasta de estudos (cria se não existir)
  -h, --help             mostra esta ajuda
  -V, --version          mostra a versão

A pasta de estudos é compartilhada com a skill professor. Ela vem de $ESTUDANDO_DIR ou,
se a variável não existir, de `pasta = \"...\"` em ~/.config/estudando/config.toml.

Dentro da TUI, aperte ? para ver os atalhos.
";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut dir: Option<PathBuf> = None;
    let mut assunto: Option<String> = None;
    let mut mostrar_pasta = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--pasta" => mostrar_pasta = true,
            "--definir-pasta" => {
                let Some(caminho) = args.next() else {
                    eprintln!("--definir-pasta precisa de um caminho (sugestão: {})", config::sugestao().display());
                    return ExitCode::from(2);
                };
                return match config::definir(&caminho) {
                    Ok((pasta, cfg)) => {
                        println!("Pasta de estudos: {}\nGravado em {}", pasta.display(), cfg.display());
                        if env::var("ESTUDANDO_DIR").is_ok_and(|v| !v.trim().is_empty()) {
                            println!("Atenção: $ESTUDANDO_DIR está definida e tem prioridade sobre o config.");
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("não consegui definir a pasta {caminho}: {e}");
                        ExitCode::FAILURE
                    }
                };
            }
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

    let Some((raiz, origem)) = config::resolver(dir) else {
        eprintln!(
            "Pasta de estudos não configurada.\n\n\
             Escolha onde guardar e rode:\n  estudando --definir-pasta {}\n\n\
             A skill professor também pergunta e grava isso na primeira vez que você estudar.",
            config::sugestao().display()
        );
        return ExitCode::FAILURE;
    };
    if mostrar_pasta {
        println!("{}", raiz.display());
        eprintln!("(de {}{})", origem.descricao(), if raiz.is_dir() { "" } else { "; a pasta ainda não existe" });
        return ExitCode::SUCCESS;
    }
    let assuntos = carregar_assuntos(&raiz);
    if assuntos.is_empty() {
        eprintln!(
            "Nenhum assunto em {} (de {}).\n\
             Cada assunto é uma pasta com _mapa.md, _perfil.md ou conceitos/. Para criar o primeiro,\n\
             peça à skill professor: \"quero aprender <assunto>\".",
            raiz.display(),
            origem.descricao()
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
