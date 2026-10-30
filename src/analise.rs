//! O que a árvore diz: situação das revisões, base frágil, pendências e próximo passo.

use std::collections::HashSet;

use chrono::{Local, NaiveDate};

use crate::model::{Assunto, Conceito, Exercicios, LinhaMapa};

pub fn hoje() -> NaiveDate {
    Local::now().date_naive()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Situacao {
    /// Está no mapa, mas ainda não tem nota.
    Planejado,
    SemData,
    Vencida(i64),
    Hoje,
    Futura(i64),
}

impl Situacao {
    pub fn texto(self) -> String {
        match self {
            Situacao::Planejado => "planejado".into(),
            Situacao::SemData => "sem data".into(),
            Situacao::Vencida(d) => format!("vencida há {d}d"),
            Situacao::Hoje => "revisar hoje".into(),
            Situacao::Futura(1) => "rev. amanhã".into(),
            Situacao::Futura(d) => format!("rev. em {d}d"),
        }
    }

    fn ordem(self) -> (u8, i64) {
        match self {
            Situacao::Vencida(d) => (0, -d),
            Situacao::Hoje => (1, 0),
            Situacao::Futura(d) => (2, d),
            Situacao::SemData => (3, 0),
            Situacao::Planejado => (4, 0),
        }
    }
}

pub fn situacao(c: Option<&Conceito>, hoje: NaiveDate) -> Situacao {
    let Some(c) = c else { return Situacao::Planejado };
    let Some(p) = c.proxima else { return Situacao::SemData };
    match (p - hoje).num_days() {
        d if d < 0 => Situacao::Vencida(-d),
        0 => Situacao::Hoje,
        d => Situacao::Futura(d),
    }
}

pub enum LinhaTrilha<'a> {
    Modulo(&'a str),
    Texto(&'a str),
    Item { slug: &'a str, numero: Option<u32> },
}

/// O mapa na ordem, seguido dos conceitos que têm nota mas não estão no mapa.
pub fn trilha(a: &Assunto) -> Vec<LinhaTrilha<'_>> {
    let mut out = Vec::new();
    let mut vistos = HashSet::new();
    for m in &a.modulos {
        out.push(LinhaTrilha::Modulo(&m.titulo));
        for l in &m.linhas {
            match l {
                LinhaMapa::Item(i) if vistos.insert(i.slug.as_str()) => {
                    out.push(LinhaTrilha::Item { slug: &i.slug, numero: i.numero })
                }
                LinhaMapa::Item(_) => {}
                LinhaMapa::Texto(t) => out.push(LinhaTrilha::Texto(t)),
            }
        }
    }
    let fora: Vec<&String> = a.conceitos.keys().filter(|s| !vistos.contains(s.as_str())).collect();
    if !fora.is_empty() {
        out.push(LinhaTrilha::Modulo("Fora do mapa"));
        out.extend(fora.into_iter().map(|s| LinhaTrilha::Item { slug: s, numero: None }));
    }
    out
}

/// Só os itens selecionáveis da trilha, na ordem em que aparecem.
pub fn itens_trilha(a: &Assunto) -> Vec<&str> {
    trilha(a)
        .into_iter()
        .filter_map(|l| match l {
            LinhaTrilha::Item { slug, .. } => Some(slug),
            _ => None,
        })
        .collect()
}

/// Itens da trilha principal (sem módulos opcionais), na ordem do mapa.
fn trilha_principal(a: &Assunto) -> Vec<&str> {
    let mut vistos = HashSet::new();
    a.modulos
        .iter()
        .filter(|m| !m.opcional())
        .flat_map(|m| m.itens())
        .map(|i| i.slug.as_str())
        .filter(|s| vistos.insert(*s))
        .collect()
}

pub fn revisoes_ordenadas(a: &Assunto, hoje: NaiveDate) -> Vec<&Conceito> {
    let mut v: Vec<&Conceito> = a.conceitos.values().collect();
    v.sort_by_key(|c| (situacao(Some(c), hoje).ordem(), c.slug.clone()));
    v
}

pub struct Resumo {
    pub vencidas: usize,
    pub hoje: usize,
    pub semana: usize,
    pub sem_data: usize,
    pub ultima_sessao: Option<NaiveDate>,
    pub na_trilha: usize,
    pub dominados: usize,
    pub planejados: usize,
    pub por_nivel: [usize; 5],
    pub exercicios_pendentes: usize,
}

pub fn resumo(a: &Assunto, hoje: NaiveDate) -> Resumo {
    let mut r = Resumo {
        vencidas: 0,
        hoje: 0,
        semana: 0,
        sem_data: 0,
        ultima_sessao: None,
        na_trilha: 0,
        dominados: 0,
        planejados: 0,
        por_nivel: [0; 5],
        exercicios_pendentes: a.exercicios.iter().filter(|e| !e.corrigida()).count(),
    };
    for c in a.conceitos.values() {
        match situacao(Some(c), hoje) {
            Situacao::Vencida(_) => r.vencidas += 1,
            Situacao::Hoje => r.hoje += 1,
            Situacao::Futura(d) if d <= 7 => r.semana += 1,
            Situacao::SemData => r.sem_data += 1,
            _ => {}
        }
        r.por_nivel[c.nivel.unwrap_or(0).min(4) as usize] += 1;
    }
    for slug in trilha_principal(a) {
        r.na_trilha += 1;
        match a.conceitos.get(slug) {
            Some(c) if c.nivel.unwrap_or(0) >= 3 => r.dominados += 1,
            Some(_) => {}
            None => r.planejados += 1,
        }
    }
    r.ultima_sessao = a
        .diario
        .iter()
        .filter_map(|e| e.data)
        .chain(a.conceitos.values().filter_map(|c| c.ultima))
        .max();
    r
}

/// Mesma regra do `arvore.py relatorio`: conceito de nível ≥ 2 apoiado em pré-requisito
/// sem nota ou com nível abaixo de min(nível, 3).
pub fn base_fragil(a: &Assunto) -> Vec<String> {
    let mut out = Vec::new();
    for c in a.conceitos.values() {
        let Some(nv) = c.nivel.filter(|&n| n >= 2) else { continue };
        for pre in &c.prerequisitos {
            match a.conceitos.get(pre).map(|p| p.nivel) {
                None => out.push(format!("**{}** (nível {nv}) depende de [[{pre}]], que não tem nota", c.titulo)),
                Some(Some(pnv)) if pnv < nv.min(3) => {
                    out.push(format!("**{}** (nível {nv}) depende de [[{pre}]] (nível {pnv})", c.titulo))
                }
                _ => {}
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
pub enum TipoPasso {
    Revisar,
    Exercitar,
    Combinado,
    Reforcar,
    Novo,
    Agendar,
}

pub struct Passo {
    pub tipo: TipoPasso,
    pub texto: String,
}

/// "**Exercícios — 30/10/2026**: 2/6 respondidas no arquivo"
pub fn descrever_lista(e: &Exercicios) -> String {
    let n = e.questoes.len();
    if n == 0 {
        format!("**{}**", e.titulo)
    } else {
        format!("**{}**: {}/{n} respondidas no arquivo", e.titulo, e.respondidas())
    }
}

/// Listas de exercícios que cobrem um conceito, da mais recente para a mais antiga.
pub fn listas_do_conceito<'a>(a: &'a Assunto, slug: &str) -> Vec<&'a Exercicios> {
    a.exercicios.iter().filter(|e| e.conceitos.iter().any(|c| c == slug)).collect()
}

/// Prioridade: revisões vencidas → listas de exercícios pendentes → o que ficou combinado
/// no diário → conceitos em construção → próximo conceito novo do mapa → conceitos fora do
/// ciclo de revisão.
pub fn proximos_passos(a: &Assunto, hoje: NaiveDate) -> Vec<Passo> {
    let mut passos = Vec::new();
    let mut sem_data = Vec::new();
    for c in revisoes_ordenadas(a, hoje) {
        let texto = match situacao(Some(c), hoje) {
            Situacao::Vencida(d) => format!("**{}**, vencida há {d} dia(s)", c.titulo),
            Situacao::Hoje => format!("**{}**, vence hoje", c.titulo),
            Situacao::SemData => {
                sem_data.push(c.titulo.clone());
                continue;
            }
            _ => continue,
        };
        passos.push(Passo { tipo: TipoPasso::Revisar, texto });
    }

    // A mais antiga primeiro: lista esquecida vale mais do que a de hoje.
    for e in a.exercicios.iter().rev().filter(|e| !e.corrigida()) {
        passos.push(Passo { tipo: TipoPasso::Exercitar, texto: descrever_lista(e) });
    }

    if let Some(e) = a.diario.iter().rev().find(|e| e.proximo.is_some())
        && let Some(p) = &e.proximo
    {
        passos.push(Passo { tipo: TipoPasso::Combinado, texto: format!("{p} *(diário, {})*", e.rotulo) });
    }

    let principal = trilha_principal(a);
    let em_construcao: Vec<String> = itens_trilha(a)
        .into_iter()
        .filter_map(|s| a.conceitos.get(s))
        .filter(|c| c.nivel.unwrap_or(0) < 3)
        .map(|c| format!("**{}** ({})", c.titulo, c.nivel.map_or("?".into(), |n| n.to_string())))
        .collect();
    if !em_construcao.is_empty() {
        passos.push(Passo {
            tipo: TipoPasso::Reforcar,
            texto: format!("{}, meta: nível 3", em_construcao.join(", ")),
        });
    }

    if let Some(pos) = principal.iter().position(|s| !a.conceitos.contains_key(*s)) {
        let slug = principal[pos];
        let descricao = a.item_mapa(slug).map(|(_, i)| i.descricao.as_str()).unwrap_or("");
        let mut texto = format!("**{}**", a.titulo_de(slug));
        if !descricao.is_empty() {
            texto.push_str(&format!(": {descricao}"));
        }
        let frageis: Vec<String> = principal[..pos]
            .iter()
            .filter(|s| a.conceitos.get(**s).is_none_or(|c| c.nivel.unwrap_or(0) < 2))
            .map(|s| a.titulo_de(s))
            .collect();
        if !frageis.is_empty() {
            texto.push_str(&format!(" · antes, firme a base: {}", frageis.join(", ")));
        }
        passos.push(Passo { tipo: TipoPasso::Novo, texto });
    }

    if !sem_data.is_empty() {
        passos.push(Passo {
            tipo: TipoPasso::Agendar,
            texto: format!("{} sem data de revisão; registre uma revisão para entrar no ciclo", sem_data.join(", ")),
        });
    }
    passos
}

pub struct Pendencias {
    pub vencidas: Vec<String>,
    pub exercicios: Vec<String>,
    pub equivocos: Vec<String>,
    pub perguntas: Vec<String>,
    pub duvidas: Vec<String>,
    pub base_fragil: Vec<String>,
    pub incompletas: Vec<String>,
    pub sem_data: Vec<String>,
}

const SECOES_DO_ALUNO: [(&str, &str); 3] = [
    ("Em minhas palavras", "suas palavras"),
    ("Exemplo que eu entendo", "exemplo"),
    ("Por que importa", "por que importa"),
];

pub fn pendencias(a: &Assunto, hoje: NaiveDate) -> Pendencias {
    let mut p = Pendencias {
        vencidas: Vec::new(),
        exercicios: a.exercicios.iter().filter(|e| !e.corrigida()).map(descrever_lista).collect(),
        equivocos: a.equivocos.clone(),
        perguntas: Vec::new(),
        duvidas: Vec::new(),
        base_fragil: base_fragil(a),
        incompletas: Vec::new(),
        sem_data: Vec::new(),
    };
    for c in revisoes_ordenadas(a, hoje) {
        match situacao(Some(c), hoje) {
            s @ (Situacao::Vencida(_) | Situacao::Hoje) => p.vencidas.push(format!("**{}**: {}", c.titulo, s.texto())),
            Situacao::SemData => p.sem_data.push(format!("**{}**", c.titulo)),
            _ => {}
        }
    }
    for c in a.conceitos.values() {
        // A pergunta mais recente que o professor deixou nas observações.
        if let Some(obs) = c.itens("Observações do professor").iter().rev().find(|o| o.contains("Pergunta"))
            && let Some(i) = obs.find("Pergunta")
        {
            p.perguntas.push(format!("**{}**: {}", c.titulo, &obs[i..]));
        }
        for d in c.itens("Dúvidas abertas") {
            p.duvidas.push(format!("**{}**: {d}", c.titulo));
        }
        let faltando: Vec<&str> = SECOES_DO_ALUNO
            .iter()
            .filter(|(secao, _)| c.secao(secao).is_none_or(|s| s.is_empty()))
            .map(|(_, rotulo)| *rotulo)
            .collect();
        if !faltando.is_empty() {
            p.incompletas.push(format!("**{}**: falta {}", c.titulo, faltando.join(", ")));
        }
    }
    p
}
