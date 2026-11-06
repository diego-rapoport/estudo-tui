//! Sessão de estudo no tmux: a TUI à esquerda e, à direita, o professor (uma IA de terminal,
//! `claude` por padrão) em cima e o caderno (REPL, calculadora, editor...) embaixo.
//!
//! Cada painel é marcado com a opção de painel `@estudando` (`tui`, `professor`, `caderno`),
//! então a TUI acha, foca, reabre e fecha os vizinhos só conversando com o tmux. Fora de um
//! tmux, `estudando --sessao` cria um servidor próprio (`tmux -L estudando`), para que as
//! opções e atalhos daqui não vazem para o tmux do usuário.
//!
//! A vida dos painéis segue a da TUI: `q` fecha os dois, Ctrl+C os mata com SIGKILL, e hooks
//! do tmux cobrem o que a TUI não vê (ela cair, ou o programa de um painel terminar: o painel
//! some e os outros ocupam o espaço).

use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

/// Servidor próprio; `$ESTUDANDO_SOCKET` troca o nome (testes, ou duas sessões ao mesmo tempo).
fn socket() -> String {
    env::var("ESTUDANDO_SOCKET").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "estudando".into())
}
const SESSAO: &str = "estudando";
const MARCA: &str = "@estudando";
const TITULO: &str = "@estudando-titulo";
/// Quando o programa do painel começou (segundos desde 1970), para o `pane-died` saber se ele
/// morreu ao abrir.
const INICIO: &str = "@estudando-inicio";
/// Um programa que termina antes disso (em segundos) deixa o painel aberto mostrando o erro.
const FALHA_AO_ABRIR: u64 = 3;
/// Largura da TUI recolhida: uma faixa que ainda dá para clicar.
pub const RECOLHIDA: u16 = 3;
/// Mata com SIGKILL o grupo de processos de cada painel do servidor e derruba o servidor.
/// Roda pelo `run-shell` do tmux, que expande os formatos (`##` vira `#`).
const ENCERRAR: &str = "for p in $(tmux -S \"#{socket_path}\" list-panes -a -F \"##{pane_pid}\"); do kill -KILL -- -$p; done \
                        2>/dev/null; tmux -S \"#{socket_path}\" kill-server";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Papel {
    Professor,
    Caderno,
}

impl Papel {
    pub fn de_nome(nome: &str) -> Option<Papel> {
        match nome {
            "professor" | "ia" => Some(Papel::Professor),
            "caderno" => Some(Papel::Caderno),
            _ => None,
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Papel::Professor => "professor",
            Papel::Caderno => "caderno",
        }
    }

    /// Chave no config.toml.
    fn chave(self) -> &'static str {
        match self {
            Papel::Professor => "ia",
            Papel::Caderno => "caderno",
        }
    }

    fn padrao(self) -> &'static str {
        match self {
            Papel::Professor => "claude",
            Papel::Caderno => "python3",
        }
    }

    fn outro(self) -> Papel {
        match self {
            Papel::Professor => Papel::Caderno,
            Papel::Caderno => Papel::Professor,
        }
    }
}

/// Comandos dados na linha de comando (`--ia`, `--caderno`); valem mais que o config.
#[derive(Clone, Default)]
pub struct Comandos {
    pub ia: Option<String>,
    pub caderno: Option<String>,
}

impl Comandos {
    /// Linha de comando > config > padrão. Vazio desliga o painel.
    fn de(&self, papel: Papel) -> String {
        let cli = match papel {
            Papel::Professor => &self.ia,
            Papel::Caderno => &self.caderno,
        };
        cli.clone()
            .or_else(|| config::chave(papel.chave()))
            .unwrap_or_else(|| papel.padrao().into())
            .trim()
            .to_string()
    }

    /// Repassa as escolhas para a TUI que vai rodar dentro da sessão nova.
    pub fn como_argumentos(&self) -> Vec<OsString> {
        let mut v = Vec::new();
        for (flag, valor) in [("--ia", &self.ia), ("--caderno", &self.caderno)] {
            if let Some(valor) = valor {
                v.push(flag.into());
                v.push(valor.into());
            }
        }
        v
    }
}

/// `tmux` no servidor atual (`$TMUX`) ou no servidor próprio da sessão.
struct Tmux {
    proprio: bool,
}

impl Tmux {
    fn comando<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut c = Command::new("tmux");
        if self.proprio {
            c.arg("-L").arg(socket());
        }
        c.args(args);
        c
    }

    fn rodar<I, S>(&self, args: I) -> io::Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let saida = self.comando(args).output()?;
        if saida.status.success() {
            Ok(String::from_utf8_lossy(&saida.stdout).trim_end().to_string())
        } else {
            let erro = String::from_utf8_lossy(&saida.stderr).trim().to_string();
            Err(io::Error::other(format!("tmux: {erro}")))
        }
    }
}

fn tem_tmux() -> bool {
    Command::new("tmux").arg("-V").output().is_ok_and(|s| s.status.success())
}

/// Tamanhos da sessão, em % (config: `largura_tui`, `altura_ia`, `altura_caderno`).
#[derive(Clone, Copy)]
struct Tamanhos {
    /// Largura da TUI na janela.
    tui: u16,
    /// Altura do professor na coluna da direita; o caderno fica com o resto.
    professor: u16,
}

impl Tamanhos {
    fn do_config() -> Tamanhos {
        let ler = |chave: &str| config::chave(chave).and_then(|v| v.trim().trim_end_matches('%').parse::<u16>().ok());
        Tamanhos::de(ler("largura_tui"), ler("altura_ia"), ler("altura_caderno"))
    }

    fn de(tui: Option<u16>, ia: Option<u16>, caderno: Option<u16>) -> Tamanhos {
        let tui = tui.unwrap_or(55);
        // As alturas são uma proporção: 60/40, 3/2 e só `altura_ia = 60` dão no mesmo.
        let professor = match (ia, caderno) {
            (Some(ia), Some(cad)) if ia + cad > 0 => ia * 100 / (ia + cad),
            (Some(ia), None) => ia,
            (None, Some(cad)) => 100u16.saturating_sub(cad),
            _ => 60,
        };
        Tamanhos { tui: tui.clamp(10, 90), professor: professor.clamp(10, 90) }
    }
}

/// Fora do tmux: cria (ou reaproveita) a sessão `estudando` no servidor próprio, com a TUI
/// rodando `tui` (que, dentro dela, abre os painéis de `abrir`) e se conecta a ela. Só volta
/// se der errado; no caminho feliz o processo vira o `tmux attach`.
pub fn abrir_sessao(tui: Vec<OsString>, raiz: &Path, comandos: &Comandos, abrir: &[Papel]) -> io::Error {
    if !tem_tmux() {
        return io::Error::other("a sessão precisa do tmux; instale-o (pacote `tmux`) e rode de novo");
    }
    let t = Tmux { proprio: true };
    if t.rodar(["has-session", "-t", SESSAO]).is_ok() {
        let marcas = t
            .rodar(["list-panes", "-s", "-t", SESSAO, "-F", &format!("#{{{MARCA}}}\t#{{pane_dead}}")])
            .unwrap_or_default();
        if marcas.lines().any(|m| m == "tui\t0") {
            return t.comando(["attach-session", "-t", SESSAO]).exec();
        }
        // Sobrou de uma sessão cuja TUI caiu (ou só restou um painel morto): recomeça do zero.
        let _ = t.rodar(["kill-session", "-t", SESSAO]);
    }

    let (largura, altura) = ratatui::crossterm::terminal::size().unwrap_or((200, 50));
    let mut nova: Vec<OsString> = ["new-session", "-d", "-P", "-F", "#{pane_id}", "-s", SESSAO, "-x"].map(OsString::from).into();
    nova.extend([largura.to_string().into(), "-y".into(), altura.to_string().into(), "-c".into()]);
    nova.push(raiz.into());
    nova.push("--".into());
    nova.extend(tui);
    nova.extend(comandos.como_argumentos());
    for papel in abrir {
        nova.extend(["--painel".into(), papel.nome().into()]);
    }
    let id_tui = match t.rodar(&nova) {
        Ok(id) => id,
        Err(e) => return e,
    };
    if let Err(e) = t.rodar(opcoes_do_servidor(&id_tui)) {
        let _ = t.rodar(["kill-server"]);
        return e;
    }
    t.comando(["attach-session", "-t", SESSAO]).exec()
}

/// Só no servidor próprio: mouse, rótulos nos painéis, Alt+setas e uma linha de ajuda.
fn opcoes_do_servidor(tui: &str) -> Vec<String> {
    let mut opcoes: Vec<Vec<String>> = [
        ["set", "-g", "mouse", "on"].as_slice(),
        &["set", "-s", "escape-time", "10"],
        &["set", "-s", "extended-keys", "on"],
        &["set", "-g", "focus-events", "on"],
        &["set", "-g", "status-style", "bg=default,fg=colour244"],
        &["set", "-g", "status-left", ""],
        &["set", "-g", "window-status-format", ""],
        &["set", "-g", "window-status-current-format", ""],
        &["set", "-g", "status-right-length", "120"],
        &["set", "-g", "status-right", " Alt+setas: trocar de painel · Alt+b: recolher a TUI · Alt+z: ampliar · Alt+q: sair de tudo · Ctrl-b d: sair sem fechar "],
        &["set", "-g", "pane-border-status", "top"],
        &["set", "-g", "pane-border-format", " #{?@estudando-titulo,#{@estudando-titulo},#{pane_title}} "],
        &["set", "-g", "pane-active-border-style", "fg=cyan"],
        &["set", "-g", "remain-on-exit-format", " não abriu (veja o erro acima): na TUI, i ou c tenta de novo · aqui, Ctrl+C fecha "],
        &["bind", "-n", "M-Left", "select-pane", "-L"],
        &["bind", "-n", "M-Right", "select-pane", "-R"],
        &["bind", "-n", "M-Up", "select-pane", "-U"],
        &["bind", "-n", "M-Down", "select-pane", "-D"],
        &["bind", "-n", "M-z", "resize-pane", "-Z"],
        // De qualquer painel: a própria TUI recolhe ou expande (tecla b).
        &["bind", "-n", "M-b", "send-keys", "-t", tui, "b"],
        // Saída garantida, mesmo com a TUI travada ou morta.
        &["bind", "-n", "M-q", "run-shell", ENCERRAR],
        // Ctrl+C continua indo para o claude e o caderno, que precisam dele. Num painel que
        // não abriu, fecha só ele; na TUI (fora do editor do `o`), mata tudo.
        &[
            "bind",
            "-n",
            "C-c",
            "if",
            "-F",
            "#{&&:#{==:#{@estudando},tui},#{||:#{pane_dead},#{==:#{pane_current_command},estudando}}}",
            &format!("run-shell '{ENCERRAR}'"),
            &format!("if -F '#{{pane_dead}}' '{}' 'send-keys C-c'", fechar_painel(tui)),
        ],
    ]
    .iter()
    .map(|o| o.iter().map(|s| s.to_string()).collect())
    .collect();
    // Sem isso o tmux reduz as cores de 24 bits do claude/gemini para 256.
    if env::var("COLORTERM").is_ok_and(|c| c == "truecolor" || c == "24bit") {
        opcoes.push(["set", "-sa", "terminal-features", ",*:RGB"].map(String::from).into());
    }
    opcoes.join(&[";".to_string()][..])
}

/// Fecha o painel atual. Se era ele que estava em foco, o foco vai para o vizinho da coluna da
/// direita, se sobrou um, ou para a TUI, que sem vizinhos já ocupa a janela toda.
fn fechar_painel(tui: &str) -> String {
    format!("kill-pane ; select-pane -t {tui} ; select-pane -R")
}

fn agora() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0).to_string()
}

/// A janela do tmux onde a TUI roda e os painéis que ela abriu ao lado.
pub struct Paineis {
    tmux: Tmux,
    tui: String,
    raiz: PathBuf,
    comandos: Comandos,
    tamanhos: Tamanhos,
}

/// O que aconteceu ao pedir um painel.
pub enum Aberto {
    /// Já estava rodando.
    Pronto(String),
    /// Acabou de abrir (ou reabrir); o programa ainda está subindo.
    Novo(String),
}

impl Paineis {
    /// Só existe dentro do tmux. Marca o painel atual como o da TUI, a menos que ele já seja o
    /// professor ou o caderno de outra TUI (alguém rodou `estudando` lá dentro) ou que a janela
    /// já tenha uma TUI viva: tomar a marca deixaria os painéis dela órfãos. O erro diz o porquê.
    pub fn nesta_janela(raiz: &Path, comandos: Comandos) -> Result<Paineis, String> {
        let sem_tmux = || "não achei o painel do tmux desta TUI ($TMUX_PANE)".to_string();
        env::var_os("TMUX").ok_or_else(sem_tmux)?;
        let tui = env::var("TMUX_PANE").map_err(|_| sem_tmux())?;
        let tmux = Tmux { proprio: false };
        let lista = tmux
            .rodar(["list-panes", "-t", &tui, "-F", &format!("#{{pane_id}}\t#{{{MARCA}}}\t#{{pane_dead}}")])
            .map_err(|e| e.to_string())?;
        let paineis: Vec<Vec<&str>> = lista.lines().map(|l| l.split('\t').collect()).collect();
        if let Some(marca) = paineis.iter().find(|c| c[0] == tui).map(|c| c[1])
            && Papel::de_nome(marca).is_some()
        {
            return Err(format!("este painel é o {marca} de outra TUI; os painéis ficam com ela"));
        }
        if let Some(c) = paineis.iter().find(|c| c[0] != tui && c[1] == "tui" && c[2] != "1") {
            return Err(format!("esta janela já tem uma TUI com painéis (painel {})", c[0]));
        }
        tmux.rodar(["set", "-p", "-t", &tui, MARCA, "tui", ";", "set", "-p", "-t", &tui, TITULO, "estudando"])
            .map_err(|e| e.to_string())?;
        let raiz = raiz.canonicalize().unwrap_or_else(|_| raiz.to_path_buf());
        Ok(Paineis { tmux, tui, raiz, comandos, tamanhos: Tamanhos::do_config() })
    }

    /// (id, vivo) do painel com esse papel nesta janela.
    fn achar(&self, papel: Papel) -> Option<(String, bool)> {
        let lista = self
            .tmux
            .rodar(["list-panes", "-t", &self.tui, "-F", &format!("#{{pane_id}}\t#{{{MARCA}}}\t#{{pane_dead}}")])
            .ok()?;
        lista.lines().find_map(|l| {
            let mut campos = l.split('\t');
            let (id, marca, morto) = (campos.next()?, campos.next()?, campos.next()?);
            (marca == papel.nome()).then(|| (id.to_string(), morto != "1"))
        })
    }

    /// Garante o painel aberto e vivo, sem tirar o foco da TUI.
    pub fn garantir(&self, papel: Papel, assunto: &str) -> io::Result<Aberto> {
        let raiz = self.raiz.to_string_lossy();
        // O respawn-pane reaproveita o comando, mas não o -c e o -e da criação.
        let ambiente = [
            "-c".into(),
            raiz.to_string(),
            "-e".into(),
            format!("ESTUDANDO_DIR={raiz}"),
            "-e".into(),
            format!("ESTUDANDO_ASSUNTO={assunto}"),
        ];
        if let Some((id, vivo)) = self.achar(papel) {
            if vivo {
                return Ok(Aberto::Pronto(id));
            }
            let inicio = [";".into(), "set".into(), "-p".into(), "-t".into(), id.clone(), INICIO.into(), agora()];
            self.tmux.rodar(["respawn-pane".into(), "-t".into(), id.clone()].into_iter().chain(ambiente).chain(inicio))?;
            return Ok(Aberto::Novo(id));
        }
        let comando = self.comandos.de(papel);
        if comando.is_empty() {
            return Err(io::Error::other(format!(
                "o painel {} está desligado (`{}` vazio no config ou na linha de comando)",
                papel.nome(),
                papel.chave()
            )));
        }
        // Coluna da direita: professor em cima, caderno embaixo. Se o vizinho já existe,
        // divide ele; senão, divide a TUI.
        let Tamanhos { tui, professor } = self.tamanhos;
        let vizinho = self.achar(papel.outro()).map(|(id, _)| id);
        let (alvo, divisao, tamanho) = match (papel, &vizinho) {
            (Papel::Professor, Some(v)) => (v, ["-v", "-b"].as_slice(), professor),
            (Papel::Caderno, Some(v)) => (v, ["-v"].as_slice(), 100 - professor),
            (_, None) => (&self.tui, ["-h"].as_slice(), 100 - tui),
        };
        let titulo = format!("{} · {comando}", papel.nome());
        let mut args: Vec<String> = vec!["split-window".into(), "-t".into(), alvo.into()];
        args.extend(divisao.iter().map(|s| s.to_string()));
        args.extend(["-l".into(), format!("{tamanho}%")]);
        args.extend(ambiente);
        args.extend(["-P".into(), "-F".into(), "#{pane_id}".into()]);
        args.push(comando);
        // Na mesma fila de comandos, antes que um programa que termina na hora feche o
        // painel: sem -d o painel novo vira o "atual" para os set-option seguintes.
        let inicio = agora();
        for opcao in [["remain-on-exit", "on"], [MARCA, papel.nome()], [TITULO, &titulo], [INICIO, &inicio]] {
            args.extend([";".into(), "set".into(), "-p".into(), opcao[0].into(), opcao[1].into()]);
        }
        // Quando o programa termina, o painel fecha e os outros ocupam o espaço: o vizinho da
        // direita fica com a coluna toda, ou a TUI com a janela toda (ver `fechar_painel`).
        // Só fica aberto, mostrando o erro, se o programa morreu ao abrir (comando errado).
        let ao_morrer = format!(
            "if -F '#{{e|<:#{{e|-:#{{pane_dead_time}},#{{{INICIO}}}}},{FALHA_AO_ABRIR}}}' '' \"if -F '#{{pane_active}}' '{}' kill-pane\"",
            fechar_painel(&self.tui)
        );
        args.extend([";".into(), "set-hook".into(), "-p".into(), "pane-died".into(), ao_morrer]);
        args.extend([";".into(), "select-pane".into(), "-t".into(), self.tui.clone()]);
        let id = self.tmux.rodar(&args)?;
        // Se a TUI cair sem fechar os painéis, o tmux fecha (senão o professor ocupa a janela
        // toda). Um índice por papel, para que um painel já fechado não impeça o outro.
        let hook = format!("pane-exited[{}]", papel as u8);
        self.tmux.rodar(["set-hook", "-p", "-t", &self.tui, &hook, &format!("kill-pane -t {id}")])?;
        Ok(Aberto::Novo(id))
    }

    /// Leva o foco para o painel, abrindo-o se preciso.
    pub fn focar(&self, papel: Papel, assunto: &str) -> io::Result<Aberto> {
        let aberto = self.garantir(papel, assunto)?;
        let (Aberto::Pronto(id) | Aberto::Novo(id)) = &aberto;
        self.tmux.rodar(["select-pane", "-t", id])?;
        Ok(aberto)
    }

    /// Digita o texto no professor (sem Enter) e leva o foco para lá, para o aluno completar
    /// a pergunta. Se o professor acabou de abrir, não digita: o programa ainda está subindo.
    pub fn perguntar(&self, texto: &str, assunto: &str) -> io::Result<Aberto> {
        let aberto = self.focar(Papel::Professor, assunto)?;
        if let Aberto::Pronto(id) = &aberto {
            self.tmux.rodar(["send-keys", "-t", id, "-l", "--", texto])?;
        }
        Ok(aberto)
    }

    /// Recolhe a TUI numa faixa estreita (o professor e o caderno ficam com a largura) ou
    /// a devolve ao tamanho do config. Devolve se ela ficou recolhida.
    pub fn alternar_tui(&self) -> io::Result<bool> {
        let largura: u16 = self.tmux.rodar(["display", "-p", "-t", &self.tui, "#{pane_width}"])?.parse().unwrap_or(0);
        if largura <= RECOLHIDA + 1 {
            let tamanho = format!("{}%", self.tamanhos.tui);
            self.tmux.rodar(["resize-pane", "-t", &self.tui, "-x", &tamanho, ";", "select-pane", "-t", &self.tui])?;
            return Ok(false);
        }
        let Some((vizinho, _)) = self.achar(Papel::Professor).or_else(|| self.achar(Papel::Caderno)) else {
            return Err(io::Error::other("nada ao lado para ocupar o espaço: abra o professor (i) ou o caderno (c)"));
        };
        let recolhida = RECOLHIDA.to_string();
        self.tmux.rodar(["resize-pane", "-t", &self.tui, "-x", &recolhida, ";", "select-pane", "-t", &vizinho])?;
        Ok(true)
    }

    /// Fecha os painéis abertos pela TUI (ao sair dela). O programa de cada um recebe SIGHUP.
    pub fn fechar(&self) {
        for papel in [Papel::Professor, Papel::Caderno] {
            if let Some((id, _)) = self.achar(papel) {
                let _ = self.tmux.rodar(["kill-pane", "-t", &id]);
            }
        }
        let tui = &self.tui;
        let _ = self.tmux.rodar([
            "set", "-p", "-u", "-t", tui, MARCA, ";", "set", "-p", "-u", "-t", tui, TITULO, ";", "set-hook", "-p",
            "-u", "-t", tui, "pane-exited",
        ]);
    }

    /// Ctrl+C: mata com SIGKILL o grupo de processos de cada painel (o programa e o que ele
    /// abriu), para sair mesmo com um deles travado ou ignorando o SIGHUP, e depois fecha.
    pub fn encerrar(&self) {
        for papel in [Papel::Professor, Papel::Caderno] {
            let Some((id, true)) = self.achar(papel) else { continue };
            if let Ok(pid) = self.tmux.rodar(["display", "-p", "-t", &id, "#{pane_pid}"])
                && !pid.is_empty()
            {
                let _ = Command::new("kill").args(["-KILL", "--", &format!("-{pid}")]).output();
            }
        }
        self.fechar();
    }
}

/// Se a TUI sair por um pânico, o desenrolar da pilha ainda fecha os painéis.
impl Drop for Paineis {
    fn drop(&mut self) {
        self.fechar();
    }
}

#[cfg(test)]
mod testes {
    use super::Tamanhos;

    #[test]
    fn tamanhos_sao_proporcoes_com_limites() {
        let t = |tui, ia, cad| {
            let t = Tamanhos::de(tui, ia, cad);
            (t.tui, t.professor)
        };
        assert_eq!(t(None, None, None), (55, 60));
        assert_eq!(t(Some(70), Some(3), Some(2)), (70, 60));
        assert_eq!(t(None, Some(60), Some(40)), (55, 60));
        assert_eq!(t(None, None, Some(25)), (55, 75));
        assert_eq!(t(Some(100), Some(0), Some(0)), (90, 60));
        assert_eq!(t(Some(0), Some(5), None), (10, 10));
    }
}
