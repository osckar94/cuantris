use crate::pieces;
use macroquad::rand::gen_range;
use std::collections::VecDeque;

pub const W: i32 = 10;
pub const H: i32 = 22;
pub const HIDDEN: i32 = 2;
pub const GARBAGE: u8 = 8;
const LOCK_DELAY: f32 = 0.5;
const MAX_RESETS: u32 = 15;

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
}

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
        };
        for i in 0..5 {
            let s = g.gen_spec(i < 2);
            g.queue.push_back(s);
        }
        g.spawn_next();
        g
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
            };
        }
        let lvl = self.level as f32;
        let p = (self.mode.superpos_base() + (lvl - 1.0) * 0.015).min(self.mode.superpos_cap());
        let superpos = gen_range(0.0f32, 1.0f32) < p;
        let entangled = !superpos && gen_range(0.0f32, 1.0f32) < self.mode.entangle_prob();
        Spec {
            kind,
            superpos,
            entangled,
        }
    }

    fn spawn(&mut self, spec: Spec) {
        self.spawned += 1;
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
            } else {
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
        if let Some(a) = self.active {
            let mut spec = a.spec;
            spec.superpos = a.dx.is_some();
            self.hold_used = true;
            match self.hold.replace(spec) {
                Some(h) => self.spawn(h),
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
        let g = self.gravity_interval();
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
                }
                self.gravity_t = 0.0;
                break;
            }
        }
        if let Some(a) = self.active {
            if a.dx.is_none() && self.grounded(&a) {
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
        self.clear_lines();
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

    fn clear_lines(&mut self) {
        let full: Vec<i32> = (0..H)
            .filter(|&y| (0..W).all(|x| self.grid[y as usize][x as usize].kind != 0))
            .collect();
        let n = full.len();
        if n == 0 {
            self.combo = -1;
            return;
        }
        let mut cells = Vec::new();
        let mut groups: Vec<u32> = Vec::new();
        for &y in &full {
            for x in 0..W {
                let c = self.grid[y as usize][x as usize];
                cells.push((x, y, c.kind));
                if c.ent != 0 && !groups.contains(&(c.ent / 2)) {
                    groups.push(c.ent / 2);
                }
            }
        }
        let mut new_grid = [[Cell::default(); W as usize]; H as usize];
        let mut dst = H - 1;
        for y in (0..H).rev() {
            if full.contains(&y) {
                continue;
            }
            new_grid[dst as usize] = self.grid[y as usize];
            dst -= 1;
        }
        self.grid = new_grid;

        // Accion fantasmal a distancia: los bloques entrelazados desaparecen juntos.
        let mut ent_cells = Vec::new();
        if !groups.is_empty() {
            for y in 0..H {
                for x in 0..W {
                    let c = self.grid[y as usize][x as usize];
                    if c.ent != 0 && groups.contains(&(c.ent / 2)) {
                        ent_cells.push((x, y, c.kind));
                        self.grid[y as usize][x as usize] = Cell::default();
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
        let empty = self.grid.iter().all(|r| r.iter().all(|c| c.kind == 0));
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
        self.level = self.mode.start_level() + self.lines / 10;
        if self.level > old {
            self.charges = (self.charges + 1).min(9);
            self.events.push(Event::LevelUp(self.level));
        }
        if tetra {
            self.charges = (self.charges + 1).min(9);
        }
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
