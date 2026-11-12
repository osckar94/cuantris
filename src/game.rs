use crate::pieces;
use macroquad::rand::gen_range;
use std::collections::VecDeque;

pub const W: i32 = 10;
pub const H: i32 = 22;
pub const HIDDEN: i32 = 2;
pub const GARBAGE: u8 = 8;
const LOCK_DELAY: f32 = 0.5;
const MAX_RESETS: u32 = 15;
/// Cuantas filas de bloques puede atravesar una pieza TUNEL.
pub const TUNNEL_DEPTH: i32 = 4;

pub fn rand_int(n: i32) -> i32 {
    let r = (gen_range(0.0f32, 1.0f32) * n as f32) as i32;
    r.max(0).min(n - 1)
}

fn coin() -> bool {
    gen_range(0.0f32, 1.0f32) < 0.5
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Facil,
    Normal,
    Experto,
}

impl Mode {
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Facil => "FACIL",
            Mode::Normal => "NORMAL",
            Mode::Experto => "EXPERTO",
        }
    }
    pub fn start_level(&self) -> u32 {
        match self {
            Mode::Facil => 1,
            Mode::Normal => 4,
            Mode::Experto => 8,
        }
    }
    fn superpos_base(&self) -> f32 {
        match self {
            Mode::Facil => 0.08,
            Mode::Normal => 0.12,
            Mode::Experto => 0.18,
        }
    }
    fn superpos_cap(&self) -> f32 {
        match self {
            Mode::Facil => 0.25,
            Mode::Normal => 0.35,
            Mode::Experto => 0.45,
        }
    }
    fn entangle_prob(&self) -> f32 {
        match self {
            Mode::Facil => 0.12,
            Mode::Normal => 0.15,
            Mode::Experto => 0.18,
        }
    }
    fn tunnel_prob(&self) -> f32 {
        match self {
            Mode::Facil => 0.11,
            Mode::Normal => 0.09,
            Mode::Experto => 0.08,
        }
    }
    fn start_charges(&self) -> u32 {
        match self {
            Mode::Facil => 4,
            Mode::Normal => 3,
            Mode::Experto => 2,
        }
    }
    /// Cada cuantas piezas sube una linea de "ruido cuantico".
    fn garbage_every(&self, level: u32) -> Option<u32> {
        match self {
            Mode::Facil => None,
            Mode::Normal => {
                if level >= 10 {
                    Some(18)
                } else {
                    None
                }
            }
            Mode::Experto => Some(if level >= 14 { 9 } else { 14 }),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Cell {
    pub kind: u8,
    /// 0 = sin entrelazar; si no, grupo * 2 + mitad.
    pub ent: u32,
}

#[derive(Clone, Copy)]
pub struct Spec {
    pub kind: usize,
    pub superpos: bool,
    pub entangled: bool,
    /// Efecto tunel: al tocar fondo atraviesa bloques y cae al hueco de abajo.
    pub tunnel: bool,
}

#[derive(Clone, Copy)]
pub struct Active {
    pub spec: Spec,
    pub rot: usize,
    pub x: i32,
    pub y: i32,
    /// Desplazamiento de la segunda copia mientras la pieza esta en superposicion.
    pub dx: Option<i32>,
}

impl Active {
    pub fn xs(&self) -> Vec<i32> {
        match self.dx {
            Some(d) => vec![self.x, self.x + d],
            None => vec![self.x],
        }
    }
}

#[derive(Clone)]
pub enum Event {
    Move,
    Rotate,
    Lock,
    HardDrop,
    Hold,
    Observe,
    Garbage,
    GameOver,
    Perfect,
    /// Se abrieron las dimensiones cuanticas.
    DimOpen,
    /// Salto a la otra dimension.
    DimSwitch,
    /// La pieza no cabe en la otra dimension.
    DimBlocked,
    /// Se borraron filas por simetria (cuantas).
    Symmetry(usize),
    LevelUp(u32),
    Combo(i32),
    Clear {
        rows: Vec<i32>,
        cells: Vec<(i32, i32, u8)>,
        n: usize,
        points: u64,
    },
    Entangled {
        cells: Vec<(i32, i32, u8)>,
    },
    Collapse {
        kind: usize,
        rot: usize,
        x: i32,
        y: i32,
    },
    Tunnel {
        kind: usize,
        rot: usize,
        x: i32,
        y0: i32,
        y1: i32,
        points: u64,
    },
    /// Resumen de como se coloco una pieza (para los retos laterales).
    Placed {
        kind: usize,
        rot: usize,
        from_hold: bool,
        observed: bool,
        tunneled: bool,
        /// Quedo con un bloque encima: entro de lado bajo un saliente.
        slid: bool,
        lines: usize,
    },
}

#[derive(Clone)]
pub struct Game {
    pub grid: [[Cell; W as usize]; H as usize],
    pub active: Option<Active>,
    pub hold: Option<Spec>,
    pub hold_used: bool,
    pub queue: VecDeque<Spec>,
    bag: Vec<usize>,
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    pub charges: u32,
    pub mode: Mode,
    pub over: bool,
    pub events: Vec<Event>,
    combo: i32,
    b2b: bool,
    gravity_t: f32,
    lock_t: f32,
    lock_resets: u32,
    lowest_y: i32,
    next_group: u32,
    pending_pair: Option<u32>,
    locks: u32,
    spawned: u32,
    /// Modo acertijo: sin gravedad, piezas contadas, sin piezas al azar.
    pub puzzle: bool,
    /// En acertijo: ya no quedan piezas por jugar.
    pub out_of_pieces: bool,
    cur_from_hold: bool,
    cur_observed: bool,
    cur_tunneled: bool,
    /// Mundo cuantico puro: niveles que abren un concepto cuantico cada uno.
    pub quantum: bool,
    /// Dimensiones cuanticas abiertas: el pozo existe en dos dimensiones a la vez.
    pub dims: bool,
    /// Dimension visible (0 = A, 1 = B). `grid` es siempre la visible.
    pub dim: u8,
    /// Bloques de la otra dimension (no chocan con la pieza, pero completan lineas).
    pub others: [[[Cell; W as usize]; H as usize]; 3],
    /// Cuantas dimensiones hay abiertas (1 al principio, hasta 4).
    pub nd: usize,
    /// Simetria: una fila simetrica con 6 o mas bloques tambien se borra.
    pub sym: bool,
}

/// Un paso de la solucion de un nivel lateral: guardar la pieza, o dejarla
/// quieta en (giro, columna, fila). Lo escribe el generador y lo repite Jev.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    Hold,
    Place(usize, i32, i32),
}

/// Un movimiento de la pieza (los mismos que tiene el jugador).
#[derive(Clone, Copy)]
pub enum Mv {
    Left,
    Right,
    Down,
    Cw,
    Ccw,
}

/// Lee una solucion guardada: "H;P1,4,19;P0,3,20".
pub fn parse_sol(s: &str) -> Vec<Op> {
    s.split(';')
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            if p == "H" {
                return Some(Op::Hold);
            }
            let v: Vec<i32> = p.trim_start_matches('P').split(',').filter_map(|n| n.parse().ok()).collect();
            if v.len() == 3 {
                Some(Op::Place(v[0] as usize, v[1], v[2]))
            } else {
                None
            }
        })
        .collect()
}

/// Valor de un pozo para Jev: pocas alturas, pocos huecos, superficie lisa, lineas hechas.
fn bot_eval(g: &Game, lines_before: u32) -> f32 {
    if g.over {
        return -1.0e6;
    }
    let (mut agg, mut holes, mut bump, mut prev) = (0.0f32, 0.0f32, 0.0f32, -1.0f32);
    for x in 0..W {
        let (mut h, mut seen) = (0, false);
        for y in 0..H {
            if g.grid[y as usize][x as usize].kind != 0 {
                if !seen {
                    h = H - y;
                    seen = true;
                }
            } else if seen {
                holes += 1.0;
            }
        }
        agg += h as f32;
        if prev >= 0.0 {
            bump += (h as f32 - prev).abs();
        }
        prev = h as f32;
    }
    -0.51 * agg + 0.76 * (g.lines - lines_before) as f32 - 0.36 * holes - 0.18 * bump
}

/// Lineas totales necesarias para llegar a cada nivel cuantico (2 a 7).
const Q_GOALS: [u32; 6] = [3, 7, 11, 16, 22, 29];

/// Nivel cuantico que corresponde a un total de lineas.
pub fn qlevel_for(lines: u32) -> u32 {
    let mut lv = 1;
    for g in Q_GOALS.iter() {
        if lines >= *g {
            lv += 1;
        }
    }
    if lines >= Q_GOALS[5] {
        lv += (lines - Q_GOALS[5]) / 7;
    }
    lv
}

/// Nombre del concepto que abre cada nivel cuantico.
pub fn qlevel_name(lv: u32) -> &'static str {
    match lv {
        0 | 1 => "SUPERPOSICION",
        2 => "EFECTO TUNEL",
        3 => "ENTRELAZAMIENTO",
        4 => "DIMENSIONES",
        5 => "TERCERA DIMENSION",
        6 => "SIMETRIA",
        7 => "CUARTA DIMENSION",
        _ => "SUPERCUANTICO",
    }
}

impl Game {
    pub fn new(mode: Mode) -> Game {
        let mut g = Game {
            grid: [[Cell::default(); W as usize]; H as usize],
            active: None,
            hold: None,
            hold_used: false,
            queue: VecDeque::new(),
            bag: Vec::new(),
            score: 0,
            lines: 0,
            level: mode.start_level(),
            charges: mode.start_charges(),
            mode,
            over: false,
            events: Vec::new(),
            combo: -1,
            b2b: false,
            gravity_t: 0.0,
            lock_t: 0.0,
            lock_resets: 0,
            lowest_y: 0,
            next_group: 0,
            pending_pair: None,
            locks: 0,
            spawned: 0,
            puzzle: false,
            out_of_pieces: false,
            cur_from_hold: false,
            cur_observed: false,
            cur_tunneled: false,
            quantum: false,
            dims: false,
            dim: 0,
            others: [[[Cell::default(); W as usize]; H as usize]; 3],
            nd: 1,
            sym: false,
        };
        for i in 0..5 {
            let s = g.gen_spec(i < 2);
            g.queue.push_back(s);
        }
        // La tercera pieza siempre es TUNEL, para descubrir la mecanica pronto.
        if let Some(s) = g.queue.get_mut(2) {
            s.superpos = false;
            s.entangled = false;
            s.tunnel = true;
        }
        g.spawn_next();
        g
    }

    /// Partida del MUNDO CUANTICO: empieza en el nivel 1 (superposicion).
    pub fn new_quantum() -> Game {
        let mut g = Game::new(Mode::Facil);
        g.quantum = true;
        g.level = 1;
        g.charges = 3;
        g.queue.clear();
        g.active = None;
        g.events.clear();
        for _ in 0..5 {
            let s = g.gen_spec(true);
            g.queue.push_back(s);
        }
        // La segunda pieza ya llega en superposicion: el concepto del nivel 1.
        if let Some(s) = g.queue.get_mut(1) {
            s.superpos = true;
        }
        g.spawn_next();
        g
    }

    /// Lineas hechas y lineas que pide el siguiente nivel cuantico.
    pub fn q_progress(&self) -> (u32, u32) {
        let mut prev = 0;
        for g in Q_GOALS.iter() {
            if self.lines < *g {
                return (self.lines - prev, *g - prev);
            }
            prev = *g;
        }
        ((self.lines - prev) % 7, 7)
    }

    /// Camino mas corto (mover, bajar, girar con sus patadas) de la pieza activa hasta
    /// (giro, columna, fila). Es el mismo recorrido que puede hacer un jugador.
    pub fn path_to(&self, rot: usize, x: i32, y: i32) -> Option<Vec<Mv>> {
        use std::collections::HashMap;
        let a0 = self.active?;
        if a0.dx.is_some() {
            return None;
        }
        let kind = a0.spec.kind;
        let start = (a0.rot, a0.x, a0.y);
        let goal = (rot, x, y);
        let mut seen: HashMap<(usize, i32, i32), Option<((usize, i32, i32), Mv)>> = HashMap::new();
        let mut q = VecDeque::new();
        seen.insert(start, None);
        q.push_back(start);
        while let Some(n) = q.pop_front() {
            if n == goal {
                let mut out = Vec::new();
                let mut cur = n;
                while let Some(Some((p, m))) = seen.get(&cur).copied() {
                    out.push(m);
                    cur = p;
                }
                out.reverse();
                return Some(out);
            }
            let (r, x0, y0) = n;
            let mut nxt: Vec<((usize, i32, i32), Mv)> = Vec::new();
            for (d, m) in [(-1, Mv::Left), (1, Mv::Right)] {
                if self.fits(kind, r, x0 + d, y0) {
                    nxt.push(((r, x0 + d, y0), m));
                }
            }
            if self.fits(kind, r, x0, y0 + 1) {
                nxt.push(((r, x0, y0 + 1), Mv::Down));
            }
            if kind != 1 {
                for (cw, m) in [(true, Mv::Cw), (false, Mv::Ccw)] {
                    let to = if cw { (r + 1) % 4 } else { (r + 3) % 4 };
                    for &(kx, ky) in pieces::kicks(kind, r, to).iter() {
                        if self.fits(kind, to, x0 + kx, y0 + ky) {
                            nxt.push(((to, x0 + kx, y0 + ky), m));
                            break;
                        }
                    }
                }
            }
            for (nn, m) in nxt {
                if !seen.contains_key(&nn) {
                    seen.insert(nn, Some((n, m)));
                    q.push_back(nn);
                }
            }
        }
        None
    }

    /// Hace un movimiento con las mismas funciones que usan los botones.
    pub fn apply_mv(&mut self, m: Mv) {
        match m {
            Mv::Left => {
                self.shift(-1);
            }
            Mv::Right => {
                self.shift(1);
            }
            Mv::Down => {
                self.soft_step();
            }
            Mv::Cw => {
                self.rotate(true);
            }
            Mv::Ccw => {
                self.rotate(false);
            }
        }
    }

    /// Jev (la IA de demostracion): prueba cada giro y columna con las reglas reales
    /// y elige la jugada que deja el pozo mas limpio. Devuelve (giros, columna).
    pub fn bot_plan(&self) -> Option<(usize, i32)> {
        self.active?;
        let mut best: Option<(f32, usize, i32)> = None;
        for r in 0..4usize {
            let mut g0 = self.clone();
            let mut ok = true;
            for _ in 0..r {
                if !g0.rotate(true) {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            for xt in -3..(W + 1) {
                let mut g = g0.clone();
                let mut guard = 0;
                loop {
                    let a = g.active?;
                    if a.x == xt || guard > 14 {
                        break;
                    }
                    if !g.shift(if xt > a.x { 1 } else { -1 }) {
                        break;
                    }
                    guard += 1;
                }
                if g.active?.x != xt {
                    continue;
                }
                let lines0 = g.lines;
                g.hard_drop();
                let sc = bot_eval(&g, lines0);
                if best.map(|b| sc > b.0).unwrap_or(true) {
                    best = Some((sc, r, xt));
                }
            }
        }
        best.map(|b| (b.1, b.2))
    }

    /// Pasa a la siguiente dimension (A, B, C, D y vuelta a A): el pozo visible pasa al
    /// final de la cola de hologramas y el primer holograma se vuelve el pozo.
    fn cycle_dim(&mut self) {
        let n = self.nd - 1;
        std::mem::swap(&mut self.grid, &mut self.others[0]);
        self.others[..n].rotate_left(1);
        self.dim = ((self.dim as usize + 1) % self.nd) as u8;
    }

    /// Salta a la siguiente dimension si la pieza cabe alli. Devuelve si pudo.
    pub fn switch_dim(&mut self) -> bool {
        if self.nd < 2 || self.over {
            return false;
        }
        self.cycle_dim();
        let ok = match self.active {
            Some(a) => self.active_fits(&a),
            None => true,
        };
        if !ok {
            // Deshacer: dar la vuelta completa menos uno es volver al punto de partida
            for _ in 1..self.nd {
                self.cycle_dim();
            }
            self.events.push(Event::DimBlocked);
            return false;
        }
        self.lock_t = 0.0;
        self.events.push(Event::DimSwitch);
        true
    }

    /// Al subir de nivel cuantico: abre dimensiones y trae ya la pieza del concepto nuevo.
    fn on_qlevel(&mut self) {
        // 4: segunda dimension, 5: tercera, 7: cuarta
        let want = match self.level {
            0..=3 => 1,
            4 => 2,
            5 | 6 => 3,
            _ => 4,
        };
        if want > self.nd {
            self.nd = want;
            self.dims = true;
            self.events.push(Event::DimOpen);
        }
        if self.level >= 6 {
            self.sym = true;
        }
        let plain = Spec {
            kind: 0,
            superpos: false,
            entangled: false,
            tunnel: false,
        };
        match self.level {
            2 => {
                if let Some(s) = self.queue.get_mut(0) {
                    *s = Spec { kind: s.kind, tunnel: true, ..plain };
                }
            }
            3 => {
                for i in 0..2 {
                    if let Some(s) = self.queue.get_mut(i) {
                        *s = Spec { kind: s.kind, entangled: true, ..plain };
                    }
                }
            }
            _ => {}
        }
    }

    /// Solo para pruebas en escritorio: sube un nivel cuantico.
    pub fn debug_level_up(&mut self) {
        if !self.quantum {
            return;
        }
        let mut l = self.lines + 1;
        while qlevel_for(l) == self.level {
            l += 1;
        }
        self.lines = l;
        self.level = qlevel_for(l);
        self.events.push(Event::LevelUp(self.level));
        self.on_qlevel();
    }

    /// Crea una partida de acertijo a partir de su descripcion.
    pub fn from_puzzle(p: &crate::puzzles::Puzzle) -> Game {
        Game::from_parts(p.rows, p.pieces, p.pending, p.charges)
    }

    /// Igual que from_puzzle pero con datos sueltos (lo usa el generador de acertijos).
    pub fn from_parts(rows: &[&str], pieces: &[&str], pending: Option<char>, charges: u32) -> Game {
        let mut g = Game::new(Mode::Facil);
        g.puzzle = true;
        g.grid = [[Cell::default(); W as usize]; H as usize];
        g.queue.clear();
        g.active = None;
        g.events.clear();
        g.charges = charges;
        g.level = 1;
        let start = H as usize - rows.len();
        let mut maxg = 0;
        for (i, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate().take(W as usize) {
                let cell = match ch {
                    '#' => Cell { kind: GARBAGE, ent: 0 },
                    c if c.is_ascii_alphabetic() => {
                        let grp = (c.to_ascii_uppercase() as u32) - 64;
                        maxg = maxg.max(grp);
                        let half = if c.is_ascii_uppercase() { 0 } else { 1 };
                        Cell { kind: 3, ent: grp * 2 + half }
                    }
                    _ => Cell::default(),
                };
                g.grid[start + i][x] = cell;
            }
        }
        g.next_group = maxg;
        g.pending_pair = pending.map(|c| (c.to_ascii_uppercase() as u32) - 64);
        for s in pieces.iter() {
            let kind = "IOTSZJL".find(s.chars().next().unwrap_or('I')).unwrap_or(0);
            let flags = &s[1..];
            g.queue.push_back(Spec {
                kind,
                superpos: flags.contains('s'),
                entangled: flags.contains('e'),
                tunnel: flags.contains('t'),
            });
        }
        g.spawn_next();
        g
    }

    /// Piezas que faltan por jugar en un acertijo (sin contar la actual).
    pub fn remaining(&self) -> usize {
        self.queue.len() + self.hold.map(|_| 1).unwrap_or(0)
    }

    /// Piezas ya colocadas (en acertijos mide la eficiencia de la solucion).
    pub fn pieces_used(&self) -> u32 {
        self.locks
    }

    /// Para el verificador de acertijos: fija la pieza activa donde esta y la coloca.
    #[allow(dead_code)]
    pub fn place_exact(&mut self, a: Active, tunneled: bool, observed: bool) {
        self.active = Some(a);
        self.cur_tunneled = tunneled;
        self.cur_observed = observed;
        self.lock();
    }

    /// Firma del estado (tablero, piezas, guardada, cargas...) para no repetir busquedas.
    #[allow(dead_code)]
    pub fn sig(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for row in self.grid.iter() {
            for c in row.iter() {
                (c.kind, c.ent).hash(&mut h);
            }
        }
        let spec = |s: &Spec| (s.kind, s.superpos, s.entangled, s.tunnel);
        if let Some(a) = &self.active {
            (spec(&a.spec), a.rot, a.x, a.y, a.dx).hash(&mut h);
        }
        for s in self.queue.iter() {
            spec(s).hash(&mut h);
        }
        self.hold.as_ref().map(spec).hash(&mut h);
        (self.hold_used, self.charges, self.pending_pair, self.next_group, self.lines).hash(&mut h);
        h.finish()
    }

    pub fn board_empty(&self) -> bool {
        self.grid.iter().all(|r| r.iter().all(|c| c.kind == 0))
    }

    fn free(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= W || y >= H {
            return false;
        }
        if y < 0 {
            return true;
        }
        self.grid[y as usize][x as usize].kind == 0
    }

    pub fn fits(&self, kind: usize, rot: usize, x: i32, y: i32) -> bool {
        pieces::cells(kind, rot)
            .iter()
            .all(|&(cx, cy)| self.free(x + cx, y + cy))
    }

    fn active_fits(&self, a: &Active) -> bool {
        a.xs()
            .iter()
            .all(|&x| self.fits(a.spec.kind, a.rot, x, a.y))
    }

    pub fn drop_y(&self, kind: usize, rot: usize, x: i32, y: i32) -> i32 {
        let mut yy = y;
        while self.fits(kind, rot, x, yy + 1) {
            yy += 1;
        }
        yy
    }

    fn grounded(&self, a: &Active) -> bool {
        let n = Active { y: a.y + 1, ..*a };
        !self.active_fits(&n)
    }

    pub fn gravity_interval(&self) -> f32 {
        let l = (self.level.min(20).max(1) - 1) as f32;
        (0.8 - l * 0.007).powf(l).max(0.025)
    }

    fn gen_spec(&mut self, plain: bool) -> Spec {
        if self.bag.is_empty() {
            self.bag = (0..7).collect();
            for i in (1..self.bag.len()).rev() {
                let j = rand_int(i as i32 + 1) as usize;
                self.bag.swap(i, j);
            }
        }
        let kind = self.bag.pop().unwrap_or(0);
        if plain {
            return Spec {
                kind,
                superpos: false,
                entangled: false,
                tunnel: false,
            };
        }
        let lvl = self.level as f32;
        // (superposicion, entrelazada, tunel): en el mundo cuantico cada nivel abre un concepto
        let (p, pe, pt) = if self.quantum {
            match self.level {
                0 | 1 => (0.30, 0.0, 0.0),
                2 => (0.15, 0.0, 0.30),
                3 => (0.15, 0.30, 0.18),
                4 => (0.15, 0.22, 0.15),
                _ => (0.22, 0.28, 0.20),
            }
        } else {
            (
                (self.mode.superpos_base() + (lvl - 1.0) * 0.015).min(self.mode.superpos_cap()),
                self.mode.entangle_prob(),
                self.mode.tunnel_prob(),
            )
        };
        let superpos = gen_range(0.0f32, 1.0f32) < p;
        let entangled = !superpos && gen_range(0.0f32, 1.0f32) < pe;
        let tunnel = !superpos && !entangled && gen_range(0.0f32, 1.0f32) < pt;
        Spec {
            kind,
            superpos,
            entangled,
            tunnel,
        }
    }

    /// Donde terminaria una pieza que esta apoyada en `y` si atraviesa bloques.
    /// Busca hasta TUNNEL_DEPTH filas mas abajo un sitio libre y deja caer ahi.
    pub fn tunnel_target(&self, kind: usize, rot: usize, x: i32, y: i32) -> Option<i32> {
        // La pieza puede bajar su altura + TUNNEL_DEPTH filas: asi cualquier
        // forma atraviesa hasta TUNNEL_DEPTH filas de bloques.
        let cs = pieces::cells(kind, rot);
        let miny = cs.iter().map(|c| c.1).min().unwrap_or(0);
        let maxy = cs.iter().map(|c| c.1).max().unwrap_or(0);
        let h = maxy - miny + 1;
        for yy in (y + 2)..=(y + h + TUNNEL_DEPTH) {
            if self.fits(kind, rot, x, yy) {
                return Some(self.drop_y(kind, rot, x, yy));
            }
        }
        None
    }

    /// Destino del tunel para la pieza activa (para dibujar la guia).
    pub fn active_tunnel_target(&self) -> Option<(i32, i32)> {
        let a = self.active?;
        if !a.spec.tunnel || a.dx.is_some() {
            return None;
        }
        let gy = self.drop_y(a.spec.kind, a.rot, a.x, a.y);
        self.tunnel_target(a.spec.kind, a.rot, a.x, gy).map(|ty| (gy, ty))
    }

    /// Si la pieza TUNEL esta apoyada y hay un hueco debajo, la atraviesa.
    fn try_tunnel(&mut self) -> bool {
        let mut a = match self.active {
            Some(a) => a,
            None => return false,
        };
        if !a.spec.tunnel || a.dx.is_some() || !self.grounded(&a) {
            return false;
        }
        if let Some(ty) = self.tunnel_target(a.spec.kind, a.rot, a.x, a.y) {
            let y0 = a.y;
            a.y = ty;
            a.spec.tunnel = false;
            self.active = Some(a);
            self.lowest_y = ty;
            self.lock_t = 0.0;
            self.lock_resets = 0;
            self.cur_tunneled = true;
            let pts = 40 * (ty - y0) as u64 * self.level as u64;
            self.score += pts;
            self.events.push(Event::Tunnel {
                kind: a.spec.kind,
                rot: a.rot,
                x: a.x,
                y0,
                y1: ty,
                points: pts,
            });
            return true;
        }
        false
    }

    /// Mete piezas al frente de la cola (despues de la pieza actual).
    pub fn inject(&mut self, specs: &[Spec]) {
        for s in specs.iter().rev() {
            self.queue.push_front(*s);
        }
    }

    pub fn add_charge(&mut self) {
        self.charges = (self.charges + 1).min(9);
    }

    fn spawn(&mut self, spec: Spec) {
        self.spawned += 1;
        self.cur_from_hold = false;
        self.cur_observed = false;
        self.cur_tunneled = false;
        let mut a = Active {
            spec,
            rot: 0,
            x: 3,
            y: HIDDEN - 1,
            dx: None,
        };
        if spec.superpos {
            let sp = Active {
                x: 1,
                dx: Some(5),
                ..a
            };
            if self.active_fits(&sp) {
                a = sp;
            } else {
                a.spec.superpos = false;
            }
        }
        if !self.active_fits(&a) {
            a.y -= 1;
            // Con dimensiones abiertas, si aqui no cabe la pieza escapa a la otra dimension.
            if !self.active_fits(&a) && self.nd >= 2 {
                let mut found = false;
                for _ in 1..self.nd {
                    self.cycle_dim();
                    let b = Active { y: a.y + 1, ..a };
                    if self.active_fits(&b) {
                        a = b;
                        found = true;
                        break;
                    }
                    if self.active_fits(&a) {
                        found = true;
                        break;
                    }
                }
                if found {
                    self.events.push(Event::DimSwitch);
                } else {
                    self.cycle_dim();
                }
            }
            if !self.active_fits(&a) {
                self.active = None;
                self.over = true;
                self.events.push(Event::GameOver);
                return;
            }
        }
        self.lowest_y = a.y;
        self.active = Some(a);
        self.gravity_t = 0.0;
        self.lock_t = 0.0;
        self.lock_resets = 0;
    }

    fn spawn_next(&mut self) {
        if self.puzzle {
            // En acertijo no hay piezas al azar: si se acaba la cola, se usa la
            // guardada; si tampoco hay, se terminaron las piezas.
            let s = match self.queue.pop_front() {
                Some(s) => s,
                None => match self.hold.take() {
                    Some(h) => h,
                    None => {
                        self.active = None;
                        self.out_of_pieces = true;
                        return;
                    }
                },
            };
            self.spawn(s);
            return;
        }
        let s = match self.queue.pop_front() {
            Some(s) => s,
            None => self.gen_spec(false),
        };
        let n = self.gen_spec(false);
        self.queue.push_back(n);
        self.spawn(s);
    }

    fn after_manip(&mut self) {
        if let Some(a) = self.active {
            if self.grounded(&a) && self.lock_resets < MAX_RESETS {
                self.lock_t = 0.0;
                self.lock_resets += 1;
            }
        }
    }

    fn set_y(&mut self, a: Active) {
        if a.y > self.lowest_y {
            self.lowest_y = a.y;
            self.lock_resets = 0;
        }
        self.active = Some(a);
    }

    pub fn shift(&mut self, dir: i32) -> bool {
        if self.over {
            return false;
        }
        if let Some(a) = self.active {
            let n = Active { x: a.x + dir, ..a };
            if self.active_fits(&n) {
                self.active = Some(n);
                self.after_manip();
                self.events.push(Event::Move);
                return true;
            }
        }
        false
    }

    pub fn rotate(&mut self, cw: bool) -> bool {
        if self.over {
            return false;
        }
        if let Some(a) = self.active {
            if a.spec.kind == 1 {
                return false;
            }
            let to = if cw { (a.rot + 1) % 4 } else { (a.rot + 3) % 4 };
            for &(kx, ky) in pieces::kicks(a.spec.kind, a.rot, to).iter() {
                let n = Active {
                    rot: to,
                    x: a.x + kx,
                    y: a.y + ky,
                    ..a
                };
                if self.active_fits(&n) {
                    self.set_y(n);
                    self.after_manip();
                    self.events.push(Event::Rotate);
                    return true;
                }
            }
        }
        false
    }

    /// Bajada manual de una fila (arrastre tactil).
    pub fn soft_step(&mut self) -> bool {
        if self.over {
            return false;
        }
        if let Some(a) = self.active {
            let n = Active { y: a.y + 1, ..a };
            if self.active_fits(&n) {
                self.set_y(n);
                self.score += 1;
                self.gravity_t = 0.0;
                return true;
            }
            if a.dx.is_some() {
                self.collapse(None);
            } else {
                self.try_tunnel();
            }
        }
        false
    }

    /// La superposicion colapsa: una de las dos copias se vuelve real.
    fn collapse(&mut self, pick: Option<usize>) {
        if let Some(mut a) = self.active {
            if let Some(d) = a.dx {
                let idx = match pick {
                    Some(i) => i,
                    None => {
                        if coin() {
                            0
                        } else {
                            1
                        }
                    }
                };
                let vanished = if idx == 0 { a.x + d } else { a.x };
                if idx == 1 {
                    a.x += d;
                }
                a.dx = None;
                self.active = Some(a);
                self.lock_t = 0.0;
                self.events.push(Event::Collapse {
                    kind: a.spec.kind,
                    rot: a.rot,
                    x: vanished,
                    y: a.y,
                });
            }
        }
    }

    /// Medicion: gasta una carga y colapsa hacia la copia que cae mas profundo.
    pub fn observe(&mut self) -> bool {
        if self.over || self.charges == 0 {
            return false;
        }
        if let Some(a) = self.active {
            if let Some(d) = a.dx {
                let y0 = self.drop_y(a.spec.kind, a.rot, a.x, a.y);
                let y1 = self.drop_y(a.spec.kind, a.rot, a.x + d, a.y);
                let pick = if y0 > y1 {
                    0
                } else if y1 > y0 {
                    1
                } else if coin() {
                    0
                } else {
                    1
                };
                self.charges -= 1;
                self.cur_observed = true;
                self.collapse(Some(pick));
                self.events.push(Event::Observe);
                return true;
            }
        }
        false
    }

    pub fn hard_drop(&mut self) {
        if self.over {
            return;
        }
        let mut count = 0u64;
        loop {
            let a = match self.active {
                Some(a) => a,
                None => return,
            };
            let n = Active { y: a.y + 1, ..a };
            if self.active_fits(&n) {
                self.active = Some(n);
                count += 1;
            } else if a.dx.is_some() {
                self.collapse(None);
            } else if !self.try_tunnel() {
                break;
            }
        }
        self.score += 2 * count;
        self.events.push(Event::HardDrop);
        self.lock();
    }

    pub fn hold(&mut self) -> bool {
        if self.hold_used || self.over {
            return false;
        }
        // En acertijo, guardar sin nada que sacar dejaria el tablero sin pieza.
        if self.puzzle && self.hold.is_none() && self.queue.is_empty() {
            return false;
        }
        if let Some(a) = self.active {
            let mut spec = a.spec;
            spec.superpos = a.dx.is_some();
            self.hold_used = true;
            match self.hold.replace(spec) {
                Some(h) => {
                    self.spawn(h);
                    self.cur_from_hold = true;
                }
                None => self.spawn_next(),
            }
            self.events.push(Event::Hold);
            return true;
        }
        false
    }

    pub fn update(&mut self, dt: f32, soft: bool) {
        if self.over || self.active.is_none() {
            return;
        }
        let g = if self.puzzle {
            // Sin gravedad: la pieza solo baja si la arrastras o la sueltas.
            if !soft {
                self.gravity_t = 0.0;
                f32::INFINITY
            } else {
                1.0
            }
        } else {
            self.gravity_interval()
        };
        let interval = if soft { (g / 20.0).min(0.04) } else { g };
        self.gravity_t += dt;
        while self.gravity_t >= interval {
            self.gravity_t -= interval;
            let cur = match self.active {
                Some(a) => a,
                None => return,
            };
            let n = Active { y: cur.y + 1, ..cur };
            if self.active_fits(&n) {
                self.set_y(n);
                if soft {
                    self.score += 1;
                }
            } else {
                if cur.dx.is_some() {
                    self.collapse(None);
                } else {
                    self.try_tunnel();
                }
                self.gravity_t = 0.0;
                break;
            }
        }
        if let Some(a) = self.active {
            // En los niveles laterales no hay reloj: la pieza se queda donde la dejes
            // y solo se fija al pulsar CAIDA (asi se puede deslizar con calma).
            if a.dx.is_none() && !self.puzzle && self.grounded(&a) {
                self.lock_t += dt;
                if self.lock_t >= LOCK_DELAY {
                    self.lock();
                }
            } else {
                self.lock_t = 0.0;
            }
        }
    }

    fn lock(&mut self) {
        let a = match self.active.take() {
            Some(a) => a,
            None => return,
        };
        let ent = if a.spec.entangled {
            match self.pending_pair.take() {
                Some(g) => g * 2 + 1,
                None => {
                    self.next_group += 1;
                    self.pending_pair = Some(self.next_group);
                    self.next_group * 2
                }
            }
        } else {
            0
        };
        let mut all_hidden = true;
        let mut slid = false;
        for &(cx, cy) in pieces::cells(a.spec.kind, a.rot).iter() {
            let (x, y) = (a.x + cx, a.y + cy);
            if !self.cur_tunneled && x >= 0 && x < W && y > 0 && y < H {
                slid = slid || (0..y).any(|yy| self.grid[yy as usize][x as usize].kind != 0);
            }
        }
        for &(cx, cy) in pieces::cells(a.spec.kind, a.rot).iter() {
            let (x, y) = (a.x + cx, a.y + cy);
            if y >= 0 && y < H && x >= 0 && x < W {
                self.grid[y as usize][x as usize] = Cell {
                    kind: a.spec.kind as u8 + 1,
                    ent,
                };
            }
            if y >= HIDDEN {
                all_hidden = false;
            }
        }
        self.events.push(Event::Lock);
        self.hold_used = false;
        self.locks += 1;
        let lines = self.clear_lines();
        self.events.push(Event::Placed {
            kind: a.spec.kind,
            rot: a.rot,
            from_hold: self.cur_from_hold,
            observed: self.cur_observed,
            tunneled: self.cur_tunneled,
            slid,
            lines,
        });
        if all_hidden {
            self.over = true;
            self.events.push(Event::GameOver);
            return;
        }
        if let Some(every) = self.mode.garbage_every(self.level) {
            if self.locks % every == 0 {
                self.add_garbage();
            }
        }
        self.spawn_next();
    }

    fn clear_lines(&mut self) -> usize {
        // Interferencia: con varias dimensiones una fila se completa sumando todas.
        // Simetria: una fila simetrica con 6 o mas bloques tambien se borra.
        let nd = self.nd;
        let sym = self.sym;
        let filled = |s: &Game, x: i32, y: i32| -> bool {
            s.grid[y as usize][x as usize].kind != 0
                || (0..nd.saturating_sub(1)).any(|k| s.others[k][y as usize][x as usize].kind != 0)
        };
        let full: Vec<i32> = (0..H)
            .filter(|&y| {
                if (0..W).all(|x| filled(self, x, y)) {
                    return true;
                }
                sym && (0..W).filter(|&x| filled(self, x, y)).count() >= 6
                    && (0..W / 2).all(|x| filled(self, x, y) == filled(self, W - 1 - x, y))
            })
            .collect();
        let n = full.len();
        if n == 0 {
            self.combo = -1;
            return 0;
        }
        let sym_rows = full
            .iter()
            .filter(|&&y| !(0..W).all(|x| filled(self, x, y)))
            .count();
        if sym_rows > 0 {
            self.events.push(Event::Symmetry(sym_rows));
        }
        let mut cells = Vec::new();
        let mut groups: Vec<u32> = Vec::new();
        for &y in &full {
            for x in 0..W {
                let mut c = self.grid[y as usize][x as usize];
                for k in 0..nd.saturating_sub(1) {
                    let o = self.others[k][y as usize][x as usize];
                    if o.ent != 0 && !groups.contains(&(o.ent / 2)) {
                        groups.push(o.ent / 2);
                    }
                    if c.kind == 0 {
                        c = o;
                    }
                }
                cells.push((x, y, c.kind));
                if c.ent != 0 && !groups.contains(&(c.ent / 2)) {
                    groups.push(c.ent / 2);
                }
            }
        }
        let compact = |g: &[[Cell; W as usize]; H as usize]| {
            let mut ng = [[Cell::default(); W as usize]; H as usize];
            let mut dst = H - 1;
            for y in (0..H).rev() {
                if full.contains(&y) {
                    continue;
                }
                ng[dst as usize] = g[y as usize];
                dst -= 1;
            }
            ng
        };
        self.grid = compact(&self.grid);
        for k in 0..3 {
            self.others[k] = compact(&self.others[k]);
        }

        // Accion fantasmal a distancia: los bloques entrelazados desaparecen juntos
        // (tambien los que estan en la otra dimension).
        let mut ent_cells = Vec::new();
        if !groups.is_empty() {
            for y in 0..H {
                for x in 0..W {
                    let c = self.grid[y as usize][x as usize];
                    if c.ent != 0 && groups.contains(&(c.ent / 2)) {
                        ent_cells.push((x, y, c.kind));
                        self.grid[y as usize][x as usize] = Cell::default();
                    }
                    for k in 0..3 {
                        let o = self.others[k][y as usize][x as usize];
                        if o.ent != 0 && groups.contains(&(o.ent / 2)) {
                            ent_cells.push((x, y, o.kind));
                            self.others[k][y as usize][x as usize] = Cell::default();
                        }
                    }
                }
            }
        }

        let lvl = self.level as u64;
        let base = [0u64, 100, 300, 500, 800][n.min(4)];
        let mut pts = base * lvl;
        let tetra = n >= 4;
        if tetra {
            if self.b2b {
                pts = pts * 3 / 2;
            }
            self.b2b = true;
        } else {
            self.b2b = false;
        }
        self.combo += 1;
        if self.combo > 0 {
            pts += 50 * self.combo as u64 * lvl;
            self.events.push(Event::Combo(self.combo));
        }
        pts += 25 * ent_cells.len() as u64 * lvl;
        let empty = self.grid.iter().all(|r| r.iter().all(|c| c.kind == 0))
            && self.others.iter().all(|g| g.iter().all(|r| r.iter().all(|c| c.kind == 0)));
        if empty {
            pts += 2000 * lvl;
            self.events.push(Event::Perfect);
        }
        self.score += pts;
        self.events.push(Event::Clear {
            rows: full,
            cells,
            n,
            points: pts,
        });
        if !ent_cells.is_empty() {
            self.events.push(Event::Entangled { cells: ent_cells });
        }
        let old = self.level;
        self.lines += n as u32;
        self.level = if self.quantum {
            qlevel_for(self.lines)
        } else {
            self.mode.start_level() + self.lines / 10
        };
        if self.level > old {
            self.charges = (self.charges + 1).min(9);
            self.events.push(Event::LevelUp(self.level));
            if self.quantum {
                self.on_qlevel();
            }
        }
        if tetra {
            self.charges = (self.charges + 1).min(9);
        }
        n
    }

    fn add_garbage(&mut self) {
        let hole = rand_int(W);
        for y in 0..(H - 1) as usize {
            self.grid[y] = self.grid[y + 1];
        }
        let mut row = [Cell {
            kind: GARBAGE,
            ent: 0,
        }; W as usize];
        row[hole as usize] = Cell::default();
        self.grid[(H - 1) as usize] = row;
        self.events.push(Event::Garbage);
    }
}
