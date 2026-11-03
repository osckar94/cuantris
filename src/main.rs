mod game;
mod pieces;
mod sfx;

use game::{Event, Game, Mode, Spec, H, HIDDEN, W};
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};
use sfx::{Sfx, S};
use std::collections::HashMap;
use std::f32::consts::PI;

const CYAN: Color = Color::new(0.0, 0.92, 1.0, 1.0);
const MAGENTA: Color = Color::new(0.95, 0.25, 1.0, 1.0);
const GOLD: Color = Color::new(1.0, 0.8, 0.2, 1.0);
const GREEN: Color = Color::new(0.25, 1.0, 0.55, 1.0);
const VIOLET: Color = Color::new(0.7, 0.5, 1.0, 1.0);
const MODES: [Mode; 3] = [Mode::Facil, Mode::Normal, Mode::Experto];

fn window_conf() -> Conf {
    Conf {
        window_title: "Cuantris".to_owned(),
        window_width: 540,
        window_height: 960,
        // En la web, high_dpi hace que los toques lleguen en pixeles fisicos
        // (x2/x3) y no coincidan con los botones. Sin el, todo usa la misma escala.
        high_dpi: !cfg!(target_arch = "wasm32"),
        // En Android el MSAA (sample_count > 1) hace fallar la creacion del
        // contexto grafico en muchos telefonos y la app se cierra al abrir.
        sample_count: if cfg!(target_os = "android") { 1 } else { 4 },
        window_resizable: !cfg!(target_os = "android"),
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Menu,
    Playing,
    Paused,
    Over,
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Free,
    Horiz,
    Down,
    Up,
}

struct Gesture {
    start: Vec2,
    anchor: Vec2,
    start_t: f64,
    axis: Axis,
    button: bool,
    hist: Vec<(f64, Vec2)>,
}

impl Gesture {
    fn new(pos: Vec2, now: f64, button: bool) -> Gesture {
        Gesture {
            start: pos,
            anchor: pos,
            start_t: now,
            axis: Axis::Free,
            button,
            hist: vec![(now, pos)],
        }
    }
}

enum Ptr {
    Down(u64, Vec2),
    Move(u64, Vec2),
    Up(u64, Vec2),
}

struct Particle {
    p: Vec2,
    v: Vec2,
    life: f32,
    max: f32,
    c: Color,
    s: f32,
}

struct Popup {
    text: String,
    p: Vec2,
    life: f32,
    c: Color,
    size: f32,
}

struct Fade {
    cells: Vec<(i32, i32)>,
    c: Color,
    t: f32,
}

struct MenuPiece {
    x: f32,
    y: f32,
    kind: usize,
    rot: usize,
    v: f32,
    sup: bool,
}

struct Lay {
    cell: f32,
    bx: f32,
    by: f32,
    sx: f32,
    sw: f32,
    top: f32,
    buttons: [Rect; 3],
    sound: Rect,
}

fn lay() -> Lay {
    let (w, h) = (screen_width(), screen_height());
    let top = (h * 0.075).max(40.0);
    let bot = (h * 0.1).max(56.0);
    let avail_h = h - top - bot - h * 0.02;
    let cell = ((w * 0.95) / 14.6).min(avail_h / 20.0).floor().max(4.0);
    let bw = cell * 10.0;
    let sw = cell * 4.2;
    let gap = cell * 0.4;
    let total = bw + gap + sw;
    let bx = ((w - total) / 2.0).floor();
    let by = top + ((avail_h - cell * 20.0) / 2.0).max(0.0);
    let area = total.max(w * 0.6).min(w);
    let ax = (w - area) / 2.0;
    let pad = area * 0.03;
    let bh = bot * 0.72;
    let byb = h - bot + (bot - bh) / 2.0 - h * 0.012;
    let bwid = (area - pad * 4.0) / 3.0;
    let buttons = [
        Rect::new(ax + pad, byb, bwid, bh),
        Rect::new(ax + pad * 2.0 + bwid, byb, bwid, bh),
        Rect::new(ax + pad * 3.0 + bwid * 2.0, byb, bwid, bh),
    ];
    let ss = top * 0.6;
    let sound = Rect::new(w - ss - w * 0.03, (top - ss) / 2.0, ss, ss);
    Lay {
        cell,
        bx,
        by,
        sx: bx + bw + gap,
        sw,
        top,
        buttons,
        sound,
    }
}

fn menu_rects() -> [Rect; 3] {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.74).min(h * 0.5);
    let bh = (h * 0.085).min(bw * 0.24);
    let x = (w - bw) / 2.0;
    let y0 = h * 0.36;
    [
        Rect::new(x, y0, bw, bh),
        Rect::new(x, y0 + bh * 1.3, bw, bh),
        Rect::new(x, y0 + bh * 2.6, bw, bh),
    ]
}

fn overlay_rects(n: usize) -> Vec<Rect> {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.66).min(h * 0.45);
    let bh = (h * 0.075).min(bw * 0.22);
    let x = (w - bw) / 2.0;
    let y0 = h * 0.5;
    (0..n)
        .map(|i| Rect::new(x, y0 + i as f32 * bh * 1.3, bw, bh))
        .collect()
}

// ---------------------------------------------------------------- dibujo

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a.max(0.0).min(1.0))
}

fn fit_size(t: &str, size: f32, max_w: f32) -> f32 {
    let s = size.max(1.0);
    let d = measure_text(t, None, s as u16, 1.0);
    if d.width > max_w && d.width > 0.0 {
        (s * max_w / d.width).max(1.0)
    } else {
        s
    }
}

fn text_c(t: &str, cx: f32, y: f32, size: f32, c: Color) {
    let d = measure_text(t, None, size.max(1.0) as u16, 1.0);
    draw_text(t, cx - d.width / 2.0, y, size, c);
}

fn glow_text_c(t: &str, cx: f32, y: f32, size: f32, c: Color) {
    for &(o, a) in [(size * 0.06, 0.12f32), (size * 0.03, 0.25)].iter() {
        for &(dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o)].iter() {
            text_c(t, cx + dx, y + dy, size, alpha(c, a));
        }
    }
    text_c(
        t,
        cx,
        y,
        size,
        Color::new(0.85 + c.r * 0.15, 0.85 + c.g * 0.15, 0.85 + c.b * 0.15, 1.0),
    );
}

fn draw_block(x: f32, y: f32, s: f32, c: Color, a: f32) {
    let g = s * 0.16;
    draw_rectangle(x - g, y - g, s + 2.0 * g, s + 2.0 * g, alpha(c, 0.10 * a));
    draw_rectangle(
        x + 1.0,
        y + 1.0,
        s - 2.0,
        s - 2.0,
        Color::new(c.r * 0.3, c.g * 0.3, c.b * 0.3, a),
    );
    let i = s * 0.13;
    draw_rectangle(x + i, y + i, s - 2.0 * i, s - 2.0 * i, alpha(c, 0.92 * a));
    draw_rectangle(
        x + i,
        y + i,
        s - 2.0 * i,
        s * 0.17,
        Color::new(1.0, 1.0, 1.0, 0.35 * a),
    );
    draw_rectangle_lines(x + 1.0, y + 1.0, s - 2.0, s - 2.0, (s * 0.05).max(1.0), alpha(c, a));
}

fn ent_mark(cx: f32, cy: f32, s: f32, t: f32) {
    let p = (t * 5.0).sin() * 0.5 + 0.5;
    draw_circle(cx, cy, s * (0.11 + 0.05 * p), Color::new(1.0, 1.0, 1.0, 0.9));
    draw_circle_lines(cx, cy, s * 0.3, (s * 0.05).max(1.0), Color::new(0.85, 0.6, 1.0, 0.7));
}

fn wavy_line(a: Vec2, b: Vec2, amp: f32, t: f32, thick: f32, c: Color) {
    let d = b - a;
    let len = d.length();
    if len < 1.0 {
        return;
    }
    let perp = vec2(-d.y, d.x) / len;
    let n = 24;
    let mut prev = a;
    for i in 1..=n {
        let k = i as f32 / n as f32;
        let off = (k * PI * 4.0 + t * 7.0).sin() * amp * (k * PI).sin();
        let p = a + d * k + perp * off;
        draw_line(prev.x, prev.y, p.x, p.y, thick, c);
        prev = p;
    }
}

fn panel(r: Rect, c: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.03, 0.04, 0.1, 0.8));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, alpha(c, 0.45));
}

fn button(r: Rect, label: &str, c: Color, enabled: bool, pulse: f32) {
    let a = if enabled { 1.0 } else { 0.3 };
    draw_rectangle(
        r.x,
        r.y,
        r.w,
        r.h,
        Color::new(c.r * 0.14, c.g * 0.14, c.b * 0.2, 0.88),
    );
    if pulse > 0.0 {
        draw_rectangle(r.x, r.y, r.w, r.h, alpha(c, 0.18 * pulse));
    }
    draw_rectangle_lines(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, 4.0, alpha(c, 0.15 * a));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, alpha(c, 0.9 * a));
    let size = fit_size(label, r.h * 0.42, r.w * 0.86);
    text_c(label, r.x + r.w / 2.0, r.y + r.h / 2.0 + size * 0.32, size, alpha(WHITE, a));
}

fn hash(i: u32) -> f32 {
    let mut x = i.wrapping_mul(2654435761) ^ 0x9E37_79B9;
    x ^= x >> 15;
    x = x.wrapping_mul(2246822519);
    x ^= x >> 13;
    (x % 10000) as f32 / 10000.0
}

fn draw_bg(t: f32) {
    let (w, h) = (screen_width(), screen_height());
    let n = 24;
    for i in 0..n {
        let f = i as f32 / n as f32;
        let c = Color::new(0.02 + 0.03 * f, 0.01 + 0.02 * (1.0 - f), 0.07 + 0.07 * f, 1.0);
        draw_rectangle(0.0, h * f, w, h / n as f32 + 1.0, c);
    }
    for i in 0..90u32 {
        let fx = hash(i * 2);
        let fy = hash(i * 2 + 1);
        let sp = 0.3 + hash(i + 500);
        let tw = (t * 1.5 + i as f32).sin() * 0.5 + 0.5;
        let y = (fy * h + t * h * 0.01 * sp) % h;
        draw_circle(
            fx * w,
            y,
            (0.6 + 1.4 * hash(i + 77)) * (w / 540.0).max(1.0),
            Color::new(0.7, 0.8, 1.0, 0.12 + 0.4 * tw),
        );
    }
    let cols = [CYAN, MAGENTA, VIOLET];
    for (k, col) in cols.iter().enumerate() {
        let amp = h * 0.03;
        let base = h * (0.25 + 0.25 * k as f32);
        let mut prev: Option<(f32, f32)> = None;
        for s in 0..=60 {
            let x = w * s as f32 / 60.0;
            let y = base
                + (x / w * 7.0 + t * (0.6 + 0.3 * k as f32)).sin() * amp
                + (x / w * 17.0 - t * 0.9).sin() * amp * 0.4;
            if let Some((px, py)) = prev {
                draw_line(px, py, x, y, 2.0, alpha(*col, 0.07));
            }
            prev = Some((x, y));
        }
    }
}

fn draw_mini(spec: &Spec, r: Rect, ms: f32, t: f32) {
    let cells = pieces::cells(spec.kind, 0);
    let (mut minx, mut maxx, mut miny, mut maxy) = (9, -9, 9, -9);
    for &(x, y) in cells.iter() {
        minx = minx.min(x);
        maxx = maxx.max(x);
        miny = miny.min(y);
        maxy = maxy.max(y);
    }
    let pw = (maxx - minx + 1) as f32 * ms;
    let ph = (maxy - miny + 1) as f32 * ms;
    let ox = r.x + (r.w - pw) / 2.0 - minx as f32 * ms;
    let oy = r.y + (r.h - ph) / 2.0 - miny as f32 * ms;
    let col = pieces::color(spec.kind);
    if spec.superpos {
        for k in 0..2 {
            let sg = if k == 0 { -1.0 } else { 1.0 };
            let a = 0.45 + 0.25 * (t * 8.0 + k as f32 * PI).sin();
            for &(x, y) in cells.iter() {
                draw_block(ox + x as f32 * ms + sg * ms * 0.4, oy + y as f32 * ms, ms, col, a);
            }
        }
    } else {
        for &(x, y) in cells.iter() {
            draw_block(ox + x as f32 * ms, oy + y as f32 * ms, ms, col, 1.0);
        }
    }
    if spec.entangled {
        for &(x, y) in cells.iter() {
            ent_mark(ox + (x as f32 + 0.5) * ms, oy + (y as f32 + 0.5) * ms, ms, t);
        }
    }
}

// ---------------------------------------------------------------- app

struct App {
    state: State,
    game: Game,
    mode: Mode,
    best: u64,
    new_best: bool,
    time: f32,
    shake: f32,
    particles: Vec<Particle>,
    popups: Vec<Popup>,
    fades: Vec<Fade>,
    flashes: Vec<(i32, f32)>,
    gestures: HashMap<u64, Gesture>,
    touch_mode: bool,
    block_input: bool,
    das_dir: i32,
    das_t: f32,
    arr_t: f32,
    sfx: Sfx,
    menu_pieces: Vec<MenuPiece>,
}

impl App {
    fn new(sfx: Sfx) -> App {
        let mut menu_pieces = Vec::new();
        for i in 0..14 {
            menu_pieces.push(MenuPiece {
                x: gen_range(0.0f32, 1.0f32),
                y: i as f32 / 14.0,
                kind: game::rand_int(7) as usize,
                rot: game::rand_int(4) as usize,
                v: gen_range(0.03f32, 0.08f32),
                sup: gen_range(0.0f32, 1.0f32) < 0.3,
            });
        }
        App {
            state: State::Menu,
            game: Game::new(Mode::Normal),
            mode: Mode::Normal,
            best: 0,
            new_best: false,
            time: 0.0,
            shake: 0.0,
            particles: Vec::new(),
            popups: Vec::new(),
            fades: Vec::new(),
            flashes: Vec::new(),
            gestures: HashMap::new(),
            touch_mode: false,
            block_input: false,
            das_dir: 0,
            das_t: 0.0,
            arr_t: 0.0,
            sfx,
            menu_pieces,
        }
    }

    fn set_state(&mut self, s: State) {
        self.state = s;
        self.gestures.clear();
        self.block_input = true;
    }

    fn start(&mut self, mode: Mode) {
        self.mode = mode;
        self.game = Game::new(mode);
        self.particles.clear();
        self.popups.clear();
        self.fades.clear();
        self.flashes.clear();
        self.new_best = false;
        self.sfx.play(S::Click, 0.5);
        self.set_state(State::Playing);
    }

    fn pointers(&mut self) -> Vec<Ptr> {
        let mut v = Vec::new();
        let ts = touches();
        if !ts.is_empty() {
            self.touch_mode = true;
        }
        if self.block_input {
            let held = !ts.is_empty() || is_mouse_button_down(MouseButton::Left);
            if !held {
                self.block_input = false;
            }
            return v;
        }
        if self.touch_mode {
            // macroquad entrega los toques en pixeles fisicos, pero la pantalla
            // se dibuja en pixeles logicos (high_dpi). Sin esta division el
            // dedo cae 2-3 veces mas lejos en moviles y los botones no responden.
            let (pw, _) = macroquad::miniquad::window::screen_size();
            let sw = screen_width();
            let mut dpi = if sw > 0.0 { pw / sw } else { 1.0 };
            if !dpi.is_finite() || dpi < 1.0 {
                dpi = 1.0;
            }
            for t in ts {
                let p = t.position / dpi;
                match t.phase {
                    TouchPhase::Started => v.push(Ptr::Down(t.id, p)),
                    TouchPhase::Moved | TouchPhase::Stationary => v.push(Ptr::Move(t.id, p)),
                    TouchPhase::Ended | TouchPhase::Cancelled => v.push(Ptr::Up(t.id, p)),
                }
            }
        } else {
            let m = mouse_position();
            let p = vec2(m.0, m.1);
            if is_mouse_button_pressed(MouseButton::Left) {
                v.push(Ptr::Down(0, p));
            } else if is_mouse_button_released(MouseButton::Left) {
                v.push(Ptr::Up(0, p));
            } else if is_mouse_button_down(MouseButton::Left) {
                v.push(Ptr::Move(0, p));
            }
        }
        v
    }

    fn sound_hit(&mut self, pos: Vec2, l: &Lay) -> bool {
        let r = Rect::new(l.sound.x - 10.0, l.sound.y - 10.0, l.sound.w + 20.0, l.sound.h + 20.0);
        if r.contains(pos) {
            self.sfx.on = !self.sfx.on;
            if !self.sfx.on {
                self.sfx.music(false);
            }
            self.sfx.play(S::Click, 0.5);
            return true;
        }
        false
    }

    fn update(&mut self, dt: f32) {
        self.time += dt;
        let l = lay();
        let ptrs = self.pointers();
        match self.state {
            State::Menu => self.update_menu(dt, &ptrs, &l),
            State::Playing => self.update_play(dt, &ptrs, &l),
            State::Paused => self.update_pause(&ptrs, &l),
            State::Over => self.update_over(&ptrs, &l),
        }
        self.update_fx(dt, &l);
        let playing = self.state == State::Playing;
        self.sfx.music(playing);
    }

    fn update_menu(&mut self, dt: f32, ptrs: &[Ptr], l: &Lay) {
        for mp in self.menu_pieces.iter_mut() {
            mp.y += mp.v * dt;
            if mp.y > 1.1 {
                mp.y = -0.15;
                mp.x = gen_range(0.0f32, 1.0f32);
                mp.kind = game::rand_int(7) as usize;
                mp.rot = game::rand_int(4) as usize;
            }
        }
        let rs = menu_rects();
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                for (i, r) in rs.iter().enumerate() {
                    if r.contains(pos) {
                        self.start(MODES[i]);
                        return;
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::Key1) {
            self.start(Mode::Facil);
        } else if is_key_pressed(KeyCode::Key2) || is_key_pressed(KeyCode::Enter) {
            self.start(Mode::Normal);
        } else if is_key_pressed(KeyCode::Key3) {
            self.start(Mode::Experto);
        }
    }

    fn update_play(&mut self, dt: f32, ptrs: &[Ptr], l: &Lay) {
        let cell = l.cell;
        let now = get_time();
        for p in ptrs {
            match *p {
                Ptr::Down(id, pos) => {
                    if self.gestures.contains_key(&id) {
                        continue;
                    }
                    if self.sound_hit(pos, l) {
                        self.gestures.insert(id, Gesture::new(pos, now, true));
                        continue;
                    }
                    if let Some(i) = l.buttons.iter().position(|r| r.contains(pos)) {
                        self.gestures.insert(id, Gesture::new(pos, now, true));
                        match i {
                            0 => {
                                self.game.hold();
                            }
                            1 => {
                                self.game.observe();
                            }
                            _ => {
                                self.sfx.play(S::Click, 0.5);
                                self.set_state(State::Paused);
                                return;
                            }
                        }
                        continue;
                    }
                    self.gestures.insert(id, Gesture::new(pos, now, false));
                }
                Ptr::Move(id, pos) => {
                    if let Some(g) = self.gestures.get_mut(&id) {
                        if g.button {
                            continue;
                        }
                        g.hist.push((now, pos));
                        if g.hist.len() > 16 {
                            g.hist.remove(0);
                        }
                        let d = pos - g.anchor;
                        if g.axis == Axis::Free {
                            if d.x.abs() > cell * 0.6 && d.x.abs() > d.y.abs() {
                                g.axis = Axis::Horiz;
                            } else if d.y > cell * 0.8 {
                                g.axis = Axis::Down;
                            } else if d.y < -cell * 0.9 {
                                g.axis = Axis::Up;
                            }
                        }
                        let step = cell * 0.85;
                        match g.axis {
                            Axis::Horiz => {
                                while pos.x - g.anchor.x > step {
                                    g.anchor.x += step;
                                    self.game.shift(1);
                                }
                                while g.anchor.x - pos.x > step {
                                    g.anchor.x -= step;
                                    self.game.shift(-1);
                                }
                            }
                            Axis::Down => {
                                while pos.y - g.anchor.y > step {
                                    g.anchor.y += step;
                                    self.game.soft_step();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ptr::Up(id, pos) => {
                    if let Some(g) = self.gestures.remove(&id) {
                        if g.button {
                            continue;
                        }
                        let dur = (now - g.start_t) as f32;
                        let tot = pos - g.start;
                        let mut vy = 0.0;
                        if let Some(&(t0, p0)) = g.hist.iter().find(|(t, _)| now - *t < 0.12) {
                            let dtt = (now - t0) as f32;
                            if dtt > 0.001 {
                                vy = (pos.y - p0.y) / dtt;
                            }
                        }
                        match g.axis {
                            Axis::Free => {
                                if tot.length() < cell * 0.8 && dur < 0.4 {
                                    let cw = pos.x >= screen_width() / 2.0;
                                    self.game.rotate(cw);
                                } else if tot.y > cell * 1.5 && dur < 0.3 {
                                    self.game.hard_drop();
                                }
                            }
                            Axis::Down => {
                                if vy > cell * 14.0 || (dur < 0.25 && tot.y > cell * 2.0) {
                                    self.game.hard_drop();
                                }
                            }
                            Axis::Up => {
                                if tot.y < -cell * 1.5 {
                                    self.game.hold();
                                }
                            }
                            Axis::Horiz => {}
                        }
                    }
                }
            }
        }

        // Teclado (escritorio)
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
            self.set_state(State::Paused);
            return;
        }
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::X) {
            self.game.rotate(true);
        }
        if is_key_pressed(KeyCode::Z) {
            self.game.rotate(false);
        }
        if is_key_pressed(KeyCode::Space) {
            self.game.hard_drop();
        }
        if is_key_pressed(KeyCode::C) || is_key_pressed(KeyCode::LeftShift) {
            self.game.hold();
        }
        if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Q) {
            self.game.observe();
        }
        let dir = is_key_down(KeyCode::Right) as i32 - is_key_down(KeyCode::Left) as i32;
        if dir != 0 {
            if dir != self.das_dir {
                self.das_dir = dir;
                self.das_t = 0.0;
                self.arr_t = 0.0;
                self.game.shift(dir);
            } else {
                self.das_t += dt;
                if self.das_t >= 0.16 {
                    self.arr_t += dt;
                    while self.arr_t >= 0.035 {
                        self.arr_t -= 0.035;
                        self.game.shift(dir);
                    }
                }
            }
        } else {
            self.das_dir = 0;
        }
        let soft = is_key_down(KeyCode::Down);
        self.game.update(dt, soft);
        self.process_events(l);
    }

    fn update_pause(&mut self, ptrs: &[Ptr], l: &Lay) {
        let rs = overlay_rects(3);
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if rs[0].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Playing);
                    return;
                }
                if rs[1].contains(pos) {
                    let m = self.mode;
                    self.start(m);
                    return;
                }
                if rs[2].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Menu);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Enter) {
            self.set_state(State::Playing);
        }
    }

    fn update_over(&mut self, ptrs: &[Ptr], l: &Lay) {
        let rs = overlay_rects(2);
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if rs[0].contains(pos) {
                    let m = self.mode;
                    self.start(m);
                    return;
                }
                if rs[1].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Menu);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            let m = self.mode;
            self.start(m);
        }
    }

    fn cell_center(l: &Lay, x: i32, y: i32) -> Vec2 {
        vec2(
            l.bx + (x as f32 + 0.5) * l.cell,
            l.by + ((y - HIDDEN) as f32 + 0.5) * l.cell,
        )
    }

    fn burst(&mut self, pos: Vec2, c: Color, n: usize, speed: f32, size: f32) {
        for _ in 0..n {
            if self.particles.len() > 700 {
                break;
            }
            let a = gen_range(0.0f32, PI * 2.0);
            let sp = gen_range(0.2f32, 1.0f32) * speed;
            let life = gen_range(0.35f32, 0.9f32);
            self.particles.push(Particle {
                p: pos,
                v: vec2(a.cos(), a.sin()) * sp,
                life,
                max: life,
                c,
                s: size * gen_range(0.5f32, 1.2f32),
            });
        }
    }

    fn popup(&mut self, l: &Lay, text: &str, c: Color, big: bool) {
        let size = if big { l.cell * 1.25 } else { l.cell * 0.8 };
        let idx = (self.popups.len() % 4) as f32;
        self.popups.push(Popup {
            text: text.to_string(),
            p: vec2(l.bx + l.cell * 5.0, l.by + l.cell * (6.0 + idx * 1.4)),
            life: 1.3,
            c,
            size,
        });
    }

    fn process_events(&mut self, l: &Lay) {
        let evs: Vec<Event> = std::mem::take(&mut self.game.events);
        let s = l.cell;
        for e in evs {
            match e {
                Event::Move => self.sfx.play(S::Move, 0.25),
                Event::Rotate => self.sfx.play(S::Rotate, 0.35),
                Event::Lock => self.sfx.play(S::Lock, 0.5),
                Event::HardDrop => {
                    self.shake = self.shake.max(s * 0.18);
                    self.sfx.play(S::Drop, 0.6);
                }
                Event::Hold => self.sfx.play(S::Hold, 0.5),
                Event::Observe => {
                    self.popup(l, "OBSERVADO", GOLD, false);
                }
                Event::Garbage => {
                    self.shake = self.shake.max(s * 0.25);
                    self.popup(l, "RUIDO CUANTICO", Color::new(0.7, 0.7, 0.8, 1.0), false);
                    self.sfx.play(S::Drop, 0.4);
                }
                Event::Perfect => self.popup(l, "LIMPIEZA PERFECTA", GOLD, true),
                Event::LevelUp(n) => {
                    self.popup(l, &format!("NIVEL {}", n), GREEN, true);
                    self.sfx.play(S::Level, 0.5);
                }
                Event::Combo(c) => self.popup(l, &format!("COMBO x{}", c), CYAN, false),
                Event::Clear { rows, cells, n, points } => {
                    for y in rows {
                        self.flashes.push((y, 0.0));
                    }
                    for (x, y, k) in cells {
                        let c = pieces::color(k.max(1) as usize - 1);
                        let p = App::cell_center(l, x, y);
                        self.burst(p, c, 5, s * 7.0, s * 0.18);
                    }
                    let name = match n {
                        1 => "SIMPLE",
                        2 => "DOBLE",
                        3 => "TRIPLE",
                        _ => "CUANTRIS!",
                    };
                    self.popup(l, &format!("{} +{}", name, points), if n >= 4 { MAGENTA } else { WHITE }, n >= 4);
                    if n >= 4 {
                        self.shake = self.shake.max(s * 0.4);
                        self.sfx.play(S::Tetra, 0.7);
                    } else {
                        self.sfx.play(S::Clear, 0.6);
                    }
                }
                Event::Entangled { cells } => {
                    let mut fc = Vec::new();
                    for (x, y, k) in cells.iter() {
                        let p = App::cell_center(l, *x, *y);
                        self.burst(p, VIOLET, 8, s * 6.0, s * 0.2);
                        self.burst(p, pieces::color((*k).max(1) as usize - 1), 4, s * 4.0, s * 0.15);
                        fc.push((*x, *y));
                    }
                    self.fades.push(Fade {
                        cells: fc,
                        c: VIOLET,
                        t: 0.0,
                    });
                    self.popup(l, "ENTRELAZADO!", VIOLET, false);
                    self.sfx.play(S::Entangle, 0.6);
                }
                Event::Collapse { kind, rot, x, y } => {
                    let c = pieces::color(kind);
                    let mut fc = Vec::new();
                    for &(cx, cy) in pieces::cells(kind, rot).iter() {
                        fc.push((x + cx, y + cy));
                        let p = App::cell_center(l, x + cx, y + cy);
                        self.burst(p, c, 6, s * 5.0, s * 0.16);
                        self.burst(p, WHITE, 2, s * 3.0, s * 0.1);
                    }
                    self.fades.push(Fade { cells: fc, c, t: 0.0 });
                    self.sfx.play(S::Collapse, 0.55);
                }
                Event::GameOver => {
                    if self.game.score > self.best {
                        self.best = self.game.score;
                        self.new_best = self.game.score > 0;
                    }
                    for y in HIDDEN..H {
                        for x in 0..W {
                            let k = self.game.grid[y as usize][x as usize].kind;
                            if k != 0 && gen_range(0.0f32, 1.0f32) < 0.4 {
                                let p = App::cell_center(l, x, y);
                                self.burst(p, pieces::color(k as usize - 1), 3, s * 6.0, s * 0.2);
                            }
                        }
                    }
                    self.shake = s * 0.5;
                    self.sfx.play(S::Over, 0.7);
                    self.set_state(State::Over);
                }
            }
        }
    }

    fn update_fx(&mut self, dt: f32, l: &Lay) {
        let grav = l.cell * 10.0;
        for p in self.particles.iter_mut() {
            p.p += p.v * dt;
            p.v.y += grav * dt;
            p.v *= 0.985;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for p in self.popups.iter_mut() {
            p.life -= dt;
            p.p.y -= dt * l.cell * 1.2;
        }
        self.popups.retain(|p| p.life > 0.0);
        for f in self.fades.iter_mut() {
            f.t += dt * 2.2;
        }
        self.fades.retain(|f| f.t < 1.0);
        for f in self.flashes.iter_mut() {
            f.1 += dt;
        }
        self.flashes.retain(|f| f.1 < 0.35);
        self.shake = (self.shake - dt * l.cell * 1.5).max(0.0);
    }

    // ------------------------------------------------------------ draw

    fn draw(&self) {
        let l = lay();
        clear_background(Color::new(0.01, 0.01, 0.04, 1.0));
        draw_bg(self.time);
        match self.state {
            State::Menu => self.draw_menu(),
            _ => {
                self.draw_game(&l);
                if self.state == State::Paused {
                    self.draw_pause();
                } else if self.state == State::Over {
                    self.draw_over();
                }
            }
        }
        self.draw_fx();
        draw_sound_icon(l.sound, self.sfx.on);
    }

    fn draw_menu(&self) {
        let (w, h) = (screen_width(), screen_height());
        let ms = w * 0.05;
        for mp in self.menu_pieces.iter() {
            let col = pieces::color(mp.kind);
            let bx = mp.x * w;
            let by = mp.y * h;
            let a = if mp.sup {
                0.12 + 0.1 * (self.time * 8.0 + mp.x * 10.0).sin()
            } else {
                0.18
            };
            for &(cx, cy) in pieces::cells(mp.kind, mp.rot).iter() {
                draw_block(bx + cx as f32 * ms, by + cy as f32 * ms, ms, col, a);
                if mp.sup {
                    draw_block(bx + cx as f32 * ms + ms * 4.5, by + cy as f32 * ms, ms, col, a);
                }
            }
        }
        let ts = fit_size("CUANTRIS", w * 0.2, w * 0.86);
        let ty = h * 0.17;
        let wob = (self.time * 2.0).sin() * ts * 0.02;
        glow_text_c("CUANTRIS", w / 2.0, ty + wob, ts, CYAN);
        let ss = fit_size("BLOQUES CUANTICOS", ts * 0.32, w * 0.8);
        text_c("BLOQUES CUANTICOS", w / 2.0, ty + ts * 0.5, ss, alpha(MAGENTA, 0.9));
        wavy_line(
            vec2(w * 0.15, ty + ts * 0.75),
            vec2(w * 0.85, ty + ts * 0.75),
            ts * 0.08,
            self.time,
            2.0,
            alpha(CYAN, 0.6),
        );

        let rs = menu_rects();
        let info = [
            ("FACIL", "Nivel 1  -  4 observaciones", GREEN),
            ("NORMAL", "Nivel 4  -  3 observaciones", CYAN),
            ("EXPERTO", "Nivel 8  -  ruido cuantico", MAGENTA),
        ];
        for (i, r) in rs.iter().enumerate() {
            let (name, desc, c) = info[i];
            let pulse = ((self.time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 0.5;
            button(Rect::new(r.x, r.y, r.w, r.h * 0.62), "", c, true, pulse);
            draw_rectangle(r.x, r.y + r.h * 0.62, r.w, r.h * 0.38, Color::new(c.r * 0.08, c.g * 0.08, c.b * 0.12, 0.85));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, alpha(c, 0.9));
            let s1 = fit_size(name, r.h * 0.38, r.w * 0.8);
            text_c(name, r.x + r.w / 2.0, r.y + r.h * 0.31 + s1 * 0.32, s1, WHITE);
            let s2 = fit_size(desc, r.h * 0.22, r.w * 0.9);
            text_c(desc, r.x + r.w / 2.0, r.y + r.h * 0.81 + s2 * 0.3, s2, alpha(c, 0.9));
        }

        let help = [
            "Desliza: mover    Toca: girar",
            "Desliza rapido abajo: caida total",
            "Desliza arriba: guardar pieza",
            "SUPERPOSICION: la pieza esta en 2 lugares",
            "y colapsa al azar al tocar fondo",
            "ENTRELAZADAS: se borran juntas a distancia",
        ];
        let last = rs[2];
        let mut y = last.y + last.h + h * 0.05;
        let hs = help
            .iter()
            .map(|t| fit_size(t, (w * 0.04).min(h * 0.024), w * 0.92))
            .fold(f32::MAX, f32::min);
        for (i, t) in help.iter().enumerate() {
            let c = if i >= 3 { alpha(VIOLET, 0.95) } else { alpha(WHITE, 0.75) };
            text_c(t, w / 2.0, y, hs, c);
            y += hs * 1.45;
        }
        if self.best > 0 {
            let t = format!("RECORD: {}", self.best);
            text_c(&t, w / 2.0, (y + hs).min(h * 0.98), hs * 1.1, GOLD);
        }
    }

    fn draw_game(&self, l: &Lay) {
        let w = screen_width();
        let ts = fit_size("CUANTRIS", l.top * 0.55, w * 0.5);
        glow_text_c("CUANTRIS", w / 2.0, l.top * 0.7, ts, CYAN);
        let ms = fit_size(self.game.mode.name(), l.top * 0.28, w * 0.25);
        draw_text(self.game.mode.name(), w * 0.04, l.top * 0.62, ms, alpha(MAGENTA, 0.8));
        let off = vec2(
            (self.time * 91.0).sin() * self.shake,
            (self.time * 77.0).cos() * self.shake,
        );
        self.draw_board(l, off);
        self.draw_side(l);
        let g = &self.game;
        let can_hold = !g.hold_used && g.active.is_some();
        let can_obs = g.charges > 0 && g.active.map(|a| a.dx.is_some()).unwrap_or(false);
        button(l.buttons[0], "GUARDAR", MAGENTA, can_hold, 0.0);
        let pulse = if can_obs { (self.time * 7.0).sin() * 0.5 + 0.5 } else { 0.0 };
        button(l.buttons[1], &format!("OBSERVAR {}", g.charges), GOLD, can_obs, pulse);
        button(l.buttons[2], "PAUSA", CYAN, true, 0.0);
    }

    fn draw_board(&self, l: &Lay, off: Vec2) {
        let g = &self.game;
        let s = l.cell;
        let bx = l.bx + off.x;
        let by = l.by + off.y;
        let bw = s * W as f32;
        let bh = s * (H - HIDDEN) as f32;
        let t = self.time;

        draw_rectangle(bx, by, bw, bh, Color::new(0.02, 0.03, 0.08, 0.85));
        for x in 1..W {
            let xx = bx + x as f32 * s;
            draw_line(xx, by, xx, by + bh, 1.0, Color::new(0.3, 0.5, 1.0, 0.07));
        }
        for y in 1..(H - HIDDEN) {
            let yy = by + y as f32 * s;
            draw_line(bx, yy, bx + bw, yy, 1.0, Color::new(0.3, 0.5, 1.0, 0.07));
        }
        let pulse = (t * 1.5).sin() * 0.5 + 0.5;
        for i in 0..4 {
            let e = (i as f32 + 1.0) * 2.5;
            draw_rectangle_lines(
                bx - e,
                by - e,
                bw + 2.0 * e,
                bh + 2.0 * e,
                2.0,
                Color::new(0.2, 0.7, 1.0, (0.3 + 0.15 * pulse) / (i as f32 + 1.0)),
            );
        }

        // Bloques fijos + enlaces de entrelazamiento
        let mut links: HashMap<u32, [(Vec2, f32); 2]> = HashMap::new();
        for y in HIDDEN..H {
            for x in 0..W {
                let c = g.grid[y as usize][x as usize];
                if c.kind == 0 {
                    continue;
                }
                let px = bx + x as f32 * s;
                let py = by + (y - HIDDEN) as f32 * s;
                draw_block(px, py, s, pieces::color(c.kind as usize - 1), 1.0);
                if c.ent != 0 {
                    let center = vec2(px + s / 2.0, py + s / 2.0);
                    ent_mark(center.x, center.y, s, t);
                    let e = links.entry(c.ent / 2).or_insert([(Vec2::ZERO, 0.0); 2]);
                    let half = (c.ent % 2) as usize;
                    e[half].0 += center;
                    e[half].1 += 1.0;
                }
            }
        }
        for (_, halves) in links.iter() {
            if halves[0].1 > 0.0 && halves[1].1 > 0.0 {
                let a = halves[0].0 / halves[0].1;
                let b = halves[1].0 / halves[1].1;
                wavy_line(a, b, s * 0.3, t, 2.0, Color::new(0.85, 0.6, 1.0, 0.35 + 0.2 * pulse));
            }
        }

        for &(y, ft) in self.flashes.iter() {
            if y >= HIDDEN {
                let a = 1.0 - ft / 0.35;
                draw_rectangle(bx, by + (y - HIDDEN) as f32 * s, bw, s, Color::new(1.0, 1.0, 1.0, 0.7 * a));
            }
        }

        for f in self.fades.iter() {
            let a = 1.0 - f.t;
            let grow = s * 0.3 * f.t;
            for &(x, y) in f.cells.iter() {
                if y >= HIDDEN {
                    draw_block(
                        bx + x as f32 * s - grow / 2.0,
                        by + (y - HIDDEN) as f32 * s - grow / 2.0,
                        s + grow,
                        f.c,
                        a * 0.6,
                    );
                }
            }
        }

        if let Some(a) = g.active {
            let col = pieces::color(a.spec.kind);
            let cells = pieces::cells(a.spec.kind, a.rot);
            let xs = a.xs();
            let sup = a.dx.is_some();
            for &x in xs.iter() {
                let gy = g.drop_y(a.spec.kind, a.rot, x, a.y);
                for &(cx, cy) in cells.iter() {
                    let yy = gy + cy;
                    if yy >= HIDDEN {
                        draw_rectangle_lines(
                            bx + (x + cx) as f32 * s + 3.0,
                            by + (yy - HIDDEN) as f32 * s + 3.0,
                            s - 6.0,
                            s - 6.0,
                            2.0,
                            alpha(col, if sup { 0.22 } else { 0.4 }),
                        );
                    }
                }
            }
            let mut centers = Vec::new();
            for (i, &x) in xs.iter().enumerate() {
                let al = if sup {
                    0.55 + 0.25 * (t * 10.0 + i as f32 * PI).sin()
                } else {
                    1.0
                };
                let jit = if sup {
                    vec2((t * 37.0 + i as f32).sin() * 1.5, (t * 29.0 + i as f32).cos() * 1.5)
                } else {
                    Vec2::ZERO
                };
                let mut sum = Vec2::ZERO;
                let mut top = f32::MAX;
                for &(cx, cy) in cells.iter() {
                    let px = bx + (x + cx) as f32 * s + jit.x;
                    let py = by + (a.y + cy - HIDDEN) as f32 * s + jit.y;
                    sum += vec2(px + s / 2.0, py + s / 2.0);
                    top = top.min(py);
                    if a.y + cy >= HIDDEN {
                        draw_block(px, py, s, col, al);
                        if a.spec.entangled {
                            ent_mark(px + s / 2.0, py + s / 2.0, s, t);
                        }
                    }
                }
                let c = sum / 4.0;
                centers.push(c);
                if sup && top > by - s {
                    let ls = s * 0.55;
                    text_c("50%", c.x, top - s * 0.2, ls, alpha(WHITE, 0.75));
                }
            }
            if sup && centers.len() == 2 {
                wavy_line(centers[0], centers[1], s * 0.5, t * 1.5, 2.0, alpha(col, 0.5));
            }
        }
    }

    fn draw_side(&self, l: &Lay) {
        let g = &self.game;
        let s = l.cell;
        let x = l.sx;
        let sw = l.sw;
        let t = self.time;
        let mut y = l.by;
        let lab = fit_size("SIGUIENTE", s * 0.5, sw);
        draw_text("SIGUIENTE", x, y + lab * 0.8, lab, alpha(CYAN, 0.85));
        y += lab * 1.3;
        for (i, spec) in g.queue.iter().take(3).enumerate() {
            let bh = if i == 0 { s * 2.6 } else { s * 2.0 };
            let r = Rect::new(x, y, sw, bh);
            panel(r, CYAN);
            draw_mini(spec, r, if i == 0 { s * 0.8 } else { s * 0.6 }, t);
            y += bh + s * 0.25;
        }
        y += s * 0.2;
        draw_text("GUARDADO", x, y + lab * 0.8, lab, alpha(MAGENTA, 0.85));
        y += lab * 1.3;
        let r = Rect::new(x, y, sw, s * 2.2);
        panel(r, MAGENTA);
        if let Some(h) = &g.hold {
            draw_mini(h, r, s * 0.65, t);
            if g.hold_used {
                draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.5));
            }
        }
        y += s * 2.2 + s * 0.45;
        let stats = [
            ("PUNTOS", g.score.to_string()),
            ("NIVEL", g.level.to_string()),
            ("LINEAS", g.lines.to_string()),
        ];
        for (name, val) in stats.iter() {
            draw_text(name, x, y + lab * 0.8, lab * 0.9, alpha(WHITE, 0.55));
            let vs = fit_size(val, s * 0.9, sw);
            draw_text(val, x, y + lab + vs * 0.85, vs, WHITE);
            y += lab + s * 1.15;
        }
        if self.best > 0 {
            let bt = format!("REC {}", self.best.max(g.score));
            let bs = fit_size(&bt, lab * 0.9, sw);
            draw_text(&bt, x, y + bs, bs, alpha(GOLD, 0.8));
        }
    }

    fn draw_overlay_bg(&self) {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.05, 0.7));
    }

    fn draw_pause(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let ts = fit_size("PAUSA", w * 0.16, w * 0.8);
        glow_text_c("PAUSA", w / 2.0, h * 0.38, ts, CYAN);
        let rs = overlay_rects(3);
        button(rs[0], "CONTINUAR", GREEN, true, 0.0);
        button(rs[1], "REINICIAR", GOLD, true, 0.0);
        button(rs[2], "MENU", MAGENTA, true, 0.0);
    }

    fn draw_over(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let ts = fit_size("COLAPSO TOTAL", w * 0.13, w * 0.9);
        glow_text_c("COLAPSO TOTAL", w / 2.0, h * 0.27, ts, MAGENTA);
        let sc = format!("{}", self.game.score);
        let ss = fit_size(&sc, w * 0.14, w * 0.85);
        glow_text_c(&sc, w / 2.0, h * 0.27 + ts * 1.3 + ss * 0.6, ss, GOLD);
        let info = format!("NIVEL {}   LINEAS {}", self.game.level, self.game.lines);
        let is = fit_size(&info, w * 0.05, w * 0.85);
        text_c(&info, w / 2.0, h * 0.27 + ts * 1.3 + ss * 1.25, is, alpha(WHITE, 0.8));
        if self.new_best {
            let p = (self.time * 5.0).sin() * 0.5 + 0.5;
            text_c("NUEVO RECORD!", w / 2.0, h * 0.27 + ts * 1.3 + ss * 1.25 + is * 1.6, is * 1.2, alpha(GREEN, 0.6 + 0.4 * p));
        }
        let rs = overlay_rects(2);
        button(rs[0], "REINTENTAR", GREEN, true, 0.0);
        button(rs[1], "MENU", MAGENTA, true, 0.0);
    }

    fn draw_fx(&self) {
        for p in self.particles.iter() {
            let k = p.life / p.max;
            draw_rectangle(p.p.x - p.s / 2.0, p.p.y - p.s / 2.0, p.s, p.s, alpha(p.c, k));
        }
        for p in self.popups.iter() {
            let k = (p.life / 1.3).min(1.0);
            let a = (k * 3.0).min(1.0);
            let size = fit_size(&p.text, p.size, screen_width() * 0.9);
            glow_text_c(&p.text, p.p.x, p.p.y, size, alpha(p.c, a));
        }
    }
}

fn draw_sound_icon(r: Rect, on: bool) {
    let c = Color::new(0.8, 0.9, 1.0, 0.8);
    let u = r.w / 10.0;
    draw_rectangle(r.x + u, r.y + u * 3.5, u * 2.5, u * 3.0, c);
    draw_triangle(
        vec2(r.x + u * 3.0, r.y + u * 3.5),
        vec2(r.x + u * 6.0, r.y + u),
        vec2(r.x + u * 6.0, r.y + u * 9.0),
        c,
    );
    draw_triangle(
        vec2(r.x + u * 3.0, r.y + u * 3.5),
        vec2(r.x + u * 6.0, r.y + u * 9.0),
        vec2(r.x + u * 3.0, r.y + u * 6.5),
        c,
    );
    if on {
        draw_line(r.x + u * 7.3, r.y + u * 3.8, r.x + u * 7.3, r.y + u * 6.2, u * 0.8, c);
        draw_line(r.x + u * 9.0, r.y + u * 2.5, r.x + u * 9.0, r.y + u * 7.5, u * 0.8, c);
    } else {
        let red = Color::new(1.0, 0.3, 0.4, 0.9);
        draw_line(r.x + u * 6.8, r.y + u * 3.2, r.x + u * 9.6, r.y + u * 6.8, u * 0.8, red);
        draw_line(r.x + u * 9.6, r.y + u * 3.2, r.x + u * 6.8, r.y + u * 6.8, u * 0.8, red);
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    let sfx = Sfx::load().await;
    let mut app = App::new(sfx);
    loop {
        let dt = get_frame_time().min(0.05);
        app.update(dt);
        app.draw();
        next_frame().await;
    }
}
