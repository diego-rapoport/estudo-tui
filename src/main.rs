//! `estudando`: painel no terminal para a árvore de conhecimento da skill professor.

mod analise;
mod app;
mod config;
mod model;
mod sessao;
mod ui;

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app::{Aba, App};
use model::carregar_assuntos;
use sessao::{Comandos, Paineis, Papel};

const AJUDA: &str = "\
estudando: progresso, revisões e próximo passo dos seus estudos

Uso: estudando [ASSUNTO] [--dir PASTA] [--aba N] [--sessao | --painel P] [--ia CMD] [--caderno CMD]
     estudando --pasta
     estudando --definir-pasta CAMINHO

  ASSUNTO                abre direto nesse assunto (nome da pasta, ex.: calculo)
  -d, --dir PASTA        usa esta pasta só nesta execução
      --aba N            começa na aba N (1-7)
  -s, --sessao           abre no tmux com o professor e o caderno ao lado da TUI
      --painel P         abre no tmux só com esse painel (professor ou caderno)
      --ia CMD           comando do professor (padrão: claude; \"\" desliga)
      --caderno CMD      comando do caderno (padrão: python3; \"\" desliga)
      --pasta            mostra qual pasta de estudos está em uso e de onde ela veio
      --definir-pasta C  grava C como pasta de estudos (cria se não existir)
  -h, --help             mostra esta ajuda
  -V, --version          mostra a versão

A pasta de estudos é compartilhada com a skill professor. Ela vem de $ESTUDANDO_DIR ou,
se a variável não existir, de `pasta = \"...\"` em ~/.config/estudando/config.toml.

Os comandos da sessão também podem ficar no config, como `ia = \"gemini\"` e
`caderno = \"qalc\"`, e os tamanhos em %: `largura_tui = 55`, `altura_ia = 60`,
`altura_caderno = 40`. Sem --sessao, apertar i, c ou p na TUI abre a sessão na hora.

Dentro da TUI, aperte ? para ver os atalhos.
";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut dir: Option<PathBuf> = None;
    let mut assunto: Option<String> = None;
    let mut mostrar_pasta = false;
    let mut abrir: Vec<Papel> = Vec::new();
    let mut aba: Option<Aba> = None;
    let mut comandos = Comandos::default();
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
            "-s" | "--sessao" => abrir = vec![Papel::Professor, Papel::Caderno],
            "--painel" => match args.next().as_deref().and_then(Papel::de_nome) {
                Some(papel) if !abrir.contains(&papel) => abrir.push(papel),
                Some(_) => {}
                None => {
                    eprintln!("--painel precisa de professor ou caderno");
                    return ExitCode::from(2);
                }
            },
            "--aba" => match args.next().and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=7).contains(n)) {
                Some(n) => aba = Some(Aba::TODAS[n - 1]),
                None => {
                    eprintln!("--aba precisa de um número de 1 a 7");
                    return ExitCode::from(2);
                }
            },
            "--ia" | "--caderno" => {
                let Some(cmd) = args.next() else {
                    eprintln!("{a} precisa de um comando (\"\" desliga o painel)");
                    return ExitCode::from(2);
                };
                if a == "--ia" {
                    comandos.ia = Some(cmd);
                } else {
                    comandos.caderno = Some(cmd);
                }
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

    let fora_do_tmux = env::var_os("TMUX").is_none();
    if !abrir.is_empty() && fora_do_tmux {
        return ir_para_a_sessao(&raiz, &assuntos[idx].nome, aba.unwrap_or(Aba::Trilha), &comandos, &abrir);
    }

    let paineis = Paineis::nesta_janela(&raiz, comandos.clone());
    let mut aviso = None;
    if let Some(p) = &paineis {
        for papel in &abrir {
            if let Err(e) = p.garantir(*papel, &assuntos[idx].nome) {
                aviso = Some(e.to_string());
            }
        }
    }

    let mut terminal = ratatui::init();
    let mut app = App::new(raiz, assuntos, idx, paineis);
    app.mensagem = aviso;
    if let Some(aba) = aba {
        app.aba = aba;
    }
    let resultado = app.run(&mut terminal);
    ratatui::restore();
    if let Some(p) = &app.paineis {
        p.fechar();
    }
    // Apertou i, c ou p fora do tmux: a TUI se muda para uma sessão com o painel pedido.
    if let (Some(papel), Ok(())) = (app.mudar_para_sessao, &resultado) {
        return ir_para_a_sessao(&app.raiz, &app.assuntos[app.idx].nome, app.aba, &comandos, &[papel]);
    }
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Fora do tmux, a sessão roda num tmux próprio: este processo vira o `tmux attach` e a TUI
/// de dentro (com a pasta já resolvida) abre os painéis. Só volta se der errado.
fn ir_para_a_sessao(raiz: &Path, assunto: &str, aba: Aba, comandos: &Comandos, abrir: &[Papel]) -> ExitCode {
    let raiz = raiz.canonicalize().unwrap_or_else(|_| raiz.to_path_buf());
    let exe = env::current_exe().map_or_else(|_| OsString::from("estudando"), PathBuf::into_os_string);
    let tui = vec![
        exe,
        assunto.into(),
        "--dir".into(),
        raiz.clone().into_os_string(),
        "--aba".into(),
        (aba.indice() + 1).to_string().into(),
    ];
    let erro = sessao::abrir_sessao(tui, &raiz, comandos, abrir);
    eprintln!("não consegui abrir a sessão: {erro}");
    ExitCode::FAILURE
}
