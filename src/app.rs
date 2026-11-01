//! Estado da aplicação e tratamento de teclas.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::widgets::{ListState, TableState};

use crate::analise::{hoje, itens_trilha, revisoes_ordenadas};
use crate::model::{Assunto, assinatura, carregar_assuntos};
use crate::sessao::{Aberto, Paineis, Papel};
use crate::ui;

const CHECAR_ARQUIVOS: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Aba {
    Trilha,
    Revisoes,
    Exercicios,
    Pendencias,
    Pesquisa,
    Perfil,
    Referencias,
}

impl Aba {
    pub const TODAS: [Aba; 7] = [
        Aba::Trilha,
        Aba::Revisoes,
        Aba::Exercicios,
        Aba::Pendencias,
        Aba::Pesquisa,
        Aba::Perfil,
        Aba::Referencias,
    ];

    pub fn titulo(self) -> &'static str {
        match self {
            Aba::Trilha => "Trilha",
            Aba::Revisoes => "Revisões",
            Aba::Exercicios => "Exercícios",
            Aba::Pendencias => "Pendências",
            Aba::Pesquisa => "Pesquisa",
            Aba::Perfil => "Perfil",
            Aba::Referencias => "Referências",
        }
    }

    pub fn indice(self) -> usize {
        Aba::TODAS.iter().position(|a| *a == self).unwrap_or(0)
    }
}

/// O que está aberto em tela cheia (Enter).
#[derive(Clone, PartialEq, Eq)]
pub enum Detalhe {
    Conceito(String),
    /// Lista de exercícios ou relatório de pesquisa.
    Arquivo(PathBuf),
}

/// Callouts recolhidos que o aluno escolheu abrir nas listas de exercícios.
#[derive(Clone, Copy, Default)]
pub struct Revelar {
    pub dicas: bool,
    pub gabarito: bool,
}

impl Revelar {
    pub const TUDO: Revelar = Revelar { dicas: true, gabarito: true };
}

pub struct App {
    pub raiz: PathBuf,
    pub assuntos: Vec<Assunto>,
    pub idx: usize,
    pub aba: Aba,
    pub sel_trilha: usize,
    pub sel_revisoes: usize,
    pub sel_exercicios: usize,
    pub sel_pesquisa: usize,
    pub estado_trilha: ListState,
    pub estado_revisoes: TableState,
    pub estado_exercicios: ListState,
    pub estado_pesquisa: ListState,
    /// Rolagem das abas de texto (pendências, perfil, referências).
    pub scroll: u16,
    pub detalhe: Option<Detalhe>,
    pub scroll_detalhe: u16,
    pub revelar: Revelar,
    pub ajuda: bool,
    pub mensagem: Option<String>,
    /// Professor e caderno ao lado, quando a TUI roda dentro do tmux.
    pub paineis: Option<Paineis>,
    /// Pediu um painel fora do tmux: ao sair, a TUI reabre numa sessão com ele.
    pub mudar_para_sessao: Option<Papel>,
    assinatura: u64,
    sair: bool,
}

impl App {
    pub fn new(raiz: PathBuf, assuntos: Vec<Assunto>, idx: usize, paineis: Option<Paineis>) -> Self {
        let mut app = App {
            assinatura: assinatura(&raiz),
            raiz,
            assuntos,
            idx,
            aba: Aba::Trilha,
            sel_trilha: 0,
            sel_revisoes: 0,
            sel_exercicios: 0,
            sel_pesquisa: 0,
            estado_trilha: ListState::default(),
            estado_revisoes: TableState::default(),
            estado_exercicios: ListState::default(),
            estado_pesquisa: ListState::default(),
            scroll: 0,
            detalhe: None,
            scroll_detalhe: 0,
            revelar: Revelar::default(),
            ajuda: false,
            mensagem: None,
            paineis,
            mudar_para_sessao: None,
            sair: false,
        };
        app.selecionar_fronteira();
        app
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut ultima_checagem = Instant::now();
        while !self.sair {
            terminal.draw(|f| ui::draw(f, self))?;
            if event::poll(Duration::from_millis(500))?
                && let Event::Key(k) = event::read()?
                && k.kind == KeyEventKind::Press
            {
                self.tecla(k, terminal)?;
            }
            // Recarrega sozinho quando a skill (ou você) altera as notas.
            if ultima_checagem.elapsed() >= CHECAR_ARQUIVOS {
                ultima_checagem = Instant::now();
                if assinatura(&self.raiz) != self.assinatura {
                    self.recarregar();
                    self.mensagem = Some("notas atualizadas".into());
                }
            }
        }
        Ok(())
    }

    fn assunto(&self) -> &Assunto {
        &self.assuntos[self.idx]
    }

    /// Começa no primeiro conceito da trilha que ainda não está dominado.
    fn selecionar_fronteira(&mut self) {
        let a = self.assunto();
        self.sel_trilha = itens_trilha(a)
            .iter()
            .position(|s| a.conceitos.get(*s).is_none_or(|c| c.nivel.unwrap_or(0) < 3))
            .unwrap_or(0);
    }

    fn recarregar(&mut self) {
        let atual = self.assunto().nome.clone();
        self.assuntos = carregar_assuntos(&self.raiz);
        self.assinatura = assinatura(&self.raiz);
        if self.assuntos.is_empty() {
            self.sair = true;
            return;
        }
        self.idx = self.assuntos.iter().position(|a| a.nome == atual).unwrap_or(0);
        self.limitar_selecoes();
    }

    fn tamanho(&self, aba: Aba) -> usize {
        let a = self.assunto();
        match aba {
            Aba::Trilha => itens_trilha(a).len(),
            Aba::Revisoes => a.conceitos.len(),
            Aba::Exercicios => a.exercicios.len(),
            Aba::Pesquisa => a.pesquisas.len(),
            _ => 0,
        }
    }

    fn limitar_selecoes(&mut self) {
        let limite = |v: usize, n: usize| v.min(n.saturating_sub(1));
        self.sel_trilha = limite(self.sel_trilha, self.tamanho(Aba::Trilha));
        self.sel_revisoes = limite(self.sel_revisoes, self.tamanho(Aba::Revisoes));
        self.sel_exercicios = limite(self.sel_exercicios, self.tamanho(Aba::Exercicios));
        self.sel_pesquisa = limite(self.sel_pesquisa, self.tamanho(Aba::Pesquisa));
    }

    fn selecionado(&self) -> Option<Detalhe> {
        let a = self.assunto();
        match self.aba {
            Aba::Trilha => itens_trilha(a).get(self.sel_trilha).map(|s| Detalhe::Conceito(s.to_string())),
            Aba::Revisoes => {
                revisoes_ordenadas(a, hoje()).get(self.sel_revisoes).map(|c| Detalhe::Conceito(c.slug.clone()))
            }
            Aba::Exercicios => a.exercicios.get(self.sel_exercicios).map(|e| Detalhe::Arquivo(e.path.clone())),
            Aba::Pesquisa => a.pesquisas.get(self.sel_pesquisa).map(|p| Detalhe::Arquivo(p.path.clone())),
            _ => None,
        }
    }

    /// A lista de exercícios aberta em tela cheia, se for o caso.
    pub fn lista_aberta(&self) -> bool {
        matches!(&self.detalhe, Some(Detalhe::Arquivo(p)) if self.assunto().exercicios.iter().any(|e| &e.path == p))
    }

    fn abrir(&mut self, d: Detalhe) {
        self.detalhe = Some(d);
        self.scroll_detalhe = 0;
        self.revelar = Revelar::default();
    }

    fn mudar_aba(&mut self, aba: Aba) {
        self.aba = aba;
        self.detalhe = None;
        self.scroll = 0;
    }

    fn mudar_assunto(&mut self, delta: i32) {
        let n = self.assuntos.len() as i32;
        self.idx = (self.idx as i32 + delta).rem_euclid(n) as usize;
        self.detalhe = None;
        self.scroll = 0;
        self.sel_revisoes = 0;
        self.sel_exercicios = 0;
        self.sel_pesquisa = 0;
        self.selecionar_fronteira();
    }

    fn mover(&mut self, delta: i32) {
        if self.detalhe.is_some() {
            self.scroll_detalhe = rolar(self.scroll_detalhe, delta);
            return;
        }
        let n = self.tamanho(self.aba);
        match self.aba {
            Aba::Trilha => self.sel_trilha = andar(self.sel_trilha, delta, n),
            Aba::Revisoes => self.sel_revisoes = andar(self.sel_revisoes, delta, n),
            Aba::Exercicios => self.sel_exercicios = andar(self.sel_exercicios, delta, n),
            Aba::Pesquisa => self.sel_pesquisa = andar(self.sel_pesquisa, delta, n),
            _ => self.scroll = rolar(self.scroll, delta),
        }
    }

    fn tecla(&mut self, k: KeyEvent, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.mensagem = None;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && k.code == KeyCode::Char('c') {
            self.sair = true;
            return Ok(());
        }
        if self.ajuda {
            self.ajuda = false;
            return Ok(());
        }
        let lista = self.lista_aberta();
        match k.code {
            KeyCode::Char('q') => self.sair = true,
            KeyCode::Esc => self.detalhe = None,
            KeyCode::Char('?') => self.ajuda = true,
            KeyCode::Char('r') => {
                self.recarregar();
                self.mensagem = Some("recarregado".into());
            }
            KeyCode::Char('o') => self.abrir_no_editor(terminal)?,
            KeyCode::Char('i') => self.painel(Papel::Professor),
            KeyCode::Char('c') => self.painel(Papel::Caderno),
            KeyCode::Char('p') => self.perguntar(),
            KeyCode::Char('b') => self.recolher(),
            KeyCode::Char('d') if lista && !ctrl => {
                self.revelar.dicas = !self.revelar.dicas;
                self.mensagem = Some(if self.revelar.dicas { "dicas visíveis" } else { "dicas ocultas" }.into());
            }
            KeyCode::Char('g') if lista => {
                self.revelar.gabarito = !self.revelar.gabarito;
                self.mensagem =
                    Some(if self.revelar.gabarito { "gabarito visível" } else { "gabarito oculto" }.into());
            }
            KeyCode::Tab => self.mudar_aba(Aba::TODAS[(self.aba.indice() + 1) % Aba::TODAS.len()]),
            KeyCode::BackTab => {
                self.mudar_aba(Aba::TODAS[(self.aba.indice() + Aba::TODAS.len() - 1) % Aba::TODAS.len()])
            }
            KeyCode::Char(c @ '1'..='7') => self.mudar_aba(Aba::TODAS[(c as u8 - b'1') as usize]),
            KeyCode::Char(']') | KeyCode::Char('l') | KeyCode::Right => self.mudar_assunto(1),
            KeyCode::Char('[') | KeyCode::Char('h') | KeyCode::Left => self.mudar_assunto(-1),
            KeyCode::Enter if self.detalhe.is_none() => {
                if let Some(d) = self.selecionado() {
                    self.abrir(d);
                }
            }
            KeyCode::Char('d') if ctrl => self.mover(10),
            KeyCode::Char('u') if ctrl => self.mover(-10),
            KeyCode::Down | KeyCode::Char('j') => self.mover(1),
            KeyCode::Up | KeyCode::Char('k') => self.mover(-1),
            KeyCode::PageDown => self.mover(10),
            KeyCode::PageUp => self.mover(-10),
            KeyCode::Home | KeyCode::Char('g') => self.mover(-100_000),
            KeyCode::End | KeyCode::Char('G') => self.mover(100_000),
            _ => {}
        }
        Ok(())
    }

    /// O arquivo do que está aberto ou selecionado (o que `o` abre e `p` cita).
    fn arquivo_em_foco(&self) -> Option<PathBuf> {
        let a = self.assunto();
        let caminho = |d: Option<Detalhe>| match d? {
            Detalhe::Conceito(s) => a.conceitos.get(&s).map(|c| c.path.clone()),
            Detalhe::Arquivo(p) => Some(p),
        };
        match (&self.detalhe, self.aba) {
            (Some(d), _) => caminho(Some(d.clone())),
            (None, Aba::Trilha | Aba::Revisoes | Aba::Exercicios | Aba::Pesquisa) => caminho(self.selecionado()),
            (None, Aba::Referencias) => a.referencias.as_ref().map(|r| r.0.clone()),
            (None, Aba::Perfil | Aba::Pendencias) => a.perfil.as_ref().map(|p| p.0.clone()),
        }
    }

    fn abrir_no_editor(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let Some(alvo) = self.arquivo_em_foco() else {
            self.mensagem = Some("nada para abrir aqui (conceito ainda sem nota)".into());
            return Ok(());
        };
        self.editar(&alvo, terminal)?;
        self.recarregar();
        Ok(())
    }

    /// Sem painéis: fora do tmux, sai para reabrir numa sessão com o painel pedido.
    fn sem_paineis(&mut self, papel: Papel) {
        if std::env::var_os("TMUX").is_none() {
            self.mudar_para_sessao = Some(papel);
            self.sair = true;
        } else {
            self.mensagem = Some("não achei o painel do tmux desta TUI ($TMUX_PANE)".into());
        }
    }

    fn painel(&mut self, papel: Papel) {
        let Some(paineis) = &self.paineis else {
            self.sem_paineis(papel);
            return;
        };
        self.mensagem = match paineis.focar(papel, &self.assunto().nome) {
            Ok(Aberto::Novo(_)) => Some("abrindo...".into()),
            Ok(Aberto::Pronto(_)) => None,
            Err(e) => Some(e.to_string()),
        };
    }

    fn recolher(&mut self) {
        let Some(paineis) = &self.paineis else {
            self.mensagem = Some("recolher só faz sentido na sessão: aperte i ou c para abri-la".into());
            return;
        };
        self.mensagem = match paineis.alternar_tui() {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        };
    }

    /// Começa uma pergunta ao professor sobre o que está em foco; o aluno completa por lá.
    fn perguntar(&mut self) {
        let texto = self.contexto_para_o_professor();
        let Some(paineis) = &self.paineis else {
            self.sem_paineis(Papel::Professor);
            return;
        };
        self.mensagem = match paineis.perguntar(&texto, &self.assunto().nome) {
            Ok(Aberto::Novo(_)) => Some("professor abrindo; aperte p de novo quando ele estiver pronto".into()),
            Ok(Aberto::Pronto(_)) => None,
            Err(e) => Some(e.to_string()),
        };
    }

    fn contexto_para_o_professor(&self) -> String {
        let a = self.assunto();
        let relativo = |p: &Path| p.strip_prefix(&self.raiz).unwrap_or(p).display().to_string();
        let foco = match self.detalhe.clone().or_else(|| self.selecionado()) {
            Some(Detalhe::Conceito(slug)) => match a.conceitos.get(&slug) {
                Some(c) => format!("no conceito \"{}\" ({})", c.titulo, relativo(&c.path)),
                None => format!("no conceito \"{}\" (ainda sem nota)", a.titulo_de(&slug)),
            },
            Some(Detalhe::Arquivo(p)) => format!("em {}", relativo(&p)),
            None => match self.arquivo_em_foco() {
                Some(p) => format!("na aba {} ({})", self.aba.titulo(), relativo(&p)),
                None => format!("na aba {}", self.aba.titulo()),
            },
        };
        format!("Estudando {} ({}/) no estudando, {foco}: ", a.titulo, a.nome)
    }

    fn editar(&mut self, alvo: &Path, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "nvim".into());
        let mut partes = editor.split_whitespace();
        let programa = partes.next().unwrap_or("vi");

        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        let status = Command::new(programa).args(partes).arg(alvo).status();
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        terminal.clear()?;

        if let Err(e) = status {
            self.mensagem = Some(format!("não consegui abrir {programa}: {e}"));
        }
        Ok(())
    }
}

fn andar(atual: usize, delta: i32, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    (atual as i64 + delta as i64).clamp(0, n as i64 - 1) as usize
}

/// O limite superior é aplicado no desenho, que sabe a altura do texto.
fn rolar(atual: u16, delta: i32) -> u16 {
    (atual as i32 + delta).clamp(0, u16::MAX as i32) as u16
}
