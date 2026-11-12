mod game;
mod lateral;
mod modo_lateral;
mod pieces;
mod puzzles;
mod sfx;
mod volumen;

use lateral::{Challenge, Stats, Tech};
use puzzles::Goal;

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
const TUNNEL_C: Color = Color::new(0.3, 1.0, 0.85, 1.0);
const MODES: [Mode; 3] = [Mode::Facil, Mode::Normal, Mode::Experto];
/// Color y letra de cada dimension cuantica: A, B, C, D.
const DIMC: [Color; 4] = [CYAN, MAGENTA, GOLD, GREEN];
const DIMN: [char; 4] = ['A', 'B', 'C', 'D'];

// Jev real (TypeSafe): la pagina web pregunta por un "puente" (Cloudflare) que guarda la
// llave. Estas funciones las pone la pagina (web_index.html); en el APK no existen.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn jev_available() -> i32;
    fn jev_ask(ptr: *const u8, len: usize);
    fn jev_poll() -> i32;
}
#[cfg(target_arch = "wasm32")]
fn jev_net_available() -> bool {
    unsafe { jev_available() == 1 }
}
#[cfg(not(target_arch = "wasm32"))]
fn jev_net_available() -> bool {
    false
}
#[cfg(target_arch = "wasm32")]
fn jev_net_ask(s: &str) {
    unsafe { jev_ask(s.as_ptr(), s.len()) }
}
#[cfg(not(target_arch = "wasm32"))]
fn jev_net_ask(_s: &str) {}
/// -1 esperando, 0..n numero de la jugada elegida, -2 error o sin respuesta.
#[cfg(target_arch = "wasm32")]
fn jev_net_poll() -> i32 {
    unsafe { jev_poll() }
}
#[cfg(not(target_arch = "wasm32"))]
fn jev_net_poll() -> i32 {
    -2
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Cuantris".to_owned(),
        window_width: 540,
        window_height: 960,
        // Con high_dpi los toques llegan en pixeles fisicos (x2/x3) y no
        // coinciden con los botones dibujados. Sin el, todo usa la misma escala.
        high_dpi: false,
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
    /// Lista de acertijos
    Puzzles,
    /// Acertijo resuelto
    PzWin,
    /// Acertijo fallado (se acabaron las piezas)
    PzFail,
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
    /// Boton DIMENSION (solo en el mundo cuantico).
    dim: Option<Rect>,
    /// Botonera: izquierda, derecha, girar izq, girar der, bajar, caida total.
    pad: Option<[Rect; 6]>,
    sound: Rect,
}

/// Mostrar botonera en pantalla (se cambia desde PAUSA).
static PAD_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Fila de acciones con cuatro botones (mundo cuantico) en vez de tres.
static FOUR_BTN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Los retos laterales ya no interrumpen la partida: cada mundo es puro.
const RETOS_EN_PARTIDA: bool = false;

fn pad_on() -> bool {
    PAD_ON.load(std::sync::atomic::Ordering::Relaxed)
}

fn lay() -> Lay {
    let (w, h) = (screen_width(), screen_height());
    let top = (h * 0.075).max(40.0);
    let with_pad = pad_on();
    let bot = if with_pad { (h * 0.21).max(118.0) } else { (h * 0.1).max(56.0) };
    let avail_h = h - top - bot - h * 0.02;
    let cell = ((w * 0.95) / 15.1).min(avail_h / 20.0).floor().max(4.0);
    let bw = cell * 10.0;
    let sw = cell * 4.2;
    // Espacio para el marco 3D del tablero sin tapar el panel lateral.
    let gap = cell * 0.9;
    let total = bw + gap + sw;
    let bx = ((w - total) / 2.0).floor();
    // El espacio libre se deja casi todo ARRIBA del tablero, para los avisos.
    let by = top + ((avail_h - cell * 20.0) * 0.8).max(0.0);
    let area = total.max(w * 0.6).min(w);
    let ax = (w - area) / 2.0;
    let pad = area * 0.03;
    let (bh, byb, padr) = if with_pad {
        // Fila de control arriba, fila de acciones abajo
        let ph = bot * 0.4;
        let py = h - bot + bot * 0.04;
        let gp = area * 0.02;
        let pw = (area - gp * 7.0) / 6.0;
        let mut r = [Rect::new(0.0, 0.0, 0.0, 0.0); 6];
        for (i, rr) in r.iter_mut().enumerate() {
            *rr = Rect::new(ax + gp + i as f32 * (pw + gp), py, pw, ph);
        }
        let bh = bot * 0.32;
        (bh, py + ph + bot * 0.13, Some(r))
    } else {
        let bh = bot * 0.72;
        (bh, h - bot + (bot - bh) / 2.0 - h * 0.012, None)
    };
    // En el mundo cuantico hay un cuarto boton: DIMENSION (entre OBSERVAR y PAUSA)
    let four = FOUR_BTN.load(std::sync::atomic::Ordering::Relaxed);
    let nb = if four { 4.0 } else { 3.0 };
    let bwid = (area - pad * (nb + 1.0)) / nb;
    let slot = |i: f32| Rect::new(ax + pad + i * (bwid + pad), byb, bwid, bh);
    let (buttons, dim) = if four {
        ([slot(0.0), slot(1.0), slot(3.0)], Some(slot(2.0)))
    } else {
        ([slot(0.0), slot(1.0), slot(2.0)], None)
    };
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
        dim,
        pad: padr,
        sound,
    }
}

/// Las dos tarjetas del menu: MUNDO CUANTICO y MUNDO LATERAL.
fn menu_rects() -> [Rect; 2] {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.9).min(h * 0.6);
    let bh = (h * 0.25).min(bw * 0.6);
    let x = (w - bw) / 2.0;
    let y0 = h * 0.24;
    [Rect::new(x, y0, bw, bh), Rect::new(x, y0 + bh + h * 0.075, bw, bh)]
}

/// Botones JEV (IA) bajo las dos tarjetas del menu: uno por mundo.
fn jev_rects() -> [Rect; 2] {
    let h = screen_height();
    let rs = menu_rects();
    let last = rs[1];
    let gap = last.w * 0.04;
    let bw = (last.w - gap) / 2.0;
    let y = last.y + last.h + h * 0.028;
    [Rect::new(last.x, y, bw, h * 0.085), Rect::new(last.x + bw + gap, y, bw, h * 0.085)]
}

/// Niveles por pagina en el MUNDO LATERAL (4 columnas x 5 filas).
const PAGE: usize = 20;

struct LevelLay {
    cells: Vec<Rect>,
    prev: Rect,
    next: Rect,
    back: Rect,
    rules: Rect,
}

/// Pantalla de niveles laterales: reglas arriba, rejilla de niveles, paginas y VOLVER.
fn level_rects() -> LevelLay {
    let (w, h) = (screen_width(), screen_height());
    let area = (w * 0.92).min(h * 0.62);
    let x0 = (w - area) / 2.0;
    let rules = Rect::new(x0, h * 0.135, area, h * 0.2);
    let gy0 = rules.y + rules.h + h * 0.035;
    let nav_h = h * 0.065;
    let gy1 = h * 0.975 - nav_h - h * 0.02;
    let cols = 4.0;
    let rows = 5.0;
    let gap = area * 0.03;
    let cw = (area - gap * (cols - 1.0)) / cols;
    let ch = ((gy1 - gy0 - gap * (rows - 1.0)) / rows).min(cw * 0.9);
    let mut cells = Vec::new();
    for i in 0..PAGE {
        let (c, r) = ((i % 4) as f32, (i / 4) as f32);
        cells.push(Rect::new(x0 + c * (cw + gap), gy0 + r * (ch + gap), cw, ch));
    }
    let ny = gy0 + rows * (ch + gap) + h * 0.005;
    let nw = area * 0.2;
    LevelLay {
        cells,
        prev: Rect::new(x0, ny, nw, nav_h),
        next: Rect::new(x0 + area - nw, ny, nw, nav_h),
        back: Rect::new(x0 + nw + gap, ny, area - 2.0 * (nw + gap), nav_h),
        rules,
    }
}

/// Botones de fin de partida (mas arriba, para dejar sitio al resumen lateral).
fn over_rects() -> Vec<Rect> {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.66).min(h * 0.45);
    let bh = (h * 0.065).min(bw * 0.2);
    let x = (w - bw) / 2.0;
    let y0 = h * 0.34;
    (0..2)
        .map(|i| Rect::new(x, y0 + i as f32 * bh * 1.3, bw, bh))
        .collect()
}

fn overlay_rects(n: usize) -> Vec<Rect> {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.66).min(h * 0.45);
    let bh = (h * 0.075).min(bw * 0.22);
    let x = (w - bw) / 2.0;
    // Con muchos botones se sube la columna para que quepa
    let y0 = (h * 0.5).min(h * 0.95 - n as f32 * bh * 1.3);
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

/// Mezcla dos colores (k=0 -> a, k=1 -> b), conserva el alfa de `a`.
fn mix(a: Color, b: Color, k: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * k,
        a.g + (b.g - a.g) * k,
        a.b + (b.b - a.b) * k,
        a.a,
    )
}

fn shade(c: Color, k: f32, a: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, a)
}

fn quad(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, c: Color) {
    draw_triangle(p0, p1, p2, c);
    draw_triangle(p0, p2, p3, c);
}

/// Cubo: cara frontal arriba a la izquierda y grosor oblicuo hacia abajo a la
/// derecha, dentro de su celda (las celdas vecinas se siguen distinguiendo).
fn draw_block(x: f32, y: f32, s: f32, c: Color, a: f32) {
    let d = (s * 0.17).max(1.5);
    let f = s - d;
    // Sombra proyectada sobre el fondo
    draw_rectangle(x + d * 1.2, y + d * 1.2, s, s, Color::new(0.0, 0.0, 0.02, 0.32 * a));
    // Grosor: cara derecha y cara inferior
    quad(vec2(x + f, y), vec2(x + s, y + d), vec2(x + s, y + s), vec2(x + f, y + f), shade(c, 0.46, a));
    quad(vec2(x, y + f), vec2(x + f, y + f), vec2(x + s, y + s), vec2(x + d, y + s), shade(c, 0.26, a));
    // Biseles de la cara frontal
    let b = (f * 0.15).max(1.0);
    let (ix0, iy0, ix1, iy1) = (x + b, y + b, x + f - b, y + f - b);
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    quad(vec2(x, y), vec2(x + f, y), vec2(ix1, iy0), vec2(ix0, iy0), alpha(mix(c, white, 0.62), a));
    quad(vec2(x, y), vec2(ix0, iy0), vec2(ix0, iy1), vec2(x, y + f), alpha(mix(c, white, 0.32), a));
    quad(vec2(x + f, y), vec2(x + f, y + f), vec2(ix1, iy1), vec2(ix1, iy0), shade(c, 0.72, a));
    quad(vec2(x, y + f), vec2(ix0, iy1), vec2(ix1, iy1), vec2(x + f, y + f), shade(c, 0.58, a));
    // Cara con degradado y brillo de gema
    let fw = ix1 - ix0;
    let fh = iy1 - iy0;
    volumen::grad_v(ix0, iy0, fw, fh, alpha(mix(c, white, 0.26), a), shade(c, 0.74, a));
    draw_triangle(vec2(ix0, iy0), vec2(ix1, iy0), vec2(ix0, iy1), Color::new(1.0, 1.0, 1.0, 0.11 * a));
    draw_rectangle(ix0 + fw * 0.1, iy0 + fh * 0.1, fw * 0.42, fh * 0.13, Color::new(1.0, 1.0, 1.0, 0.6 * a));
    draw_rectangle(ix0 + fw * 0.1, iy0 + fh * 0.3, fw * 0.12, fh * 0.12, Color::new(1.0, 1.0, 1.0, 0.45 * a));
}

/// Titulo en fuente de bloques: y es la linea base, como en draw_text.
fn block_title(t: &str, cx: f32, y: f32, max_h: f32, max_w: f32, c: Color) {
    let cell = (max_w / volumen::block_text_width(t, 1.0).max(1.0)).min(max_h / 7.0).max(1.0);
    volumen::block_text_c(t, cx, y - cell * 7.0, cell, c);
}

/// Marca de pieza TUNEL: flecha hacia abajo con anillos que bajan.
fn tunnel_mark(cx: f32, cy: f32, s: f32, t: f32) {
    // Centrado en la cara del cubo, con un agujero oscuro detras
    let (cx, cy) = (cx - s * 0.085, cy - s * 0.085);
    draw_circle(cx, cy, s * 0.34, Color::new(0.0, 0.05, 0.08, 0.55));
    let p = (t * 4.0).fract();
    for k in 0..2 {
        let q = (p + k as f32 * 0.5).fract();
        let yy = cy - s * 0.25 + q * s * 0.5;
        let w = s * (0.32 - 0.12 * q);
        draw_line(cx - w, yy - w * 0.5, cx, yy, (s * 0.07).max(1.0), Color::new(1.0, 1.0, 1.0, 0.85 * (1.0 - q)));
        draw_line(cx, yy, cx + w, yy - w * 0.5, (s * 0.07).max(1.0), Color::new(1.0, 1.0, 1.0, 0.85 * (1.0 - q)));
    }
    draw_circle_lines(cx, cy, s * 0.36, (s * 0.04).max(1.0), alpha(TUNNEL_C, 0.7));
}

/// Texto extruido en 3D (capas oscuras desplazadas + cara brillante).
fn text_3d_c(t: &str, cx: f32, y: f32, size: f32, c: Color) {
    let depth = ((size * 0.07).max(2.0)) as i32;
    for i in (1..=depth).rev() {
        let k = i as f32 / depth as f32;
        text_c(t, cx + i as f32, y + i as f32, size, shade(c, 0.45 - 0.3 * k, c.a));
    }
    for &(o, a) in [(size * 0.06, 0.10f32), (size * 0.03, 0.22)].iter() {
        for &(dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o)].iter() {
            text_c(t, cx + dx, y + dy, size, alpha(c, a * c.a));
        }
    }
    text_c(t, cx, y, size, Color::new(0.85 + c.r * 0.15, 0.85 + c.g * 0.15, 0.85 + c.b * 0.15, c.a));
    // Reflejo superior
    text_c(t, cx - 1.0, y - 1.0, size, Color::new(1.0, 1.0, 1.0, 0.25 * c.a));
}

fn pzs(n: u32) -> String {
    if n == 1 {
        "1 pieza".to_string()
    } else {
        format!("{} piezas", n)
    }
}

/// Estrella de cinco puntas (rellena si `on`, apagada si no).
fn draw_star(cx: f32, cy: f32, r: f32, on: bool) {
    let pts: Vec<Vec2> = (0..10)
        .map(|k| {
            let a = -PI / 2.0 + k as f32 * PI / 5.0;
            let rr = if k % 2 == 0 { r } else { r * 0.45 };
            vec2(cx + a.cos() * rr, cy + a.sin() * rr)
        })
        .collect();
    let (face, edge) = if on {
        (Color::new(1.0, 0.82, 0.2, 1.0), Color::new(1.0, 0.95, 0.6, 1.0))
    } else {
        (Color::new(0.12, 0.13, 0.2, 0.9), Color::new(0.35, 0.37, 0.5, 0.8))
    };
    for k in 0..10 {
        draw_triangle(vec2(cx, cy), pts[k], pts[(k + 1) % 10], face);
    }
    for k in 0..10 {
        let (a, b) = (pts[k], pts[(k + 1) % 10]);
        draw_line(a.x, a.y, b.x, b.y, (r * 0.08).max(1.0), edge);
    }
    if on {
        draw_circle(cx - r * 0.2, cy - r * 0.25, r * 0.12, Color::new(1.0, 1.0, 1.0, 0.7));
    }
}

fn ent_mark(cx: f32, cy: f32, s: f32, t: f32) {
    // Esfera engastada en la cara del cubo
    let (cx, cy) = (cx - s * 0.085, cy - s * 0.085);
    let p = (t * 5.0).sin() * 0.5 + 0.5;
    let r = s * (0.13 + 0.04 * p);
    draw_circle(cx, cy, s * 0.26, Color::new(0.12, 0.03, 0.28, 0.75));
    draw_circle_lines(cx, cy, s * 0.28, (s * 0.05).max(1.0), Color::new(0.85, 0.6, 1.0, 0.8));
    draw_circle(cx, cy, r, Color::new(0.93, 0.86, 1.0, 1.0));
    draw_circle(cx - r * 0.3, cy - r * 0.35, r * 0.38, WHITE);
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

/// Vitrina hundida: labio con luz abajo/derecha y sombra interior arriba/izquierda.
fn panel(r: Rect, c: Color) {
    let e = (r.h.min(r.w) * 0.06).max(2.0).min(8.0);
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    let (ox0, oy0, ox1, oy1) = (x0 - e, y0 - e, x1 + e, y1 + e);
    let edge = mix(Color::new(0.16, 0.2, 0.34, 1.0), c, 0.35);
    quad(vec2(ox0, oy0), vec2(ox1, oy0), vec2(x1, y0), vec2(x0, y0), shade(edge, 0.3, 1.0));
    quad(vec2(ox0, oy0), vec2(x0, y0), vec2(x0, y1), vec2(ox0, oy1), shade(edge, 0.42, 1.0));
    quad(vec2(ox1, oy0), vec2(ox1, oy1), vec2(x1, y1), vec2(x1, y0), shade(edge, 0.85, 1.0));
    quad(vec2(ox0, oy1), vec2(x0, y1), vec2(x1, y1), vec2(ox1, oy1), mix(edge, WHITE, 0.25));
    // Fondo y sombra interior
    volumen::grad_v(x0, y0, r.w, r.h, Color::new(0.008, 0.012, 0.04, 0.97), Color::new(0.04, 0.05, 0.12, 0.97));
    let d = (r.h.min(r.w) * 0.16).max(4.0).min(22.0);
    let clear = Color::new(0.0, 0.0, 0.0, 0.0);
    volumen::grad_v(x0, y0, r.w, d, Color::new(0.0, 0.0, 0.0, 0.7), clear);
    volumen::grad_h(x0, y0, d * 0.8, r.h, Color::new(0.0, 0.0, 0.0, 0.5), clear);
    volumen::grad_v(x0, y1 - d, r.w, d, alpha(c, 0.0), alpha(c, 0.14));
}

/// Tecla fisica: sombra en el suelo, canto grueso, cara convexa y brillo.
fn button(r: Rect, label: &str, c: Color, enabled: bool, pulse: f32) {
    button_d(r, label, c, enabled, pulse, volumen::key_depth(r.h));
}

fn button_d(r: Rect, label: &str, c: Color, enabled: bool, pulse: f32, depth: f32) {
    let base = if enabled { c } else { mix(c, Color::new(0.3, 0.32, 0.4, 1.0), 0.72) };
    let lit = if enabled { 1.0 } else { 0.62 };
    let rad = (r.h * 0.24).min(r.w * 0.24);
    volumen::rounded_rect(r.x + depth * 0.3, r.y + depth * 1.4, r.w, r.h, rad, Color::new(0.0, 0.0, 0.0, 0.45));
    // Canto (de la cara hacia el suelo)
    volumen::rounded_grad(r.x, r.y + depth * 0.5, r.w, r.h + depth * 0.5, rad, shade(base, 0.42 * lit, 1.0), shade(base, 0.2 * lit, 1.0));
    // Filo claro y cara
    volumen::rounded_rect(r.x, r.y, r.w, r.h, rad, shade(mix(base, WHITE, 0.4), lit, 1.0));
    let e = (r.h * 0.04).max(1.5);
    volumen::rounded_grad(r.x + e, r.y + e, r.w - e * 2.0, r.h - e * 2.0, rad - e, shade(base, 0.86 * lit, 1.0), shade(base, 0.4 * lit, 1.0));
    if pulse > 0.0 {
        volumen::rounded_rect(r.x + e, r.y + e, r.w - e * 2.0, r.h - e * 2.0, rad - e, Color::new(1.0, 1.0, 1.0, 0.22 * pulse));
    }
    // Brillo curvo en la mitad superior
    let gm = r.w.min(r.h) * 0.08;
    volumen::rounded_grad(
        r.x + gm,
        r.y + e + r.h * 0.03,
        r.w - gm * 2.0,
        r.h * 0.4,
        rad * 0.75,
        Color::new(1.0, 1.0, 1.0, 0.42 * lit),
        Color::new(1.0, 1.0, 1.0, 0.03),
    );
    if !label.is_empty() {
        let a = if enabled { 1.0 } else { 0.6 };
        let size = fit_size(label, r.h * 0.42, r.w * 0.84);
        let ty = r.y + r.h / 2.0 + size * 0.32;
        text_c(label, r.x + r.w / 2.0 + 1.5, ty + 1.5, size, Color::new(0.0, 0.0, 0.0, 0.8 * a));
        text_c(label, r.x + r.w / 2.0, ty, size, alpha(WHITE, a));
    }
}

fn hash(i: u32) -> f32 {
    let mut x = i.wrapping_mul(2654435761) ^ 0x9E37_79B9;
    x ^= x >> 15;
    x = x.wrapping_mul(2246822519);
    x ^= x >> 13;
    (x % 10000) as f32 / 10000.0
}

fn draw_bg(t: f32, k: f32, flow: Vec2) {
    modo_lateral::fondo(t, k, flow);
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
    if spec.tunnel {
        for &(x, y) in cells.iter() {
            tunnel_mark(ox + (x as f32 + 0.5) * ms, oy + (y as f32 + 0.5) * ms, ms, t);
        }
    }
}

// ---------------------------------------------------------------- app

/// Momento de exito de un reto: el juego se congela un instante.
struct Eureka {
    t: f32,
    tech: Tech,
    bonus: u64,
}

struct App {
    /// Mundo actual: 0 = modo vertical, 1 = modo lateral (suavizado)
    lat_k: f32,
    lat_on: bool,
    flow: Vec2,
    /// Barrido de cambio de mundo: (progreso 0..1, entrando)
    wipe: Option<(f32, bool)>,
    sway_t: f32,
    next_total: u32,
    eureka: Option<Eureka>,
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
    active: Vec<u64>,
    /// Acertijo en juego (indice en PUZZLES)
    puzzle: Option<usize>,
    solved: Vec<bool>,
    /// Mejor puntuacion por acertijo: 0 sin resolver, 3 = solucion lateral, 1 = solucion obvia
    stars: Vec<u8>,
    last_stars: u8,
    last_used: u32,
    show_hint: bool,
    // Entrenamiento lateral en cada partida
    challenge: Option<Challenge>,
    next_ch: u32,
    last_tech: Option<Tech>,
    game_stats: Stats,
    session_stats: Stats,
    ent_flag: bool,
    /// Ultimo resultado de reto para mostrar la leccion unos segundos.
    last_result: Option<(Tech, bool, f32)>,
    /// Botones de la botonera mantenidos: (puntero, boton, tiempo)
    pad_held: Vec<(u64, usize, f32)>,
    /// Explicacion de pieza especial (0 superposicion, 1 entrelazada, 2 tunel)
    explain: Option<(u8, f32)>,
    explained: [bool; 5],
    q_intro_seen: bool,
    lat_rules_seen: bool,
    /// Pagina de la lista de niveles laterales
    page: usize,
    /// Destello al saltar de dimension (1 -> 0)
    dim_flash: f32,
    /// Mejor nivel cuantico alcanzado en la sesion
    q_best: u32,
    /// Jev (IA) esta jugando: el jugador solo mira.
    bot: bool,
    bot_t: f32,
    bot_plan: Option<(usize, usize, i32)>,
    bot_sw: usize,
    bot_for: u32,
    bot_rots: usize,
    bot_wait: u32,
    /// Jev en el mundo lateral: pasos de la solucion guardada y su avance
    jev_ops: Vec<game::Op>,
    jev_i: usize,
    jev_moves: Option<Vec<game::Mv>>,
    bot_end_t: f32,
    /// Jev real (TypeSafe) elige la jugada entre las mejores candidatas
    jev_real: bool,
    jev_asking: bool,
    jev_cands: Vec<game::BotCand>,
    jev_ticks: u32,
    jev_ok: u32,
    /// Tarjeta de explicacion del entrenamiento lateral al empezar
    intro: bool,
    intro_seen: [bool; 3],
    intro_t: f32,
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
            lat_k: 0.0,
            lat_on: false,
            flow: Vec2::ZERO,
            wipe: None,
            sway_t: 9.0,
            next_total: 5,
            eureka: None,
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
            active: Vec::new(),
            das_dir: 0,
            das_t: 0.0,
            arr_t: 0.0,
            sfx,
            menu_pieces,
            puzzle: None,
            solved: vec![false; puzzles::total()],
            stars: vec![0; puzzles::total()],
            last_stars: 0,
            last_used: 0,
            show_hint: false,
            challenge: None,
            next_ch: 5,
            last_tech: None,
            game_stats: Stats::default(),
            session_stats: Stats::default(),
            ent_flag: false,
            last_result: None,
            pad_held: Vec::new(),
            explain: None,
            explained: [false; 5],
            q_intro_seen: false,
            lat_rules_seen: false,
            page: 0,
            dim_flash: 0.0,
            q_best: 0,
            bot: false,
            bot_t: 0.0,
            bot_plan: None,
            bot_sw: 0,
            bot_for: u32::MAX,
            bot_rots: 0,
            bot_wait: 0,
            jev_ops: Vec::new(),
            jev_i: 0,
            jev_moves: None,
            bot_end_t: 0.0,
            jev_real: false,
            jev_asking: false,
            jev_cands: Vec::new(),
            jev_ticks: 0,
            jev_ok: 0,
            intro: false,
            intro_seen: [false; 3],
            intro_t: 0.0,
        }
    }

    /// Lanza un reto lateral y prepara las piezas para que sea posible.
    fn start_challenge(&mut self, l: &Lay) {
        let tech = lateral::pick(&self.session_stats, self.last_tech);
        self.last_tech = Some(tech);
        // (posicion en la cola, 0 = tunel / 1 = entrelazada / 2 = superposicion)
        let marks: &[(usize, u8)] = match tech {
            Tech::Supuesto => &[(1, 0), (3, 0)],
            Tech::Invertir => &[(0, 1), (1, 1)],
            Tech::Mirar => &[(0, 2)],
            Tech::Orden | Tech::Inesperado | Tech::Lado => &[],
        };
        for &(i, f) in marks.iter() {
            if let Some(s) = self.game.queue.get_mut(i) {
                s.superpos = f == 2;
                s.entangled = f == 1;
                s.tunnel = f == 0;
            }
        }
        if tech == Tech::Mirar && self.game.charges == 0 {
            self.game.add_charge();
        }
        let lv = lateral::level(self.mode);
        let n = (tech.pieces() as i32 + lv.extra).max(2) as u32;
        self.challenge = Some(Challenge {
            tech,
            left: n,
            total: n,
            t: 0.0,
            announcing: true,
        });
        self.last_result = None;
        self.popups.clear();
        self.block_input = true;
        let _ = l;
    }

    /// Programa cuantas piezas faltan para volver al modo lateral.
    fn schedule_next(&mut self) {
        let (a, b) = lateral::level(self.mode).every;
        self.next_ch = a + game::rand_int((b - a + 1) as i32) as u32;
        self.next_total = self.next_ch;
    }

    /// La pista del reto se muestra segun el nivel (FACIL siempre, NORMAL a mitad).
    fn hint_visible(&self) -> bool {
        match &self.challenge {
            Some(c) => match lateral::level(self.mode).hint {
                0 => true,
                1 => c.left * 2 <= c.total,
                _ => false,
            },
            None => false,
        }
    }

    /// Revisa el reto activo tras colocar una pieza.
    #[allow(clippy::too_many_arguments)]
    fn on_placed(&mut self, l: &Lay, kind: usize, rot: usize, from_hold: bool, observed: bool, tunneled: bool, slid: bool, lines: usize) {
        if self.puzzle.is_some() || self.state != State::Playing || !RETOS_EN_PARTIDA {
            self.ent_flag = false;
            return;
        }
        let ent = self.ent_flag;
        self.ent_flag = false;
        let ch = match self.challenge.as_mut() {
            Some(c) => c,
            None => {
                self.next_ch = self.next_ch.saturating_sub(1);
                if self.next_ch == 0 {
                    self.start_challenge(l);
                }
                return;
            }
        };
        let ok = match ch.tech {
            Tech::Orden => from_hold && lines > 0,
            Tech::Supuesto => tunneled,
            Tech::Invertir => ent,
            Tech::Mirar => observed && lines > 0,
            Tech::Inesperado => rot == 2 && (kind == 2 || kind == 5 || kind == 6) && lines > 0,
            Tech::Lado => slid,
        };
        ch.left = ch.left.saturating_sub(1);
        let tech = ch.tech;
        let left = ch.left;
        if ok {
            self.game_stats.add(tech, true);
            self.session_stats.add(tech, true);
            let bonus = 500 * self.game.level as u64;
            self.game.score += bonus;
            self.game.add_charge();
            self.popups.clear();
            self.eureka = Some(Eureka { t: 0.0, tech, bonus });
            self.block_input = true;
            self.sfx.play(S::Level, 0.6);
            for k in 0..20 {
                let p = vec2(l.bx + l.cell * (k % 10) as f32 + l.cell * 0.5, l.by + l.cell * 2.0);
                self.burst(p, GOLD, 5, l.cell * 7.0, l.cell * 0.18);
            }
            self.challenge = None;
            self.last_result = Some((tech, true, 0.0));
            self.schedule_next();
        } else if left == 0 {
            self.game_stats.add(tech, false);
            self.session_stats.add(tech, false);
            self.popup(l, "RETO NO SUPERADO", MAGENTA, false);
            self.challenge = None;
            self.last_result = Some((tech, false, 0.0));
            self.schedule_next();
        }
    }

    fn clear_fx(&mut self) {
        self.particles.clear();
        self.popups.clear();
        self.fades.clear();
        self.flashes.clear();
        self.shake = 0.0;
    }

    fn start_puzzle(&mut self, i: usize, hint: bool) {
        let i = i.min(puzzles::total() - 1);
        self.challenge = None;
        self.eureka = None;
        self.puzzle = Some(i);
        self.game = Game::from_puzzle(puzzles::get(i));
        self.clear_fx();
        self.show_hint = hint;
        self.new_best = false;
        self.sfx.play(S::Click, 0.5);
        self.set_state(State::Playing);
        let l = lay();
        self.popup(&l, &format!("NIVEL LATERAL {}", i + 1), GOLD, true);
        // La primera vez se muestran las reglas del mundo lateral
        self.intro = !self.lat_rules_seen;
        self.lat_rules_seen = true;
        self.intro_t = 0.0;
        self.page = i / PAGE;
        // Jev repite la solucion guardada de este nivel
        self.jev_ops = game::parse_sol(puzzles::get(i).sol);
        self.jev_i = 0;
        self.jev_moves = None;
        self.bot_wait = 6;
        self.bot_end_t = 0.0;
        FOUR_BTN.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Primer nivel lateral sin resolver: hasta ahi estan abiertos los niveles.
    fn lat_unlocked(&self) -> usize {
        self.solved.iter().position(|s| !*s).unwrap_or(self.solved.len().saturating_sub(1))
    }

    /// Despues de resolver: el siguiente nivel (o la lista si era el ultimo).
    fn pz_next(&mut self, i: usize) {
        if i + 1 < puzzles::total() {
            self.start_puzzle(i + 1, false);
        } else {
            self.set_state(State::Puzzles);
        }
    }

    /// Jev (IA) juega el MUNDO CUANTICO y el jugador mira.
    fn start_jev(&mut self) {
        self.start_quantum();
        self.bot = true;
        self.intro = false;
        self.bot_plan = None;
        self.bot_for = u32::MAX;
        self.bot_wait = 0;
        // Si hay puente configurado, el Jev real elige; si no, juega el Jev local
        self.jev_real = jev_net_available();
        self.jev_asking = false;
        self.jev_ok = 0;
    }

    /// La pregunta que se le hace al Jev real: elegir entre las mejores jugadas.
    fn jev_question(&self) -> String {
        let g = &self.game;
        let kind = g.active.map(|a| a.spec.kind).unwrap_or(0).min(6);
        let name = ["I", "O", "T", "S", "Z", "J", "L"][kind];
        let mut crit = String::new();
        for (i, c) in self.jev_cands.iter().enumerate() {
            if i > 0 {
                crit.push(',');
            }
            crit.push_str(&format!(
                "\"c{}\":\"lines cleared {}, holes {}, max height {}, bumpiness {}{}\"",
                i,
                c.lines,
                c.holes,
                c.height,
                c.bump,
                if c.sw > 0 { ", jumps to another dimension" } else { "" }
            ));
        }
        let mut q = String::new();
        q.push_str("{\"model\":\"jev-latest\",\"state\":\"Falling-block puzzle game, level ");
        q.push_str(&g.level.to_string());
        q.push_str(". Current piece: ");
        q.push_str(name);
        q.push_str(". Each candidate placement is described by the board it leaves behind.\",");
        q.push_str("\"questions\":{\"move\":{\"type\":\"choice\",\"instructions\":");
        q.push_str("\"Pick the placement that leaves the cleanest board: clear more lines, fewer holes, lower height, smoother surface.\",");
        q.push_str("\"criteria\":{");
        q.push_str(&crit);
        q.push_str("}}}}");
        q
    }

    /// Jev (IA) resuelve los niveles del MUNDO LATERAL desde el 1 y el jugador mira.
    fn start_jev_lateral(&mut self) {
        self.bot = true;
        self.start_puzzle(0, false);
        self.bot = true;
    }

    /// Jev en el mundo lateral: guarda y mueve la pieza como lo haria un jugador,
    /// siguiendo la solucion que comprobo el generador.
    fn bot_step_lateral(&mut self, dt: f32) {
        if self.game.over || self.game.active.is_none() {
            return;
        }
        self.bot_t += dt;
        if self.bot_t < 0.16 {
            return;
        }
        self.bot_t = 0.0;
        if self.bot_wait > 0 {
            self.bot_wait -= 1;
            return;
        }
        let op = match self.jev_ops.get(self.jev_i) {
            Some(o) => *o,
            None => return,
        };
        match op {
            game::Op::Hold => {
                self.game.hold();
                self.jev_i += 1;
                self.jev_moves = None;
                self.bot_wait = 5;
            }
            game::Op::Place(r, x, y) => {
                if self.jev_moves.is_none() {
                    match self.game.path_to(r, x, y) {
                        Some(mut v) => {
                            v.reverse();
                            self.jev_moves = Some(v);
                        }
                        None => {
                            self.game.hard_drop();
                            self.jev_i += 1;
                            return;
                        }
                    }
                }
                let next = self.jev_moves.as_mut().and_then(|v| v.pop());
                match next {
                    Some(m) => self.game.apply_mv(m),
                    None => {
                        self.game.hard_drop();
                        self.jev_i += 1;
                        self.jev_moves = None;
                        self.bot_wait = 6;
                    }
                }
            }
        }
    }

    /// Un paso de Jev: gira, mueve y suelta la pieza, a ritmo para poder verlo.
    fn bot_step(&mut self, dt: f32) {
        if self.bot && self.puzzle.is_some() {
            self.bot_step_lateral(dt);
            return;
        }
        if !self.bot || self.game.over {
            return;
        }
        let a = match self.game.active {
            Some(a) => a,
            None => return,
        };
        self.bot_t += dt;
        let period = if self.game.level >= 4 { 0.05 } else { 0.09 };
        if self.bot_t < period {
            return;
        }
        self.bot_t = 0.0;
        // Pieza doble: observa antes de moverla
        if a.dx.is_some() && self.game.charges > 0 {
            self.game.observe();
            self.bot_plan = None;
            return;
        }
        let locks = self.game.pieces_used();
        if self.bot_plan.is_none() || self.bot_for != locks {
            if self.jev_real {
                // Jev real: se le muestran las mejores jugadas y el elige una
                if !self.jev_asking {
                    self.jev_cands = self.game.bot_candidates(8);
                    if self.jev_cands.is_empty() {
                        return;
                    }
                    let q = self.jev_question();
                    jev_net_ask(&q);
                    self.jev_asking = true;
                    self.jev_ticks = 0;
                    self.bot_plan = None;
                    return;
                }
                self.jev_ticks += 1;
                let p = jev_net_poll();
                let idx = if p >= 0 && (p as usize) < self.jev_cands.len() {
                    Some(p as usize)
                } else if p == -2 || self.jev_ticks > 90 {
                    // Sin respuesta: juega la mejor segun la formula local
                    Some(0)
                } else {
                    None
                };
                let idx = match idx {
                    Some(i) => i,
                    None => return,
                };
                if p >= 0 {
                    self.jev_ok += 1;
                }
                let c = self.jev_cands[idx];
                self.bot_plan = Some((c.sw, c.rot, c.x));
                self.jev_asking = false;
            } else {
                self.bot_plan = self.game.bot_plan();
            }
            self.bot_for = locks;
            self.bot_rots = 0;
            self.bot_sw = 0;
            self.bot_wait = 4;
        }
        let (sw, r, xt) = match self.bot_plan {
            Some(p) => p,
            None => return,
        };
        if self.bot_wait > 0 {
            self.bot_wait -= 1;
            return;
        }
        // Primero salta a la dimension elegida (A, B, C o D)
        if self.bot_sw < sw {
            self.game.switch_dim();
            self.bot_sw += 1;
            return;
        }
        if self.bot_rots < r {
            self.game.rotate(true);
            self.bot_rots += 1;
            return;
        }
        let moved = if a.x < xt {
            self.game.shift(1)
        } else if a.x > xt {
            self.game.shift(-1)
        } else {
            false
        };
        if !moved {
            self.game.hard_drop();
        }
    }

    /// Empieza una partida del MUNDO CUANTICO desde el nivel 1.
    fn start_quantum(&mut self) {
        self.bot = false;
        self.start(Mode::Facil);
        self.game = Game::new_quantum();
        self.intro = !self.q_intro_seen;
        self.q_intro_seen = true;
        FOUR_BTN.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Salto de dimension (boton DIMENSION o tecla D).
    fn dim_press(&mut self, l: &Lay) {
        if !self.game.dims {
            self.popup(l, "DIMENSIONES: se abren en el NIVEL 4", VIOLET, false);
            self.sfx.play(S::Click, 0.3);
        } else {
            self.game.switch_dim();
        }
    }

    /// Revisa si el acertijo se resolvio o se acabaron las piezas.
    fn check_puzzle(&mut self) {
        let i = match self.puzzle {
            Some(i) => i,
            None => return,
        };
        if self.state != State::Playing {
            return;
        }
        let done = match puzzles::get(i).goal {
            Goal::ClearAll => self.game.board_empty(),
            Goal::Lines(n) => self.game.lines >= n,
        };
        if done || self.game.out_of_pieces || self.game.over {
            // Los avisos flotantes taparian la ventana de resultado.
            self.popups.clear();
        }
        if done {
            // Tres estrellas con la solucion lateral; una si resolviste "a lo obvio" con mas piezas.
            let p = puzzles::get(i);
            let used = self.game.pieces_used();
            let stars = if p.dual && used > p.par { 1 } else { 3 };
            self.last_stars = stars;
            self.last_used = used;
            if !self.bot {
                self.stars[i] = self.stars[i].max(stars);
                self.solved[i] = true;
            }
            self.sfx.play(S::Tetra, 0.7);
            let l = lay();
            for k in 0..30 {
                let p = vec2(l.bx + l.cell * (k % 10) as f32 + l.cell * 0.5, l.by + l.cell * (k / 10) as f32 * 6.0 + l.cell * 3.0);
                self.burst(p, if k % 2 == 0 { GOLD } else { GREEN }, 6, l.cell * 8.0, l.cell * 0.2);
            }
            self.set_state(State::PzWin);
        } else if self.game.out_of_pieces || self.game.over {
            self.sfx.play(S::Over, 0.5);
            self.set_state(State::PzFail);
        }
    }

    fn toggle_pad(&mut self) {
        let on = !pad_on();
        PAD_ON.store(on, std::sync::atomic::Ordering::Relaxed);
        self.sfx.play(S::Click, 0.5);
        self.pad_held.clear();
    }

    /// Accion inmediata de un boton de la botonera.
    fn pad_press(&mut self, i: usize) {
        match i {
            0 => {
                self.game.shift(-1);
            }
            1 => {
                self.game.shift(1);
            }
            2 => {
                self.game.rotate(false);
            }
            3 => {
                self.game.rotate(true);
            }
            4 => {
                self.game.soft_step();
            }
            _ => self.game.hard_drop(),
        }
    }

    fn set_state(&mut self, s: State) {
        self.state = s;
        if s == State::Menu || s == State::Puzzles {
            self.bot = false;
        }
        self.pad_held.clear();
        self.gestures.clear();
        self.block_input = true;
    }

    fn start(&mut self, mode: Mode) {
        self.puzzle = None;
        self.challenge = None;
        self.eureka = None;
        let mi = MODES.iter().position(|m| *m == mode).unwrap_or(0);
        self.intro = !self.intro_seen[mi];
        self.intro_seen[mi] = true;
        self.intro_t = 0.0;
        self.next_ch = lateral::level(mode).every.0;
        self.next_total = self.next_ch;
        self.game_stats = Stats::default();
        self.last_result = None;
        self.ent_flag = false;
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
            self.active.clear();
            return v;
        }
        if self.touch_mode {
            // En algunos navegadores/telefonos macroquad no llega a mostrar la
            // fase "Started" de un toque (solo Stationary/Moved/Ended). Por eso
            // cualquier dedo que no conociamos cuenta como un toque nuevo.
            for t in ts.iter() {
                let p = t.position;
                let known = self.active.contains(&t.id);
                match t.phase {
                    TouchPhase::Started => {
                        if !known {
                            self.active.push(t.id);
                            v.push(Ptr::Down(t.id, p));
                        }
                    }
                    TouchPhase::Moved | TouchPhase::Stationary => {
                        if !known {
                            self.active.push(t.id);
                            v.push(Ptr::Down(t.id, p));
                        } else {
                            v.push(Ptr::Move(t.id, p));
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        if !known {
                            v.push(Ptr::Down(t.id, p));
                        }
                        self.active.retain(|&id| id != t.id);
                        v.push(Ptr::Up(t.id, p));
                    }
                }
            }
            // Olvidar dedos que desaparecieron sin evento de fin.
            self.active.retain(|id| ts.iter().any(|t| t.id == *id));
        } else {
            let m = mouse_position();
            let p = vec2(m.0, m.1);
            // Un clic rapido puede pulsar y soltar en el mismo fotograma:
            // hay que entregar ambos o el gesto queda "colgado".
            let pressed = is_mouse_button_pressed(MouseButton::Left);
            let released = is_mouse_button_released(MouseButton::Left);
            if pressed {
                v.push(Ptr::Down(0, p));
            }
            if released {
                v.push(Ptr::Up(0, p));
            } else if !pressed && is_mouse_button_down(MouseButton::Left) {
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
                self.sfx.music(false, false);
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
            State::Puzzles => self.update_puzzles(&ptrs, &l),
            State::PzWin => self.update_pzwin(&ptrs, &l),
            State::PzFail => self.update_pzfail(&ptrs, &l),
        }
        self.update_fx(dt, &l);
        self.update_world(dt);
        // Jev sigue solo: al ganar pasa al nivel siguiente, y si fallara lo reintenta
        if self.bot && self.puzzle.is_some() && (self.state == State::PzWin || self.state == State::PzFail) {
            self.bot_end_t += dt;
            if self.bot_end_t > 3.5 {
                self.bot_end_t = 0.0;
                let i = self.puzzle.unwrap_or(0);
                if self.state == State::PzWin {
                    self.pz_next(i);
                } else {
                    self.start_puzzle(i, false);
                }
            }
        }
        let playing = self.state == State::Playing;
        self.sfx.music(playing, self.lat_on);
    }

    /// Cambio de mundo: decide el modo y anima fondo, barrido y balanceo.
    fn update_world(&mut self, dt: f32) {
        if self.state == State::Over {
            // Un game over a mitad de reto termina el modo lateral.
            self.challenge = None;
            self.eureka = None;
        }
        let target = match self.state {
            State::Menu | State::Over => false,
            State::Puzzles => true,
            _ => self.challenge.is_some() || self.eureka.is_some() || self.puzzle.is_some(),
        };
        if target != self.lat_on {
            self.lat_on = target;
            if self.state != State::Over {
                self.wipe = Some((0.0, target));
                self.sway_t = 0.0;
                self.sfx.play(if target { S::LatIn } else { S::LatOut }, 0.6);
            }
        }
        let goal = if target { 1.0 } else { 0.0 };
        self.lat_k += (goal - self.lat_k) * (1.0 - (-dt * 5.0).exp());
        if (goal - self.lat_k).abs() < 0.002 {
            self.lat_k = goal;
        }
        self.flow += modo_lateral::direccion(self.lat_k) * dt;
        self.sway_t += dt;
        if let Some(w) = self.wipe.as_mut() {
            w.0 += dt / modo_lateral::BARRIDO;
            if w.0 >= 1.0 {
                self.wipe = None;
            }
        }
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
                let jr = jev_rects();
                if jr[0].contains(pos) {
                    self.start_jev();
                    return;
                }
                if jr[1].contains(pos) {
                    self.start_jev_lateral();
                    return;
                }
                if rs[0].contains(pos) {
                    self.start_quantum();
                    return;
                }
                if rs[1].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.page = self.lat_unlocked() / PAGE;
                    self.set_state(State::Puzzles);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Key4) {
            self.start_jev_lateral();
        } else if is_key_pressed(KeyCode::Key3) {
            self.start_jev();
        } else if is_key_pressed(KeyCode::Key2) {
            self.page = self.lat_unlocked() / PAGE;
            self.set_state(State::Puzzles);
        } else if is_key_pressed(KeyCode::Key1) || is_key_pressed(KeyCode::Enter) {
            self.start_quantum();
        }
    }

    fn update_play(&mut self, dt: f32, ptrs: &[Ptr], l: &Lay) {
        // Tarjeta de explicacion o anuncio de reto: el juego espera un toque.
        let tap = ptrs.iter().any(|p| matches!(p, Ptr::Down(..)))
            || is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter);
        if self.intro {
            self.intro_t += dt;
            if (tap && self.intro_t > 0.4) || (self.bot && self.intro_t > 1.8) {
                self.intro = false;
                self.block_input = true;
                self.sfx.play(S::Click, 0.5);
            }
            return;
        }
        // Eureka: congela el juego un instante y vuelve solo.
        if let Some(e) = self.eureka.as_mut() {
            e.t += dt;
            if e.t > 1.6 || (tap && e.t > 0.7) {
                self.eureka = None;
                self.block_input = true;
            }
            return;
        }
        if let Some(c) = self.challenge.as_mut() {
            if c.announcing {
                c.t += dt;
                if (tap && c.t > 0.4) || c.t > lateral::ANNOUNCE {
                    c.announcing = false;
                    c.t = 0.0;
                    self.block_input = true;
                    self.sway_t = 0.0;
                    self.sfx.play(S::Click, 0.5);
                }
                return;
            }
        }
        // Primera vez que aparece una pieza especial: explicar que es y como se usa
        if let Some((k, t)) = self.explain.as_mut() {
            *t += dt;
            let _ = k;
            if (tap && *t > 0.4) || (self.bot && *t > 2.4) {
                self.explain = None;
                self.block_input = true;
                self.sfx.play(S::Click, 0.5);
            }
            return;
        }
        if let Some(a) = self.game.active {
            let kind = if a.dx.is_some() {
                Some(0u8)
            } else if a.spec.entangled {
                Some(1)
            } else if a.spec.tunnel {
                Some(2)
            } else {
                None
            };
            if let Some(k) = kind {
                // En acertijos no se explica: descubrirlo es parte del reto.
                if !self.explained[k as usize] && self.puzzle.is_none() {
                    self.explained[k as usize] = true;
                    self.explain = Some((k, 0.0));
                    self.block_input = true;
                    self.sfx.play(S::Entangle, 0.5);
                    return;
                }
            }
        }
        let cell = l.cell;
        let now = get_time();
        for p in ptrs {
            match *p {
                Ptr::Down(id, pos) => {
                    // Un gesto viejo con el mismo id quedo sin final: se descarta.
                    self.gestures.remove(&id);
                    self.pad_held.retain(|h| h.0 != id);
                    if self.sound_hit(pos, l) {
                        self.gestures.insert(id, Gesture::new(pos, now, true));
                        continue;
                    }
                    if l.dim.map(|r| r.contains(pos)).unwrap_or(false) {
                        self.gestures.insert(id, Gesture::new(pos, now, true));
                        self.dim_press(l);
                        continue;
                    }
                    if let Some(i) = l.buttons.iter().position(|r| r.contains(pos)) {
                        self.gestures.insert(id, Gesture::new(pos, now, true));
                        match i {
                            0 => {
                                if !self.game.hold() {
                                    self.popup(l, "Ya guardaste esta pieza: suelta una primero", MAGENTA, false);
                                }
                            }
                            1 if self.puzzle.is_some() => {
                                self.show_hint = true;
                                self.sfx.play(S::Click, 0.5);
                            }
                            1 => {
                                if !self.game.observe() {
                                    let msg = if self.game.charges == 0 {
                                        "Sin cargas de OBSERVAR: sube de nivel para ganar"
                                    } else {
                                        "OBSERVAR solo sirve con pieza DOBLE (2 copias con onda)"
                                    };
                                    self.popup(l, msg, GOLD, false);
                                    self.sfx.play(S::Click, 0.3);
                                }
                            }
                            _ => {
                                self.sfx.play(S::Click, 0.5);
                                self.set_state(State::Paused);
                                return;
                            }
                        }
                        continue;
                    }
                    // Botonera de control (un poco mas grande que el dibujo para dedos)
                    if let Some(pad) = l.pad {
                        if let Some(i) = pad.iter().position(|r| {
                            Rect::new(r.x - 3.0, r.y - 6.0, r.w + 6.0, r.h + 12.0).contains(pos)
                        }) {
                            self.gestures.insert(id, Gesture::new(pos, now, true));
                            self.pad_press(i);
                            self.pad_held.push((id, i, 0.0));
                            continue;
                        }
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
                    self.pad_held.retain(|h| h.0 != id);
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
        // Botonera: soltar dedos perdidos y repetir movimiento al mantener
        let touch = self.touch_mode;
        let mouse_down = is_mouse_button_down(MouseButton::Left);
        let alive: Vec<u64> = self.active.clone();
        self.pad_held.retain(|h| if touch { alive.contains(&h.0) } else { mouse_down });
        let mut pad_soft = false;
        let mut shifts = 0i32;
        for h in self.pad_held.iter_mut() {
            let prev = h.2;
            h.2 += dt;
            match h.1 {
                0 | 1 => {
                    let (d, r) = (0.17f32, 0.05f32);
                    if h.2 > d {
                        let before = if prev > d { ((prev - d) / r).floor() as i32 } else { -1 };
                        let after = ((h.2 - d) / r).floor() as i32;
                        let n = after - before;
                        shifts += if h.1 == 0 { -n } else { n };
                    }
                }
                4 => pad_soft = true,
                _ => {}
            }
        }
        while shifts != 0 {
            let dir = shifts.signum();
            self.game.shift(dir);
            shifts -= dir;
        }
        self.bot_step(dt);
        if self.puzzle.is_none() {
            if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Tab) {
                self.dim_press(l);
            }
            if is_key_pressed(KeyCode::F2) {
                self.game.debug_level_up();
            }
        }
        self.dim_flash = (self.dim_flash - dt * 2.5).max(0.0);
        let soft = is_key_down(KeyCode::Down) || pad_soft;
        // Tiempo lento dentro del reto: mas margen para pensar.
        let gdt = if !soft && self.challenge.is_some() { dt * lateral::level(self.mode).slow } else { dt };
        self.game.update(gdt, soft);
        self.process_events(l);
        self.check_puzzle();
        if let Some(c) = self.challenge.as_mut() {
            c.t += dt;
        }
        if let Some(r) = self.last_result.as_mut() {
            r.2 += dt;
            if r.2 > 7.0 {
                self.last_result = None;
            }
        }
    }

    fn update_puzzles(&mut self, ptrs: &[Ptr], l: &Lay) {
        let ll = level_rects();
        let total = puzzles::total();
        let pages = (total + PAGE - 1) / PAGE;
        let open = self.lat_unlocked();
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if ll.prev.contains(pos) && self.page > 0 {
                    self.page -= 1;
                    self.sfx.play(S::Click, 0.5);
                    return;
                }
                if ll.next.contains(pos) && self.page + 1 < pages {
                    self.page += 1;
                    self.sfx.play(S::Click, 0.5);
                    return;
                }
                for (k, r) in ll.cells.iter().enumerate() {
                    let i = self.page * PAGE + k;
                    if i < total && r.contains(pos) {
                        // Los niveles se abren en orden: hasta el primero sin resolver
                        if i <= open {
                            self.start_puzzle(i, false);
                        } else {
                            self.sfx.play(S::Click, 0.25);
                        }
                        return;
                    }
                }
                let back = ll.back;
                if back.contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Menu);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            self.set_state(State::Menu);
        } else if is_key_pressed(KeyCode::Enter) {
            self.start_puzzle(open, false);
        }
    }

    fn update_pzwin(&mut self, ptrs: &[Ptr], l: &Lay) {
        // Con una estrella hay un boton mas: BUSCAR LA LATERAL (volver a intentarlo)
        let retry = self.last_stars < 3;
        let rs = overlay_rects(if retry { 3 } else { 2 });
        let (b_next, b_list) = if retry { (1, 2) } else { (0, 1) };
        let i = self.puzzle.unwrap_or(0);
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if retry && rs[0].contains(pos) {
                    self.start_puzzle(i, false);
                    return;
                }
                if rs[b_next].contains(pos) {
                    self.pz_next(i);
                    return;
                }
                if rs[b_list].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Puzzles);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            self.pz_next(i);
        }
    }

    fn update_pzfail(&mut self, ptrs: &[Ptr], l: &Lay) {
        let rs = overlay_rects(3);
        let i = self.puzzle.unwrap_or(0);
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if rs[0].contains(pos) {
                    let h = self.show_hint;
                    self.start_puzzle(i, h);
                    return;
                }
                if rs[1].contains(pos) {
                    self.start_puzzle(i, true);
                    return;
                }
                if rs[2].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Puzzles);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            let h = self.show_hint;
            self.start_puzzle(i, h);
        }
    }

    fn update_pause(&mut self, ptrs: &[Ptr], l: &Lay) {
        if let Some(i) = self.puzzle {
            let rs = overlay_rects(5);
            for p in ptrs {
                if let Ptr::Down(_, pos) = *p {
                    if rs[4].contains(pos) {
                        self.toggle_pad();
                        return;
                    }
                }
            }
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
                        self.show_hint = true;
                        self.sfx.play(S::Entangle, 0.5);
                        self.set_state(State::Playing);
                        return;
                    }
                    if rs[2].contains(pos) {
                        let h = self.show_hint;
                        self.start_puzzle(i, h);
                        return;
                    }
                    if rs[3].contains(pos) {
                        self.sfx.play(S::Click, 0.5);
                        self.set_state(State::Puzzles);
                        return;
                    }
                }
            }
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Enter) {
                self.set_state(State::Playing);
            }
            return;
        }
        let rs = overlay_rects(4);
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
                    self.start_quantum();
                    return;
                }
                if rs[2].contains(pos) {
                    self.toggle_pad();
                    continue;
                }
                if rs[3].contains(pos) {
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
        let rs = over_rects();
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if rs[0].contains(pos) {
                    self.start_quantum();
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
            self.start_quantum();
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
                    self.popups.clear();
                    self.popup(l, &format!("NIVEL CUANTICO {}", n), GREEN, true);
                    self.popup(l, game::qlevel_name(n), CYAN, true);
                    self.q_best = self.q_best.max(n);
                    // La simetria se explica al abrirse (nivel 6)
                    if n == 6 && !self.explained[4] {
                        self.explained[4] = true;
                        self.explain = Some((4, 0.0));
                        self.block_input = true;
                    }
                    self.sfx.play(S::Level, 0.5);
                    for k in 0..20 {
                        let p = vec2(l.bx + s * (k % 10) as f32 + s * 0.5, l.by + s * 2.0);
                        self.burst(p, if k % 2 == 0 { CYAN } else { VIOLET }, 4, s * 7.0, s * 0.18);
                    }
                }
                Event::DimOpen => {
                    // Primera vez: el juego se detiene y explica las dimensiones
                    if !self.explained[3] {
                        self.explained[3] = true;
                        self.explain = Some((3, 0.0));
                        self.block_input = true;
                    }
                    self.dim_flash = 1.0;
                    self.sfx.play(S::LatIn, 0.6);
                }
                Event::DimSwitch => {
                    self.dim_flash = 1.0;
                    self.sway_t = 0.0;
                    self.sfx.play(S::Tunnel, 0.45);
                }
                Event::Symmetry(n) => {
                    self.popup(l, &format!("SIMETRIA x{}", n), MAGENTA, true);
                    self.shake = self.shake.max(s * 0.25);
                    self.sfx.play(S::Entangle, 0.6);
                }
                Event::DimBlocked => {
                    self.popup(l, "BLOQUEADO: la pieza no cabe en la otra dimension", MAGENTA, false);
                    self.shake = self.shake.max(s * 0.15);
                    self.sfx.play(S::Click, 0.4);
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
                Event::Placed { kind, rot, from_hold, observed, tunneled, slid, lines } => {
                    self.on_placed(l, kind, rot, from_hold, observed, tunneled, slid, lines);
                }
                Event::Entangled { cells } => {
                    self.ent_flag = true;
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
                Event::Tunnel { kind, rot, x, y0, y1, points } => {
                    let c = pieces::color(kind);
                    let cells = pieces::cells(kind, rot);
                    // Silueta fantasma donde estaba + estela que atraviesa los bloques.
                    let mut fc = Vec::new();
                    for &(cx, cy) in cells.iter() {
                        fc.push((x + cx, y0 + cy));
                    }
                    self.fades.push(Fade { cells: fc, c: TUNNEL_C, t: 0.0 });
                    for yy in y0..=y1 {
                        for &(cx, cy) in cells.iter() {
                            let p = App::cell_center(l, x + cx, yy + cy);
                            self.burst(p, TUNNEL_C, 2, s * 2.5, s * 0.12);
                        }
                    }
                    for &(cx, cy) in cells.iter() {
                        let p = App::cell_center(l, x + cx, y1 + cy);
                        self.burst(p, c, 6, s * 5.0, s * 0.16);
                        self.burst(p, WHITE, 2, s * 3.0, s * 0.1);
                    }
                    self.shake = self.shake.max(s * 0.2);
                    self.popup(l, &format!("TUNEL +{}", points), TUNNEL_C, false);
                    self.sfx.play(S::Tunnel, 0.6);
                }
                Event::GameOver if self.puzzle.is_some() => {
                    // En acertijo, check_puzzle lo trata como "no funciono".
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
        draw_bg(self.time, self.lat_k, self.flow);
        match self.state {
            State::Menu => self.draw_menu(),
            State::Puzzles => self.draw_puzzles(),
            _ => {
                self.draw_game(&l);
                if self.state == State::Paused {
                    self.draw_pause();
                } else if self.state == State::Over {
                    self.draw_over();
                } else if self.state == State::PzWin {
                    self.draw_pzwin();
                } else if self.state == State::PzFail {
                    self.draw_pzfail();
                } else if self.state == State::Playing {
                    if self.intro {
                        self.draw_intro();
                    } else if let Some(e) = &self.eureka {
                        self.draw_eureka(e);
                    } else if let Some((k, _)) = self.explain {
                        self.draw_explain(k);
                    } else if let Some(c) = &self.challenge {
                        if c.announcing {
                            self.draw_announce();
                        }
                    }
                }
            }
        }
        self.draw_fx();
        if let Some((p, entra)) = self.wipe {
            modo_lateral::barrido(p, entra);
        }
        draw_sound_icon(l.sound, self.sfx.on);
    }

    /// Menu: dos mundos puros, cada uno con su tarjeta ilustrada.
    fn draw_menu(&self) {
        let (w, h) = (screen_width(), screen_height());
        let t = self.time;
        let ms = w * 0.05;
        for mp in self.menu_pieces.iter() {
            let col = pieces::color(mp.kind);
            let bx = mp.x * w;
            let by = mp.y * h;
            let a = if mp.sup { 0.10 + 0.08 * (t * 8.0 + mp.x * 10.0).sin() } else { 0.14 };
            for &(cx, cy) in pieces::cells(mp.kind, mp.rot).iter() {
                draw_block(bx + cx as f32 * ms, by + cy as f32 * ms, ms, col, a);
                if mp.sup {
                    draw_block(bx + cx as f32 * ms + ms * 4.5, by + cy as f32 * ms, ms, col, a);
                }
            }
        }
        // Titulo en bloques: cada letra del color de una pieza
        let title = "CUANTRIS";
        let tc = (w * 0.88 / volumen::block_text_width(title, 1.0)).min(h * 0.016).max(2.0);
        let tx0 = w / 2.0 - volumen::block_text_width(title, tc) / 2.0;
        let ty = h * 0.055;
        for (i, ch) in title.chars().enumerate() {
            let wob = (t * 2.0 + i as f32 * 0.6).sin() * tc * 0.5;
            let lx = tx0 + i as f32 * 6.0 * tc + 2.5 * tc;
            volumen::block_text_c(&ch.to_string(), lx, ty + wob, tc, pieces::color(i % 7));
        }
        let by = ty + tc * 8.6;
        let ss = fit_size("ELIGE TU MUNDO", tc * 2.4, w * 0.8);
        text_c("ELIGE TU MUNDO", w / 2.0, by + ss * 0.7, ss, alpha(WHITE, 0.85));
        wavy_line(vec2(w * 0.15, by + ss * 1.35), vec2(w * 0.85, by + ss * 1.35), tc * 0.8, t, 2.0, alpha(CYAN, 0.6));

        let rs = menu_rects();
        let open = self.lat_unlocked();
        let n_ok = self.solved.iter().filter(|s| **s).count();
        let n_st: u32 = self.stars.iter().map(|s| *s as u32).sum();
        let q_line = if self.q_best > 0 {
            format!("Tu mejor nivel: {}", self.q_best)
        } else {
            "8 niveles cuanticos".to_string()
        };
        let l_line = format!("Nivel {} de {}", open + 1, puzzles::total());
        let l_line2 = format!("{} resueltos, {} estrellas", n_ok, n_st);
        // (nombre, dos lineas de descripcion, dos lineas de progreso, color)
        let cards = [
            ("CUANTICO", ["Las piezas caen y la", "realidad se desdobla"], [q_line.as_str(), "4 dimensiones y simetria"], CYAN),
            ("LATERAL", ["Sin reloj. Lo obvio", "no basta: piensa de lado"], [l_line.as_str(), l_line2.as_str()], GOLD),
        ];
        for (i, r) in rs.iter().enumerate() {
            let (name, desc, line, c) = cards[i];
            let pulse = ((t * 2.0 + i as f32 * 1.7).sin() * 0.5 + 0.5) * 0.25;
            button(*r, "", c, true, pulse);
            // Ilustracion a la izquierda dentro de una vitrina
            let iw = r.h * 0.62;
            let ir = Rect::new(r.x + r.h * 0.13, r.y + (r.h - iw) / 2.0, iw, iw);
            panel(ir, c);
            let u = iw / 6.5;
            if i == 0 {
                // Dos dimensiones superpuestas + pieza en superposicion
                for k in 0..2 {
                    let off = if k == 0 { -u * 0.9 } else { u * 0.9 };
                    let a = 0.55 + 0.3 * (t * 6.0 + k as f32 * PI).sin();
                    for &(cx, cy) in pieces::cells(2, 0).iter() {
                        draw_block(ir.x + iw / 2.0 - u * 1.5 + off + cx as f32 * u, ir.y + u * 0.9 + cy as f32 * u, u, pieces::color(2), a);
                    }
                }
                for x in 0..5 {
                    let px = ir.x + u * 0.75 + x as f32 * u;
                    let py = ir.y + iw - u * 1.9;
                    if x == 2 {
                        let m = u * 0.14;
                        draw_rectangle_lines(px + m, py + m, u - 2.0 * m, u - 2.0 * m, 2.0, alpha(MAGENTA, 0.6 + 0.4 * (t * 3.0).sin().abs()));
                        draw_line(px + m, py + u - m, px + u - m, py + m, 1.0, alpha(MAGENTA, 0.7));
                    } else {
                        draw_block(px, py, u, pieces::color(0), 1.0);
                    }
                }
            } else {
                // Pieza que entra de lado bajo un saliente + estrellas
                let gx = ir.x + u * 0.75;
                let gy = ir.y + u * 1.0;
                let grey = pieces::color(7);
                for x in 0..5 {
                    draw_block(gx + x as f32 * u, gy + u * 2.4, u, grey, 1.0);
                }
                draw_block(gx + u * 3.0, gy + u * 0.4, u, grey, 1.0);
                draw_block(gx + u * 4.0, gy + u * 0.4, u, grey, 1.0);
                let sl = ((t * 0.8).fract() * 1.4).min(1.0);
                let sx = gx + u * (0.0 + 2.0 * sl);
                draw_block(sx, gy + u * 1.4, u, pieces::color(3), 1.0);
                draw_block(sx + u, gy + u * 1.4, u, pieces::color(3), 1.0);
                modo_lateral::chevrones(gx + u * 1.0, gy + u * 0.6, u * 0.32, 1.0, 3, t, alpha(GOLD, 0.95));
                for k in 0..3 {
                    draw_star(ir.x + iw / 2.0 + (k as f32 - 1.0) * u * 1.5, ir.y + iw - u * 0.95, u * 0.55, true);
                }
            }
            // Textos a la derecha
            let tx = ir.x + iw + r.h * 0.1;
            let tw = r.x + r.w - tx - r.h * 0.1;
            let dark = Color::new(0.0, 0.0, 0.0, 0.8);
            let s0 = fit_size("MUNDO", r.h * 0.1, tw);
            draw_text("MUNDO", tx, r.y + r.h * 0.15, s0, alpha(WHITE, 0.85));
            let s1 = fit_size(name, r.h * 0.22, tw);
            draw_text(name, tx + 2.0, r.y + r.h * 0.33 + 2.0, s1, dark);
            draw_text(name, tx, r.y + r.h * 0.33, s1, WHITE);
            let s2 = desc.iter().map(|d| fit_size(d, r.h * 0.1, tw)).fold(f32::MAX, f32::min);
            for (k, d) in desc.iter().enumerate() {
                let yy = r.y + r.h * (0.47 + 0.1 * k as f32);
                draw_text(d, tx + 1.0, yy + 1.0, s2, dark);
                draw_text(d, tx, yy, s2, alpha(WHITE, 0.97));
            }
            let s3 = line.iter().map(|d| fit_size(d, r.h * 0.09, tw)).fold(f32::MAX, f32::min);
            for (k, d) in line.iter().enumerate() {
                let yy = r.y + r.h * (0.69 + 0.09 * k as f32);
                draw_text(d, tx + 1.0, yy + 1.0, s3, dark);
                draw_text(d, tx, yy, s3, Color::new(1.0, 0.97, 0.75, 1.0));
            }
            let go = "JUGAR >";
            let s4 = fit_size(go, r.h * 0.12, tw);
            let p = (t * 3.0 + i as f32).sin() * 0.5 + 0.5;
            draw_text(go, tx + 1.5, r.y + r.h * 0.94 + 1.5, s4, dark);
            draw_text(go, tx, r.y + r.h * 0.94, s4, alpha(WHITE, 0.7 + 0.3 * p));
        }
        // Boton JEV: la IA juega el mundo cuantico y tu la miras
        let jrs = jev_rects();
        let jp = (t * 2.5).sin() * 0.5 + 0.5;
        let jlabels = [("JEV", "juega el CUANTICO", CYAN), ("JEV", "resuelve el LATERAL", GOLD)];
        for (ji, jr) in jrs.iter().enumerate() {
            let (jn, jd, jc) = jlabels[ji];
            button(*jr, "", mix(VIOLET, jc, 0.35), true, 0.2 + 0.3 * jp);
            let chip = Rect::new(jr.x + jr.h * 0.14, jr.y + jr.h * 0.2, jr.h * 0.58, jr.h * 0.58);
            panel(chip, jc);
            text_c("IA", chip.x + chip.w / 2.0, chip.y + chip.h * 0.68, fit_size("IA", chip.h * 0.55, chip.w * 0.8), WHITE);
            let jx = chip.x + chip.w + jr.h * 0.14;
            let jw = jr.x + jr.w - jx - jr.h * 0.1;
            let j1 = fit_size(jn, jr.h * 0.36, jw);
            draw_text(jn, jx + 1.5, jr.y + jr.h * 0.44 + 1.5, j1, Color::new(0.0, 0.0, 0.0, 0.8));
            draw_text(jn, jx, jr.y + jr.h * 0.44, j1, WHITE);
            let j2 = fit_size(jd, jr.h * 0.2, jw);
            draw_text(jd, jx, jr.y + jr.h * 0.78, j2, alpha(WHITE, 0.95));
        }
        let jr = jrs[0];
        let foot = "Cada mundo es puro: en uno piensas en cuantico, en el otro de lado";
        let fs = fit_size(foot, (w * 0.04).min(h * 0.022), w * 0.94);
        text_c(foot, w / 2.0, (jr.y + jr.h + h * 0.04).min(h * 0.985), fs, alpha(WHITE, 0.7));
    }

    fn draw_game(&self, l: &Lay) {
        let w = screen_width();
        // Cabecera: placa en relieve con canto y sombra
        let ed = (l.top * 0.1).max(3.0);
        volumen::grad_v(0.0, l.top + ed, w, ed * 2.5, Color::new(0.0, 0.0, 0.0, 0.5), Color::new(0.0, 0.0, 0.0, 0.0));
        draw_rectangle(0.0, l.top, w, ed, Color::new(0.04, 0.06, 0.13, 1.0));
        volumen::grad_v(0.0, 0.0, w, l.top, Color::new(0.2, 0.27, 0.44, 1.0), Color::new(0.08, 0.11, 0.22, 1.0));
        draw_rectangle(0.0, 0.0, w, 2.0, Color::new(1.0, 1.0, 1.0, 0.35));
        draw_line(0.0, l.top, w, l.top, 2.0, alpha(CYAN, 0.6));
        if let Some(i) = self.puzzle {
            self.header_tint(l);
            let p = puzzles::get(i);
            let name = format!("NIVEL {}", i + 1);
            let ts = fit_size(&name, l.top * 0.5, w * 0.5);
            text_3d_c(&name, w / 2.0, l.top * 0.7, ts, GOLD);
            let tag = if self.bot { "JEV IA JUEGA".to_string() } else { format!("DE {}", puzzles::total()) };
            let ms = fit_size(&tag, l.top * 0.22, w * 0.2);
            draw_text(&tag, w * 0.03, l.top * 0.82, ms, if self.bot { VIOLET } else { alpha(WHITE, 0.6) });
            let mo = fit_size("LATERAL", l.top * 0.26, w * 0.19);
            draw_text("LATERAL", w * 0.03, l.top * 0.5, mo, GOLD);
            // Meta y pista entre el titulo y el tablero
            let fr = (l.cell * 0.3).max(4.0);
            let space = (l.by - fr - l.top).max(l.cell * 1.2);
            let lines_n = if self.show_hint { 2.6 } else { 1.4 };
            let gs = (space / lines_n).min(l.cell * 0.7);
            let mut goal = p.goal.text();
            if p.dual {
                // Hay una salida obvia y otra lateral que gasta menos piezas
                goal = format!("{} - 3 estrellas con {} pzs", goal, p.par);
            }
            let gsz = fit_size(&goal, gs, w * 0.94);
            let mut y = l.by - fr - 6.0 - if self.show_hint { gs * 1.2 } else { 0.0 };
            text_c(&goal, w / 2.0 + 1.0, y + 1.0, gsz, Color::new(0.0, 0.0, 0.0, 0.7));
            text_c(&goal, w / 2.0, y, gsz, WHITE);
            if self.show_hint {
                y += gs * 1.2;
                let hint = format!("Pista: {}", p.hint);
                let hsz = fit_size(&hint, gs * 0.85, w * 0.94);
                text_c(&hint, w / 2.0, y, hsz, GOLD);
            }
        } else if RETOS_EN_PARTIDA {
            self.draw_mode_header(l);
        } else {
            self.draw_quantum_header(l);
        }
        let off = vec2(
            (self.time * 91.0).sin() * self.shake + modo_lateral::balanceo(self.sway_t, l.cell),
            (self.time * 77.0).cos() * self.shake,
        );
        self.draw_board(l, off);
        self.draw_side(l);
        if self.puzzle.is_none() {
            if RETOS_EN_PARTIDA {
                self.draw_lateral_banner(l);
            } else {
                self.draw_quantum_banner(l);
            }
        }
        // Boton DIMENSION: salta entre los dos pozos (se abre en el nivel 4)
        if let Some(r) = l.dim {
            let gq = &self.game;
            let (label, c) = if !gq.dims {
                ("DIMENSION".to_string(), VIOLET)
            } else {
                (format!("DIMENSION {}", DIMN[gq.dim as usize % 4]), DIMC[gq.dim as usize % 4])
            };
            button(r, &label, c, gq.dims, self.dim_flash);
            if !gq.dims {
                let cap = "se abre en nivel 4";
                let cs = fit_size(cap, r.h * 0.2, r.w * 0.9);
                text_c(cap, r.x + r.w / 2.0, r.y + r.h - cs * 0.35, cs, alpha(VIOLET, 0.9));
            }
        }
        let g = &self.game;
        let can_hold = !g.hold_used && g.active.is_some();
        let can_obs = g.charges > 0 && g.active.map(|a| a.dx.is_some()).unwrap_or(false);
        button(l.buttons[0], "GUARDAR", MAGENTA, can_hold, 0.0);
        // En acertijo el boton no parpadea: descubrir que hay que observar es parte del reto.
        let pulse = if can_obs && !g.puzzle { (self.time * 7.0).sin() * 0.5 + 0.5 } else { 0.0 };
        if g.puzzle {
            // Mundo lateral: no hay piezas cuanticas; el boton central da la pista
            button(l.buttons[1], "PISTA", GOLD, !self.show_hint, 0.0);
        } else {
            button(l.buttons[1], &format!("OBSERVAR {}", g.charges), GOLD, can_obs, pulse);
        }
        // Explicar por que esta apagado
        if !can_obs && !g.puzzle {
            let r = l.buttons[1];
            let cap = if g.charges == 0 { "sin cargas" } else { "solo con pieza doble" };
            let cs = fit_size(cap, r.h * 0.2, r.w * 0.9);
            text_c(cap, r.x + r.w / 2.0, r.y + r.h - cs * 0.35, cs, alpha(GOLD, 0.75));
        }
        if !can_hold && g.active.is_some() && g.hold_used {
            let r = l.buttons[0];
            let cap = "ya usado";
            let cs = fit_size(cap, r.h * 0.2, r.w * 0.9);
            text_c(cap, r.x + r.w / 2.0, r.y + r.h - cs * 0.35, cs, alpha(MAGENTA, 0.8));
        }
        button(l.buttons[2], if g.puzzle { "OPCIONES" } else { "PAUSA" }, CYAN, true, 0.0);
        if let Some(pad) = l.pad {
            self.draw_pad(&pad);
        }
        // Pista visual: ilumina el boton clave del reto (segun el nivel)
        if self.hint_visible() {
            if let Some(c) = &self.challenge {
                let r = match c.tech.key_button() {
                    Some(lateral::KeyBtn::Hold) => Some(l.buttons[0]),
                    Some(lateral::KeyBtn::Observe) => Some(l.buttons[1]),
                    Some(lateral::KeyBtn::Rotate) => l.pad.map(|p| p[3]),
                    None => None,
                };
                if let Some(r) = r {
                    let p = (self.time * 6.0).sin() * 0.5 + 0.5;
                    let e = 3.0 + 3.0 * p;
                    draw_rectangle_lines(r.x - e, r.y - e, r.w + 2.0 * e, r.h + 2.0 * e + 4.0, 3.0, alpha(GREEN, 0.5 + 0.5 * p));
                }
            }
        }
    }

    fn draw_board(&self, l: &Lay, off: Vec2) {
        let g = &self.game;
        let s = l.cell;
        let bx = l.bx + off.x;
        let by = l.by + off.y;
        let bw = s * W as f32;
        let bh = s * (H - HIDDEN) as f32;
        let t = self.time;

        let pulse = (t * 1.5).sin() * 0.5 + 0.5;
        // Marco en relieve alrededor del pozo
        let fr = (s * 0.3).max(4.0);
        let (ox0, oy0, ox1, oy1) = (bx - fr, by - fr, bx + bw + fr, by + bh + fr);
        draw_rectangle(ox0 + fr * 0.8, oy0 + fr * 1.1, ox1 - ox0, oy1 - oy0, Color::new(0.0, 0.0, 0.0, 0.55));
        let frame = Color::new(0.2, 0.3, 0.5, 1.0);
        // Moldura exterior (saliente) y labio interior (entrante)
        let (mx0, my0, mx1, my1) = (bx - fr * 0.42, by - fr * 0.42, bx + bw + fr * 0.42, by + bh + fr * 0.42);
        quad(vec2(ox0, oy0), vec2(ox1, oy0), vec2(mx1, my0), vec2(mx0, my0), mix(frame, WHITE, 0.45));
        quad(vec2(ox0, oy0), vec2(mx0, my0), vec2(mx0, my1), vec2(ox0, oy1), mix(frame, WHITE, 0.15));
        quad(vec2(ox1, oy0), vec2(ox1, oy1), vec2(mx1, my1), vec2(mx1, my0), shade(frame, 0.55, 1.0));
        quad(vec2(ox0, oy1), vec2(mx0, my1), vec2(mx1, my1), vec2(ox1, oy1), shade(frame, 0.35, 1.0));
        quad(vec2(mx0, my0), vec2(mx1, my0), vec2(bx + bw, by), vec2(bx, by), shade(frame, 0.2, 1.0));
        quad(vec2(mx0, my0), vec2(bx, by), vec2(bx, by + bh), vec2(mx0, my1), shade(frame, 0.3, 1.0));
        quad(vec2(mx1, my0), vec2(mx1, my1), vec2(bx + bw, by + bh), vec2(bx + bw, by), shade(frame, 0.8, 1.0));
        quad(vec2(mx0, my1), vec2(bx, by + bh), vec2(bx + bw, by + bh), vec2(mx1, my1), mix(frame, WHITE, 0.2));
        draw_rectangle_lines(ox0, oy0, ox1 - ox0, oy1 - oy0, 1.5, Color::new(0.3, 0.8, 1.0, 0.3 + 0.25 * pulse));
        // Remaches en las esquinas de la moldura
        let rv = (fr * 0.2).max(1.2);
        let rm = fr * 0.71;
        for &(rx, ry) in [(bx - rm, by - rm), (bx + bw + rm, by - rm), (bx - rm, by + bh + rm), (bx + bw + rm, by + bh + rm)].iter() {
            draw_circle(rx, ry, rv, shade(frame, 0.45, 1.0));
            draw_circle(rx - rv * 0.25, ry - rv * 0.25, rv * 0.5, Color::new(0.9, 0.95, 1.0, 0.9));
        }
        // Fondo del pozo con degradado (mas oscuro arriba = profundidad)
        volumen::grad_v(bx, by, bw, bh, Color::new(0.012, 0.016, 0.05, 0.96), Color::new(0.04, 0.055, 0.13, 0.96));
        for x in 1..W {
            let xx = bx + x as f32 * s;
            draw_line(xx, by, xx, by + bh, 1.0, Color::new(0.3, 0.5, 1.0, 0.07));
        }
        for y in 1..(H - HIDDEN) {
            let yy = by + y as f32 * s;
            draw_line(bx, yy, bx + bw, yy, 1.0, Color::new(0.3, 0.5, 1.0, 0.07));
        }
        // Sombra interior (el pozo esta hundido)
        for i in 0..4 {
            let k = i as f32 / 4.0;
            let d = s * 0.45;
            let a = 0.4 * (1.0 - k);
            draw_rectangle(bx, by + d * k, bw, d / 4.0, Color::new(0.0, 0.0, 0.0, a));
            draw_rectangle(bx + d * k, by, d / 4.0, bh, Color::new(0.0, 0.0, 0.0, a * 0.8));
        }
        // Modo lateral: marco dorado, rejilla de lado, luz y ecos "Y si...?"
        modo_lateral::pozo(bx, by, bw, bh, s, t, self.lat_k);
        if self.lat_k > 0.01 {
            let fr = (s * 0.3).max(4.0);
            draw_rectangle(bx - fr, by - fr, bw + 2.0 * fr, fr, alpha(GOLD, 0.5 * self.lat_k));
            draw_rectangle(bx - fr, by + bh, bw + 2.0 * fr, fr, alpha(GOLD, 0.35 * self.lat_k));
            draw_rectangle(bx - fr, by, fr, bh, alpha(GOLD, 0.42 * self.lat_k));
            draw_rectangle(bx + bw, by, fr, bh, alpha(GOLD, 0.42 * self.lat_k));
            self.draw_echoes(bx, by, s);
        }

        // Dimensiones cuanticas: tinte del pozo y bloques "fantasma" de la otra dimension.
        // No chocan con la pieza, pero cuentan para completar filas.
        if g.dims {
            let dc = DIMC[g.dim as usize % 4];
            draw_rectangle(bx, by, bw, bh, alpha(dc, 0.05 + 0.12 * self.dim_flash));
            draw_rectangle_lines(bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, 2.0, alpha(dc, 0.55 + 0.3 * pulse));
            let sh = (t * 3.0).sin() * 0.5 + 0.5;
            for y in HIDDEN..H {
                // Fila que ya se completa sumando las dos dimensiones menos N huecos
                for x in 0..W {
                    // Cada dimension holograma con su color (A azul, B rosa, C dorada, D verde)
                    for k in 0..g.nd.saturating_sub(1) {
                        let o = g.others[k][y as usize][x as usize];
                        if o.kind == 0 {
                            continue;
                        }
                        let oc = DIMC[(g.dim as usize + 1 + k) % g.nd];
                        let px = bx + x as f32 * s;
                        let py = by + (y - HIDDEN) as f32 * s;
                        let m = s * (0.14 + 0.07 * k as f32);
                        draw_rectangle(px + m, py + m, s - 2.0 * m, s - 2.0 * m, alpha(oc, 0.13 + 0.05 * sh));
                        draw_rectangle_lines(px + m, py + m, s - 2.0 * m, s - 2.0 * m, (s * 0.07).max(1.5), alpha(oc, 0.5 + 0.2 * sh));
                        // Trama diagonal: se lee como holograma, no como bloque solido
                        draw_line(px + m, py + s - m, px + s - m, py + m, 1.0, alpha(oc, 0.35));
                        if o.ent != 0 {
                            draw_circle(px + s / 2.0, py + s / 2.0, s * 0.1, alpha(WHITE, 0.6));
                        }
                    }
                }
            }
        }
        // Simetria: eje central y el reflejo de la pieza activa
        if g.sym {
            let axx = bx + bw / 2.0;
            let dash = s * 0.35;
            let mut yy = by;
            while yy < by + bh {
                draw_line(axx, yy, axx, (yy + dash).min(by + bh), 1.5, alpha(WHITE, 0.28));
                yy += dash * 2.0;
            }
            if let Some(a) = g.active {
                for &(cx2, cy2) in pieces::cells(a.spec.kind, a.rot).iter() {
                    for &x in a.xs().iter() {
                        let mx = W - 1 - (x + cx2);
                        let ry = a.y + cy2;
                        if ry >= HIDDEN {
                            draw_rectangle_lines(bx + mx as f32 * s + 3.0, by + (ry - HIDDEN) as f32 * s + 3.0, s - 6.0, s - 6.0, 1.5, alpha(MAGENTA, 0.5));
                        }
                    }
                }
            }
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
            let tun = g.active_tunnel_target();
            for &x in xs.iter() {
                let gy = g.drop_y(a.spec.kind, a.rot, x, a.y);
                let ga = if sup { 0.22 } else if tun.is_some() { 0.2 } else { 0.4 };
                for &(cx, cy) in cells.iter() {
                    let yy = gy + cy;
                    if yy >= HIDDEN {
                        let gx = bx + (x + cx) as f32 * s;
                        let gyp = by + (yy - HIDDEN) as f32 * s;
                        draw_rectangle(gx + 2.0, gyp + 2.0, s - 4.0, s - 4.0, alpha(col, ga * 0.25));
                        draw_rectangle_lines(gx + 3.0, gyp + 3.0, s - 6.0, s - 6.0, 2.0, alpha(col, ga));
                    }
                }
            }
            // Guia del TUNEL: donde reaparecera la pieza tras atravesar bloques
            if let Some((gy, ty)) = tun {
                let k = (t * 6.0).sin() * 0.5 + 0.5;
                for &(cx, cy) in cells.iter() {
                    let yy = ty + cy;
                    if yy >= HIDDEN {
                        let gx = bx + (a.x + cx) as f32 * s;
                        let gyp = by + (yy - HIDDEN) as f32 * s;
                        draw_rectangle(gx + 2.0, gyp + 2.0, s - 4.0, s - 4.0, alpha(TUNNEL_C, 0.18 + 0.12 * k));
                        draw_rectangle_lines(gx + 2.0, gyp + 2.0, s - 4.0, s - 4.0, 2.5, alpha(TUNNEL_C, 0.7 + 0.3 * k));
                    }
                }
                // Linea punteada que atraviesa los bloques
                let mut minx = 9;
                let mut maxx = -9;
                for &(cx, _) in cells.iter() {
                    minx = minx.min(cx);
                    maxx = maxx.max(cx);
                }
                let lx = bx + (a.x as f32 + (minx + maxx + 1) as f32 / 2.0) * s;
                let ly0 = by + ((gy + 2 - HIDDEN) as f32) * s;
                let ly1 = by + ((ty + 1 - HIDDEN) as f32) * s;
                let mut yy = ly0;
                let dash = s * 0.3;
                let off = (t * s * 3.0) % (dash * 2.0);
                yy += off;
                while yy < ly1 {
                    let e = (yy + dash).min(ly1);
                    draw_line(lx, yy, lx, e, 2.5, alpha(TUNNEL_C, 0.9));
                    yy += dash * 2.0;
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
                        if a.spec.tunnel {
                            tunnel_mark(px + s / 2.0, py + s / 2.0, s, t);
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
            // Etiqueta que explica la pieza especial (en juego normal)
            if self.puzzle.is_none() && !centers.is_empty() {
                let (label, lc) = if sup {
                    (if g.charges > 0 { "SUPERPOSICION: pulsa OBSERVAR" } else { "SUPERPOSICION: caera al azar" }, CYAN)
                } else if a.spec.entangled {
                    ("ENLAZADA: borra una y cae su gemela", VIOLET)
                } else if a.spec.tunnel {
                    ("TUNEL: atraviesa bloques", TUNNEL_C)
                } else {
                    ("", WHITE)
                };
                if !label.is_empty() {
                    let mid = centers.iter().fold(Vec2::ZERO, |acc, c| acc + *c) / centers.len() as f32;
                    let ls = fit_size(label, s * 0.5, s * 9.6);
                    let lw = measure_text(label, None, ls as u16, 1.0).width;
                    let ly = (mid.y + s * 2.2).max(by + s * 2.6);
                    let lx = (mid.x).clamp(bx + lw / 2.0 + 4.0, bx + s * W as f32 - lw / 2.0 - 4.0);
                    draw_rectangle(lx - lw / 2.0 - 5.0, ly - ls * 0.95, lw + 10.0, ls * 1.3, Color::new(0.0, 0.0, 0.05, 0.75));
                    draw_rectangle_lines(lx - lw / 2.0 - 5.0, ly - ls * 0.95, lw + 10.0, ls * 1.3, 1.5, alpha(lc, 0.8));
                    text_c(label, lx, ly, ls, lc);
                }
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
            // Etiqueta de pieza especial
            let tag = if spec.superpos {
                Some(("DOBLE", CYAN))
            } else if spec.entangled {
                Some(("ENLACE", VIOLET))
            } else if spec.tunnel {
                Some(("TUNEL", TUNNEL_C))
            } else {
                None
            };
            if let Some((tg, tc)) = tag {
                let ts = fit_size(tg, s * 0.42, sw * 0.6);
                let tw = measure_text(tg, None, ts as u16, 1.0).width;
                draw_rectangle(r.x + r.w - tw - 6.0, r.y + 2.0, tw + 4.0, ts * 1.05, alpha(tc, 0.85));
                draw_text(tg, r.x + r.w - tw - 4.0, r.y + ts * 0.85 + 2.0, ts, Color::new(0.02, 0.02, 0.08, 1.0));
            }
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
        let stats = if let Some(i) = self.puzzle {
            let left = g.remaining() + g.active.map(|_| 1).unwrap_or(0);
            let goal = match puzzles::get(i).goal {
                Goal::Lines(n) => format!("{}/{}", g.lines.min(n), n),
                Goal::ClearAll => {
                    let n = g.grid.iter().flatten().filter(|c| c.kind != 0).count();
                    format!("{}", n)
                }
            };
            let gl = match puzzles::get(i).goal {
                Goal::Lines(_) => "LINEAS",
                Goal::ClearAll => "BLOQUES",
            };
            [
                ("PIEZAS", left.to_string()),
                (gl, goal),
                ("USADAS", g.pieces_used().to_string()),
            ]
        } else {
            [
                ("PUNTOS", g.score.to_string()),
                ("NIVEL", g.level.to_string()),
                ("LINEAS", g.lines.to_string()),
            ]
        };
        for (name, val) in stats.iter() {
            // Cada dato en su propia vitrina
            panel(Rect::new(x, y, sw, lab * 0.95 + s * 0.95), CYAN);
            let mg = s * 0.14;
            draw_text(name, x + mg, y + lab * 0.85, lab * 0.85, alpha(WHITE, 0.65));
            let vs = fit_size(val, s * 0.82, sw - mg * 2.0);
            draw_text(val, x + mg, y + lab * 0.85 + s * 0.78, vs, WHITE);
            y += lab + s * 1.15;
        }
        if self.best > 0 && self.puzzle.is_none() {
            let bt = format!("REC {}", self.best.max(g.score));
            let bs = fit_size(&bt, lab * 0.9, sw);
            draw_text(&bt, x, y + bs, bs, alpha(GOLD, 0.8));
        }
    }

    fn draw_overlay_bg(&self) {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.05, 0.7));
    }

    /// Tarjeta de reglas al entrar a un mundo: deja claro como se juega en el.
    fn draw_intro(&self) {
        let (w, h) = (screen_width(), screen_height());
        self.draw_overlay_bg();
        let lateral = self.puzzle.is_some();
        let col = if lateral { GOLD } else { CYAN };
        let pw = (w * 0.94).min(h * 0.6);
        let px = (w - pw) / 2.0;
        // La tarjeta mide lo que ocupan sus 9 lineas, centrada en la pantalla
        let ls = (h * 0.86 / 31.0).min(w * 0.046);
        let ph = ls * 30.0;
        let py = (h - ph) / 2.0;
        panel(Rect::new(px, py, pw, ph), col);
        let cx = w / 2.0;
        let mut y = py + ls * 4.2;
        block_title(if lateral { "LATERAL" } else { "CUANTICO" }, cx, y, ls * 2.6, pw * 0.86, col);
        y += ls * 1.7;
        let t = "COMO SE JUEGA EN ESTE MUNDO";
        text_c(t, cx, y, fit_size(t, ls * 1.0, pw * 0.9), alpha(WHITE, 0.75));
        y += ls * 2.0;
        // (numero de regla o 0 para sub-linea, texto, color)
        let rules: Vec<(u32, &str, Color)> = if lateral {
            vec![
                (1, "SIN RELOJ: nada cae ni se fija solo. CAIDA fija.", WHITE),
                (2, "PIEZAS CONTADAS: solo las del panel PIEZAS.", WHITE),
                (3, "META: esta escrita arriba del tablero.", WHITE),
                (4, "LO OBVIO NO BASTA. Tus herramientas:", GOLD),
                (0, "GUARDAR: cambia el orden de las piezas", MAGENTA),
                (0, "GIRAR: la forma rara a veces encaja", VIOLET),
                (0, "DESLIZAR: entra de lado bajo un saliente", GREEN),
                (5, "MENOS PIEZAS = 3 ESTRELLAS.", WHITE),
                (6, "Cada nivel resuelto abre el siguiente.", WHITE),
            ]
        } else {
            vec![
                (1, "Las piezas caen. Llena filas para borrarlas.", WHITE),
                (2, "Cada nivel abre un concepto cuantico:", CYAN),
                (0, "N1 SUPERPOSICION: pieza en 2 lugares", CYAN),
                (0, "N2 EFECTO TUNEL: atraviesa bloques", TUNNEL_C),
                (0, "N3 ENTRELAZAMIENTO: gemelas a distancia", VIOLET),
                (0, "N4 DIMENSIONES: dos pozos a la vez", MAGENTA),
                (0, "N5 TERCERA DIMENSION: tres pozos", GOLD),
                (0, "N6 SIMETRIA: fila espejo se borra", VIOLET),
                (0, "N7 CUARTA DIMENSION: cuatro pozos", GREEN),
                (0, "N8 SUPERCUANTICO: todo junto y veloz", WHITE),
                (3, "Subes de nivel borrando lineas (barra de arriba).", WHITE),
                (4, "Cada concepto se explica al aparecer.", WHITE),
            ]
        };
        for (n, s, c) in rules.iter() {
            if *n > 0 {
                let r = ls * 0.62;
                let bx = px + pw * 0.075;
                draw_circle(bx, y - ls * 0.3, r, alpha(col, 0.95));
                draw_circle(bx, y - ls * 0.3, r * 0.78, Color::new(0.03, 0.04, 0.1, 1.0));
                text_c(&n.to_string(), bx, y, ls * 0.95, col);
                draw_text(s, px + pw * 0.14, y, fit_size(s, ls * 0.95, pw * 0.83), *c);
            } else {
                draw_rectangle(px + pw * 0.15, y - ls * 0.55, ls * 0.35, ls * 0.35, *c);
                draw_text(s, px + pw * 0.2, y, fit_size(s, ls * 0.88, pw * 0.76), *c);
            }
            y += ls * 1.62;
        }
        let p = (self.time * 3.0).sin() * 0.5 + 0.5;
        let t = "TOCA PARA JUGAR";
        text_c(t, cx, py + ph - ls * 1.2, fit_size(t, ls * 1.3, pw * 0.8), alpha(GREEN, 0.55 + 0.45 * p));
    }

    /// "Que es esto?": explica una pieza especial con un dibujo (el juego espera).
    fn draw_explain(&self, k: u8) {
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.05, 0.6));
        let pw = (w * 0.92).min(h * 0.6);
        let px = (w - pw) / 2.0;
        let ph = h * 0.62;
        let py = h * 0.16;
        let (title, color, lines): (&str, Color, [&str; 6]) = match k {
            0 => (
                "SUPERPOSICION",
                CYAN,
                [
                    "La ONDA une dos copias de la misma pieza:",
                    "esta en DOS lugares a la vez (50% / 50%).",
                    "Si no haces nada, al tocar fondo se queda",
                    "AL AZAR en una de las dos.",
                    "COMO USARLA: pulsa OBSERVAR (gasta 1 carga).",
                    "Se queda la copia mas honda y la mueves libre.",
                ],
            ),
            1 => (
                "ENTRELAZADA",
                VIOLET,
                [
                    "El PUNTO blanco marca una pieza enlazada.",
                    "La ONDA violeta une a las dos gemelas.",
                    "Si borras una linea que toca a una,",
                    "la otra DESAPARECE tambien, este donde este.",
                    "COMO USARLA: no hay boton. Pon las gemelas",
                    "donde te convenga borrar bloques lejos.",
                ],
            ),
            4 => (
                "SIMETRIA",
                MAGENTA,
                [
                    "Una linea vertical parte el pozo en dos mitades.",
                    "Tu pieza tiene un REFLEJO rosa al otro lado.",
                    "Si una fila es SIMETRICA (la mitad izquierda",
                    "igual a la derecha) y tiene 6+ bloques, se borra",
                    "aunque no este llena.",
                    "COMO USARLA: arma filas espejo, no solo llenas.",
                ],
            ),
            3 => (
                "DIMENSIONES",
                MAGENTA,
                [
                    "El pozo existe en DOS dimensiones: A y B.",
                    "Juegas en una. La otra se ve como HOLOGRAMA.",
                    "Los hologramas NO chocan con tu pieza, pero",
                    "una fila se borra si entre las DOS la llenan.",
                    "COMO USARLO: pulsa DIMENSION para saltar.",
                    "Un hueco tapado aqui, esta abierto alla.",
                ],
            ),
            _ => (
                "TUNEL",
                TUNNEL_C,
                [
                    "La FLECHA con anillos marca una pieza tunel.",
                    "Al tocar fondo ATRAVIESA hasta 4 filas",
                    "de bloques y cae en el hueco de abajo.",
                    "La guia verde punteada muestra donde caera.",
                    "COMO USARLA: se activa sola. Ponla encima",
                    "de un hueco tapado y sueltala.",
                ],
            ),
        };
        panel(Rect::new(px, py, pw, ph), color);
        let ls = (ph / 17.0).min(w * 0.046);
        let cx = w / 2.0;
        let mut y = py + ls * 1.8;
        let t = "QUE ES ESTO?";
        text_c(t, cx, y, fit_size(t, ls * 1.0, pw * 0.9), alpha(WHITE, 0.7));
        y += ls * 1.9;
        text_3d_c(title, cx, y, fit_size(title, ls * 1.9, pw * 0.9), color);
        y += ls * 1.0;
        // Dibujo de ejemplo
        let s = ls * 1.3;
        let t0 = self.time;
        let ty = y + s * 0.4;
        let tcol = pieces::color(2);
        let tcells = pieces::cells(2, 0);
        match k {
            0 => {
                let mut cs = Vec::new();
                for (i, off) in [-s * 2.6, s * 1.0].iter().enumerate() {
                    let al = 0.55 + 0.25 * (t0 * 10.0 + i as f32 * PI).sin();
                    let mut sum = Vec2::ZERO;
                    for &(x, yy) in tcells.iter() {
                        let p = vec2(cx + off + x as f32 * s, ty + yy as f32 * s);
                        draw_block(p.x, p.y, s, tcol, al);
                        sum += p + vec2(s / 2.0, s / 2.0);
                    }
                    cs.push(sum / 4.0);
                    text_c("50%", cs[i].x, ty - s * 0.2, s * 0.6, WHITE);
                }
                wavy_line(cs[0], cs[1], s * 0.5, t0 * 1.5, 2.5, alpha(tcol, 0.8));
            }
            1 => {
                let a = vec2(cx - s * 2.5, ty + s * 0.5);
                let b = vec2(cx + s * 2.5, ty + s * 1.2);
                for p in [a, b].iter() {
                    draw_block(p.x - s / 2.0, p.y - s / 2.0, s, pieces::color(5), 1.0);
                    ent_mark(p.x, p.y, s, t0);
                }
                wavy_line(a, b, s * 0.35, t0, 2.5, Color::new(0.85, 0.6, 1.0, 0.8));
            }
            4 => {
                // Fila simetrica de 6 bloques respecto al eje central
                let x0 = cx - s * 3.5;
                let ry = ty + s * 0.9;
                for x in 0..7 {
                    let px = x0 + x as f32 * s;
                    if x == 3 {
                        draw_rectangle_lines(px + 3.0, ry + 3.0, s - 6.0, s - 6.0, 1.5, alpha(WHITE, 0.25));
                    } else if x >= 1 && x <= 5 || x == 0 || x == 6 {
                        draw_block(px, ry, s * 0.96, pieces::color(if x < 3 { 2 } else { 5 }), 1.0);
                    }
                }
                let ax = x0 + s * 3.5;
                draw_line(ax, ry - s * 0.5, ax, ry + s * 1.4, 2.0, alpha(WHITE, 0.6));
                text_c("EJE", ax, ry - s * 0.7, s * 0.6, WHITE);
            }
            3 => {
                // Una fila con hueco en A; el holograma de B la completa
                let x0 = cx - s * 3.0;
                let ry = ty + s * 0.9;
                for x in 0..6 {
                    let px = x0 + x as f32 * s;
                    if x == 2 || x == 3 {
                        let m = s * 0.14;
                        let k = (t0 * 3.0).sin() * 0.5 + 0.5;
                        draw_rectangle(px + m, ry + m, s - 2.0 * m, s - 2.0 * m, alpha(MAGENTA, 0.15 + 0.1 * k));
                        draw_rectangle_lines(px + m, ry + m, s - 2.0 * m, s - 2.0 * m, 2.0, alpha(MAGENTA, 0.6 + 0.3 * k));
                        draw_line(px + m, ry + s - m, px + s - m, ry + m, 1.0, alpha(MAGENTA, 0.5));
                    } else {
                        draw_block(px, ry, s * 0.96, pieces::color(0), 1.0);
                    }
                }
                text_c("A", x0 - s * 0.6, ry + s * 0.75, s * 0.8, CYAN);
                text_c("B", x0 + s * 6.6, ry + s * 0.75, s * 0.8, MAGENTA);
                text_c("A + B = FILA COMPLETA", cx, ry - s * 0.35, s * 0.6, WHITE);
            }
            _ => {
                let bx = cx - s * 1.5;
                for &(x, yy) in tcells.iter() {
                    let p = vec2(bx + x as f32 * s, ty + yy as f32 * s - s * 0.4);
                    draw_block(p.x, p.y, s, tcol, 1.0);
                    tunnel_mark(p.x + s / 2.0, p.y + s / 2.0, s, t0);
                }
                for x in 0..5 {
                    draw_block(cx - s * 2.5 + x as f32 * s, ty + s * 1.8, s * 0.95, pieces::color(7), 0.9);
                }
            }
        }
        y = ty + s * 3.4;
        for (i, line) in lines.iter().enumerate() {
            let c = if i >= 4 { GREEN } else { WHITE };
            text_c(line, cx, y, fit_size(line, ls * 0.95, pw * 0.94), c);
            y += ls * 1.35;
            if i == 3 {
                y += ls * 0.4;
            }
        }
        let p = (self.time * 3.0).sin() * 0.5 + 0.5;
        let t = "TOCA PARA SEGUIR";
        text_c(t, cx, py + ph - ls * 0.9, fit_size(t, ls * 1.1, pw * 0.8), alpha(GREEN, 0.55 + 0.45 * p));
    }

    /// Anuncio grande al empezar un reto (el juego espera un toque).
    fn draw_announce(&self) {
        let c = match &self.challenge {
            Some(c) => c,
            None => return,
        };
        let (w, h) = (screen_width(), screen_height());
        let cx = w / 2.0;
        let lv = lateral::level(self.mode);
        // El mundo lateral ocupa toda la pantalla
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.09, 0.03, 0.1, 0.9));
        let cell = (w * 0.88 / 41.0).min(h * 0.017);
        let ls = (w * 0.05).min(h * 0.03);
        let est = cell * 15.5 + ls * 15.5;
        let mut y = ((h - est) / 2.0 - h * 0.02).max(h * 0.04);
        // "MODO" / "LATERAL": cada letra entra de lado, con desfase
        self.letters_in("MODO", y, cell, c.t, 0.0);
        y += cell * 8.5;
        self.letters_in("LATERAL", y, cell, c.t, 0.2);
        y += cell * 7.0;
        let cy = y - cell * 3.5;
        let chev = (cell * 1.1).max(4.0);
        modo_lateral::chevrones(cx - cell * 22.0, cy, chev, -1.0, 2, self.time, alpha(GOLD, 0.8));
        modo_lateral::chevrones(cx + cell * 22.0, cy, chev, 1.0, 2, self.time, alpha(GOLD, 0.8));
        // De aqui en adelante aparece en cuanto llegan las letras
        let f = ((c.t - 0.55) * 4.0).max(0.0).min(1.0);
        y += ls * 1.7;
        let t = c.tech.name();
        text_c(t, cx, y, fit_size(t, ls * 1.2, w * 0.9), alpha(TUNNEL_C, f));
        y += ls * 2.1;
        let (q1, q2) = modo_lateral::dos_lineas(c.tech.question());
        let qs = fit_size(&q1, ls * 1.7, w * 0.92).min(fit_size(&q2, ls * 1.7, w * 0.92));
        text_c(&q1, cx, y, qs, alpha(WHITE, f));
        y += qs * 1.15;
        text_c(&q2, cx, y, qs, alpha(WHITE, f));
        y += ls * 1.8;
        let (g1, g2) = modo_lateral::dos_lineas(c.tech.goal());
        let gs = fit_size(&g1, ls * 0.95, w * 0.92).min(fit_size(&g2, ls * 0.95, w * 0.92));
        text_c(&g1, cx, y, gs, alpha(GOLD, f));
        y += gs * 1.25;
        text_c(&g2, cx, y, gs, alpha(GOLD, f));
        y += ls * 1.5;
        // Fichas: las piezas que tienes para lograrlo
        let n = c.total.min(12);
        let r = (ls * 0.3).max(3.0);
        let gap = r * 2.7;
        let x0 = cx - (n as f32 - 1.0) * gap / 2.0;
        for i in 0..n {
            draw_circle(x0 + i as f32 * gap, y, r, alpha(GOLD, f));
            draw_circle(x0 + i as f32 * gap - r * 0.25, y - r * 0.25, r * 0.35, alpha(WHITE, 0.7 * f));
        }
        y += ls * 1.3;
        let t = format!("{} PIEZAS  -  TIEMPO LENTO x{:.2}", c.total, lv.slow);
        text_c(&t, cx, y, fit_size(&t, ls * 0.85, w * 0.9), alpha(WHITE, 0.85 * f));
        y += ls * 1.6;
        if lv.hint == 0 {
            let (h1, h2) = modo_lateral::dos_lineas(c.tech.hint());
            let hs = fit_size(&h1, ls * 0.8, w * 0.92).min(fit_size(&h2, ls * 0.8, w * 0.92));
            text_c(&h1, cx, y, hs, alpha(GREEN, f));
            text_c(&h2, cx, y + hs * 1.25, hs, alpha(GREEN, f));
        } else if lv.hint == 1 {
            let t = "La pista aparecera a mitad del reto";
            text_c(t, cx, y, fit_size(t, ls * 0.8, w * 0.9), alpha(WHITE, 0.6 * f));
        } else {
            let t = "EXPERTO: sin pistas. Piensa de lado.";
            text_c(t, cx, y, fit_size(t, ls * 0.8, w * 0.9), alpha(MAGENTA, 0.9 * f));
        }
        // Tiempo restante y aviso para seguir
        let p = (self.time * 3.0).sin() * 0.5 + 0.5;
        let t = "TOCA PARA EMPEZAR";
        text_c(t, cx, h * 0.955, fit_size(t, ls * 1.05, w * 0.8), alpha(GREEN, 0.55 + 0.45 * p));
        let left = (1.0 - c.t / lateral::ANNOUNCE).max(0.0);
        draw_rectangle(cx - w * 0.4 * left, h * 0.975, w * 0.8 * left, (h * 0.005).max(2.0), alpha(GOLD, 0.8));
    }

    /// Letras de bloques que llegan de lado, alternando el costado.
    fn letters_in(&self, t: &str, y: f32, cell: f32, time: f32, delay: f32) {
        let w = screen_width();
        let n = t.chars().count();
        let x0 = w / 2.0 - volumen::block_text_width(t, cell) / 2.0;
        for (i, ch) in t.chars().enumerate() {
            let q = ((time - delay - i as f32 * 0.04) / 0.32).max(0.0).min(1.0);
            let e = 1.0 - (1.0 - q) * (1.0 - q) * (1.0 - q);
            let lado = if i % 2 == 0 { -1.0 } else { 1.0 };
            let cx = x0 + (i as f32 * 6.0 + 2.5) * cell + (1.0 - e) * lado * w;
            let c = mix(GOLD, modo_lateral::CORAL, i as f32 / n.max(2) as f32);
            volumen::block_text_c(&ch.to_string(), cx, y, cell, c);
        }
    }

    /// Exito de un reto: el juego se congela y celebra.
    fn draw_eureka(&self, e: &Eureka) {
        let (w, h) = (screen_width(), screen_height());
        let cx = w / 2.0;
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.09, 0.03, 0.1, 0.78));
        let cell = (w * 0.9 / 47.0).min(h * 0.016);
        let ls = (w * 0.05).min(h * 0.03);
        let mut y = h * 0.28;
        self.letters_in("PENSASTE", y, cell, e.t * 1.4, 0.0);
        y += cell * 8.5;
        self.letters_in("DE LADO", y, cell, e.t * 1.4, 0.1);
        y += cell * 7.0 + ls * 1.8;
        let t = format!("{} +{}", e.tech.name(), e.bonus);
        text_c(&t, cx, y, fit_size(&t, ls * 1.2, w * 0.92), GREEN);
        y += ls * 1.9;
        let (a, b) = modo_lateral::dos_lineas(e.tech.lesson());
        let s = fit_size(&a, ls, w * 0.92).min(fit_size(&b, ls, w * 0.92));
        text_c(&a, cx, y, s, WHITE);
        text_c(&b, cx, y + s * 1.3, s, WHITE);
        y += s * 1.3 + ls * 1.6;
        let t = "+1 carga de OBSERVAR";
        text_c(t, cx, y, fit_size(t, ls * 0.85, w * 0.9), alpha(GOLD, 0.9));
    }

    /// Ecos "Y si...?": donde caeria la pieza un par de columnas a cada lado.
    fn draw_echoes(&self, bx: f32, by: f32, s: f32) {
        let g = &self.game;
        let a = match g.active {
            Some(a) if a.dx.is_none() => a,
            _ => return,
        };
        let k = self.lat_k;
        for &d in [-2, 2].iter() {
            let x = a.x + d;
            if !g.fits(a.spec.kind, a.rot, x, a.y) {
                continue;
            }
            let gy = g.drop_y(a.spec.kind, a.rot, x, a.y);
            let mut sum = Vec2::ZERO;
            for &(cx, cy) in pieces::cells(a.spec.kind, a.rot).iter() {
                let yy = gy + cy;
                let px = bx + (x + cx) as f32 * s;
                let py = by + (yy - HIDDEN) as f32 * s;
                sum += vec2(px + s / 2.0, py + s / 2.0);
                if yy >= HIDDEN {
                    draw_rectangle_lines(px + 4.0, py + 4.0, s - 8.0, s - 8.0, 1.0, alpha(GOLD, 0.26 * k));
                }
            }
            let c = sum / 4.0;
            let p = (self.time * 2.5 + d as f32).sin() * 0.5 + 0.5;
            text_c("?", c.x, c.y + s * 0.25, s * 0.8, alpha(GOLD, (0.14 + 0.16 * p) * k));
        }
    }

    /// Tinte calido de la cabecera segun el mundo.
    fn header_tint(&self, l: &Lay) {
        let w = screen_width();
        let k = self.lat_k;
        draw_rectangle(0.0, 0.0, w, l.top, Color::new(0.38, 0.16, 0.05, 0.75 * k));
        draw_line(0.0, l.top, w, l.top, 2.0, alpha(GOLD, 0.9 * k));
    }

    /// Cabecera: deja siempre claro en que modo de pensamiento estas.
    fn draw_mode_header(&self, l: &Lay) {
        let w = screen_width();
        let t = self.time;
        self.header_tint(l);
        let cx = w / 2.0;
        let left = w * 0.03;
        let ms = fit_size(self.game.mode.name(), l.top * 0.24, w * 0.19);
        if self.lat_on {
            draw_text(self.game.mode.name(), left, l.top * 0.38, ms, alpha(WHITE, 0.85));
            let title = "MODO LATERAL";
            let ts = fit_size(title, l.top * 0.42, w * 0.44);
            text_3d_c(title, cx, l.top * 0.52, ts, GOLD);
            let half = measure_text(title, None, ts.max(1.0) as u16, 1.0).width / 2.0;
            let chev = l.top * 0.09;
            modo_lateral::chevrones(cx - half - chev, l.top * 0.38, chev, -1.0, 3, t, alpha(GOLD, 0.95));
            modo_lateral::chevrones(cx + half + chev, l.top * 0.38, chev, 1.0, 3, t, alpha(GOLD, 0.95));
            if let Some(c) = &self.challenge {
                // Fichas: piezas que quedan
                let n = c.left.min(12);
                let r = (w * 0.19 / (n.max(1) as f32 * 2.7)).min(l.top * 0.08).max(1.5);
                for i in 0..n {
                    draw_circle(left + r + i as f32 * r * 2.6, l.top * 0.68, r, GOLD);
                }
                let sub = format!("{}  -  TIEMPO LENTO x{:.2}", c.tech.short(), lateral::level(self.mode).slow);
                let ss = fit_size(&sub, l.top * 0.2, w * 0.5);
                let p = (t * 2.0).sin() * 0.5 + 0.5;
                text_c(&sub, cx, l.top * 0.88, ss, alpha(WHITE, 0.7 + 0.3 * p));
            }
        } else {
            draw_text(self.game.mode.name(), left, l.top * 0.62, ms, alpha(MAGENTA, 0.85));
            let title = "MODO VERTICAL";
            let ts = fit_size(title, l.top * 0.34, w * 0.44);
            text_3d_c(title, cx, l.top * 0.38, ts, CYAN);
            // Medidor: se llena con cada pieza hasta saltar al modo lateral
            let gw = w * 0.44;
            let gx = cx - gw / 2.0;
            let gy = l.top * 0.5;
            let gh = (l.top * 0.11).max(3.0);
            let total = self.next_total.max(1);
            let k = 1.0 - self.next_ch.min(total) as f32 / total as f32;
            draw_rectangle(gx, gy, gw, gh, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_rectangle(gx, gy, gw * k, gh, mix(CYAN, GOLD, k));
            for i in 1..total {
                let xx = gx + gw * i as f32 / total as f32;
                draw_line(xx, gy, xx, gy + gh, 1.0, Color::new(0.0, 0.0, 0.05, 0.9));
            }
            draw_rectangle_lines(gx, gy, gw, gh, 1.0, alpha(WHITE, 0.4));
            let sub = if self.next_ch == 1 {
                "LATERAL EN 1 PIEZA".to_string()
            } else {
                format!("LATERAL EN {} PIEZAS", self.next_ch)
            };
            let ss = fit_size(&sub, l.top * 0.21, gw);
            text_c(&sub, cx, l.top * 0.9, ss, alpha(GOLD, 0.95));
        }
    }

    /// Lineas de aviso: sobre el tablero si hay sitio; si no, en una banda dentro de el.
    fn draw_notice(&self, l: &Lay, lines: &[(&str, Color)]) {
        let w = screen_width();
        let fr = (l.cell * 0.3).max(4.0);
        let n = lines.len() as f32;
        let room = l.by - fr - l.top - l.top * 0.12;
        let gs = (room / (n * 1.2)).min(l.cell * 0.62);
        if gs >= l.cell * 0.36 {
            let mut y = l.by - fr - 6.0 - (n - 1.0) * gs * 1.2;
            for &(t, c) in lines.iter() {
                let s = fit_size(t, gs, w * 0.95);
                text_c(t, w / 2.0 + 1.0, y + 1.0, s, Color::new(0.0, 0.0, 0.0, 0.75 * c.a));
                text_c(t, w / 2.0, y, s, c);
                y += gs * 1.2;
            }
            return;
        }
        // Pantalla baja: banda en lo alto del pozo; se aparta si la pieza pasa debajo
        let bw = l.cell * W as f32;
        let sz = l.cell * 0.5;
        let bh = n * sz * 1.25 + sz * 0.3;
        let near = self.game.active.map(|a| a.y < HIDDEN + 3).unwrap_or(false);
        let fade = if near { 0.3 } else { 1.0 };
        draw_rectangle(l.bx, l.by, bw, bh, Color::new(0.1, 0.04, 0.1, 0.72 * fade));
        let mut y = l.by + sz * 1.05;
        for &(t, c) in lines.iter() {
            let s = fit_size(t, sz, bw * 0.96);
            text_c(t, l.bx + bw / 2.0, y, s, alpha(c, fade));
            y += sz * 1.25;
        }
    }

    /// Botonera de control con iconos dibujados (la fuente no trae flechas).
    fn draw_pad(&self, pad: &[Rect; 6]) {
        let cols = [CYAN, CYAN, VIOLET, VIOLET, GREEN, MAGENTA];
        for (i, r) in pad.iter().enumerate() {
            let held = self.pad_held.iter().any(|h| h.1 == i);
            let c = cols[i];
            // Al pulsar, el boton "se hunde": baja hasta su canto
            let depth = volumen::key_depth(r.h);
            let rr = if held { Rect::new(r.x, r.y + depth * 0.7, r.w, r.h) } else { *r };
            button_d(rr, "", c, true, if held { 1.0 } else { 0.0 }, if held { depth * 0.3 } else { depth });
            let cx = rr.x + rr.w / 2.0;
            let cy = rr.y + rr.h / 2.0;
            let u = rr.w.min(rr.h) * 0.22;
            let ic = Color::new(1.0, 1.0, 1.0, 0.95);
            let sh = Color::new(0.0, 0.0, 0.0, 0.5);
            let th = (u * 0.35).max(2.0);
            for &(o, col) in [(1.5f32, sh), (0.0, ic)].iter() {
                let (x, y) = (cx + o, cy + o);
                match i {
                    0 => draw_triangle(vec2(x - u * 1.2, y), vec2(x + u * 0.8, y - u * 1.1), vec2(x + u * 0.8, y + u * 1.1), col),
                    1 => draw_triangle(vec2(x + u * 1.2, y), vec2(x - u * 0.8, y - u * 1.1), vec2(x - u * 0.8, y + u * 1.1), col),
                    2 | 3 => {
                        // Flecha circular: arco de 270 grados + punta
                        let rad = u * 0.95;
                        let n = 14;
                        let dir = if i == 3 { 1.0 } else { -1.0 };
                        let a0 = -PI * 0.5;
                        let mut prev = vec2(x + rad * a0.cos(), y + rad * a0.sin());
                        for k in 1..=n {
                            let a = a0 + dir * (k as f32 / n as f32) * PI * 1.5;
                            let p = vec2(x + rad * a.cos(), y + rad * a.sin());
                            draw_line(prev.x, prev.y, p.x, p.y, th, col);
                            prev = p;
                        }
                        // punta de flecha al final del arco
                        let a = a0 + dir * PI * 1.5;
                        let tip = vec2(x + rad * a.cos(), y + rad * a.sin());
                        let tang = vec2(-a.sin(), a.cos()) * dir;
                        let norm = vec2(a.cos(), a.sin());
                        draw_triangle(
                            tip + tang * u * 0.7,
                            tip - norm * u * 0.55,
                            tip + norm * u * 0.55,
                            col,
                        );
                    }
                    4 => draw_triangle(vec2(x, y + u * 1.1), vec2(x - u * 1.1, y - u * 0.7), vec2(x + u * 1.1, y - u * 0.7), col),
                    _ => {
                        draw_triangle(vec2(x, y + u * 0.2), vec2(x - u * 1.0, y - u * 1.0), vec2(x + u * 1.0, y - u * 1.0), col);
                        draw_triangle(vec2(x, y + u * 1.0), vec2(x - u * 1.0, y - u * 0.2), vec2(x + u * 1.0, y - u * 0.2), col);
                        draw_rectangle(x - u * 1.1, y + u * 1.1, u * 2.2, th, col);
                    }
                }
            }
        }
    }

    /// Mision del reto (o leccion al terminar) junto al tablero.
    fn draw_lateral_banner(&self, l: &Lay) {
        if let Some(c) = &self.challenge {
            if c.announcing {
                return;
            }
            let mut lines = vec![(c.tech.question(), WHITE), (c.tech.goal(), GOLD)];
            if self.hint_visible() {
                lines.push((c.tech.hint(), GREEN));
            }
            self.draw_notice(l, &lines);
        } else if let Some((tech, ok, t)) = self.last_result {
            if self.eureka.is_some() {
                return;
            }
            let a = (1.0 - (t - 5.5).max(0.0) / 1.5).max(0.0);
            let t1 = if ok {
                format!("SUPERADO: {}", tech.name())
            } else {
                format!("NO SUPERADO: {}", tech.name())
            };
            let c1 = alpha(if ok { GREEN } else { MAGENTA }, a);
            self.draw_notice(l, &[(t1.as_str(), c1), (tech.lesson(), alpha(WHITE, 0.9 * a))]);
        }
    }

    /// Resumen del entrenamiento lateral (pantalla de fin de partida).
    fn draw_training(&self, y0: f32) {
        let (w, h) = (screen_width(), screen_height());
        let pw = (w * 0.9).min(h * 0.55);
        let px = (w - pw) / 2.0;
        let ph = (h - y0 - h * 0.02).max(h * 0.2);
        panel(Rect::new(px, y0, pw, ph), GOLD);
        let ls = (ph / 11.8).min(w * 0.045);
        let mut y = y0 + ls * 1.4;
        let t = "ENTRENAMIENTO LATERAL";
        text_c(t, w / 2.0, y, fit_size(t, ls * 1.1, pw * 0.9), GOLD);
        y += ls * 1.5;
        let (ok, tried) = self.game_stats.total();
        let s = format!("Retos superados: {} de {}   Indice: {}", ok, tried, self.game_stats.index());
        text_c(&s, w / 2.0, y, fit_size(&s, ls * 0.9, pw * 0.92), WHITE);
        y += ls * 0.6;
        for tech in lateral::ALL.iter() {
            y += ls * 1.25;
            let e = self.game_stats.s[tech.idx()];
            draw_text(tech.short(), px + pw * 0.05, y, ls * 0.85, alpha(WHITE, 0.8));
            let bx = px + pw * 0.42;
            let bw = pw * 0.4;
            draw_rectangle(bx, y - ls * 0.65, bw, ls * 0.6, Color::new(1.0, 1.0, 1.0, 0.08));
            if e.tried > 0 {
                let k = e.ok as f32 / e.tried as f32;
                let col = if k >= 0.99 { GREEN } else if k > 0.0 { GOLD } else { MAGENTA };
                draw_rectangle(bx, y - ls * 0.65, bw * k.max(0.04), ls * 0.6, col);
            }
            let v = format!("{}/{}", e.ok, e.tried);
            draw_text(&v, bx + bw + pw * 0.03, y, ls * 0.85, alpha(WHITE, 0.8));
        }
        y += ls * 1.5;
        if tried > 0 {
            let weak = self.game_stats.weakest_tried();
            let p = format!("Practica: {}", weak.name());
            text_c(&p, w / 2.0, y, fit_size(&p, ls * 0.9, pw * 0.92), TUNNEL_C);
            y += ls * 1.2;
            text_c(weak.lesson(), w / 2.0, y, fit_size(weak.lesson(), ls * 0.8, pw * 0.94), alpha(WHITE, 0.75));
        } else {
            let p = "Juega mas piezas: cada 5-10 aparece un reto";
            text_c(p, w / 2.0, y, fit_size(p, ls * 0.85, pw * 0.92), alpha(WHITE, 0.7));
        }
    }

    /// Cabecera del MUNDO CUANTICO: nivel, concepto, barra hacia el siguiente y dimension.
    fn draw_quantum_header(&self, l: &Lay) {
        let w = screen_width();
        let g = &self.game;
        let t = self.time;
        let cx = w / 2.0;
        let dc = if g.dims { DIMC[g.dim as usize % 4] } else { CYAN };
        if g.dims {
            draw_rectangle(0.0, 0.0, w, l.top, alpha(dc, 0.10 + 0.25 * self.dim_flash));
            draw_line(0.0, l.top, w, l.top, 2.0, alpha(dc, 0.9));
        }
        // Izquierda: numero de nivel en una placa
        let ps = l.top * 0.74;
        let pr = Rect::new(w * 0.025, (l.top - ps) / 2.0, ps * 1.25, ps);
        panel(pr, dc);
        let lab = fit_size("NIVEL", ps * 0.26, pr.w * 0.86);
        text_c("NIVEL", pr.x + pr.w / 2.0, pr.y + ps * 0.3, lab, alpha(WHITE, 0.75));
        let num = g.level.to_string();
        let ns = fit_size(&num, ps * 0.62, pr.w * 0.8);
        text_c(&num, pr.x + pr.w / 2.0, pr.y + ps * 0.88, ns, dc);
        // Centro: concepto del nivel y barra de progreso
        let name = game::qlevel_name(g.level);
        let ts = fit_size(name, l.top * 0.36, w * 0.5);
        text_3d_c(name, cx, l.top * 0.4, ts, dc);
        let gw = w * 0.5;
        let gx = cx - gw / 2.0;
        let gy = l.top * 0.52;
        let gh = (l.top * 0.12).max(3.0);
        let (have, need) = g.q_progress();
        draw_rectangle(gx, gy, gw, gh, Color::new(0.0, 0.0, 0.0, 0.55));
        for i in 0..need {
            let x0 = gx + gw * i as f32 / need as f32;
            let sw = gw / need as f32 - 2.0;
            if i < have {
                let k = (t * 3.0 + i as f32 * 0.5).sin() * 0.5 + 0.5;
                draw_rectangle(x0 + 1.0, gy + 1.0, sw, gh - 2.0, mix(dc, WHITE, 0.15 + 0.2 * k));
            } else {
                draw_rectangle_lines(x0 + 1.0, gy + 1.0, sw, gh - 2.0, 1.0, alpha(WHITE, 0.18));
            }
        }
        draw_rectangle_lines(gx, gy, gw, gh, 1.0, alpha(WHITE, 0.45));
        let left = need - have;
        let next = game::qlevel_name(g.level + 1);
        let sub = if g.level >= 8 {
            format!("{} LINEAS PARA EL NIVEL {}", left, g.level + 1)
        } else if left == 1 {
            format!("1 LINEA PARA {}", next)
        } else {
            format!("{} LINEAS PARA {}", left, next)
        };
        let ss = fit_size(&sub, l.top * 0.2, gw * 1.15);
        text_c(&sub, cx, l.top * 0.9, ss, alpha(GOLD, 0.95));
    }

    /// Una linea bajo la cabecera: la regla del concepto que acabas de abrir.
    fn draw_quantum_banner(&self, l: &Lay) {
        let g = &self.game;
        if g.over || self.intro {
            return;
        }
        if self.bot && self.jev_real {
            let s = format!("JEV REAL (TypeSafe): {} jugadas elegidas por el - tu solo miras", self.jev_ok);
            self.draw_notice(l, &[(s.as_str(), VIOLET)]);
            return;
        }
        let (txt, c) = if self.bot {
            ("JEV (IA local) juega: tu solo miras. PAUSA para salir", VIOLET)
        } else if g.level == 6 {
            ("SIMETRIA: una fila simetrica con 6+ bloques se borra", MAGENTA)
        } else if g.dims {
            let t: &'static str = match g.nd {
                2 => "DIMENSION A/B - el holograma es la otra dimension",
                3 => "3 DIMENSIONES - cada holograma tiene su color",
                _ => "4 DIMENSIONES - A azul, B rosa, C dorada, D verde",
            };
            (t, DIMC[g.dim as usize % 4])
        } else {
            match g.level {
                0 | 1 => ("SUPERPOSICION: OBSERVAR elige la copia mas honda", CYAN),
                2 => ("TUNEL: la pieza con flecha atraviesa bloques", TUNNEL_C),
                _ => ("ENTRELAZADAS: borra una y desaparece su gemela", VIOLET),
            }
        };
        self.draw_notice(l, &[(txt, alpha(c, 0.95))]);
    }

    /// Fin de partida cuantica: hasta donde llegaste y que venia despues.
    fn draw_qsummary(&self, y0: f32) {
        let (w, h) = (screen_width(), screen_height());
        let pw = (w * 0.9).min(h * 0.55);
        let px = (w - pw) / 2.0;
        let ph = (h - y0 - h * 0.02).max(h * 0.2);
        panel(Rect::new(px, y0, pw, ph), CYAN);
        let ls = (ph / 14.0).min(w * 0.045);
        let lv = self.game.level;
        let mut y = y0 + ls * 1.5;
        let t = "TU VIAJE CUANTICO";
        text_c(t, w / 2.0, y, fit_size(t, ls * 1.1, pw * 0.9), CYAN);
        y += ls * 0.7;
        let rows = [
            (1, "SUPERPOSICION", CYAN),
            (2, "EFECTO TUNEL", TUNNEL_C),
            (3, "ENTRELAZAMIENTO", VIOLET),
            (4, "DIMENSIONES", MAGENTA),
            (5, "TERCERA DIMENSION", GOLD),
            (6, "SIMETRIA", VIOLET),
            (7, "CUARTA DIMENSION", GREEN),
            (8, "SUPERCUANTICO", WHITE),
        ];
        for (n, name, c) in rows.iter() {
            y += ls * 1.35;
            let on = lv >= *n;
            let bx = px + pw * 0.1;
            draw_block(bx - ls * 0.5, y - ls * 0.85, ls, if on { *c } else { Color::new(0.2, 0.22, 0.3, 1.0) }, if on { 1.0 } else { 0.6 });
            let s = format!("NIVEL {}  {}", n, name);
            draw_text(&s, px + pw * 0.2, y, fit_size(&s, ls * 0.9, pw * 0.62), if on { *c } else { alpha(WHITE, 0.35) });
            if on {
                // Marca de logrado
                let (mx, my) = (px + pw * 0.88, y - ls * 0.3);
                draw_line(mx - ls * 0.3, my, mx - ls * 0.05, my + ls * 0.25, 3.0, GREEN);
                draw_line(mx - ls * 0.05, my + ls * 0.25, mx + ls * 0.4, my - ls * 0.3, 3.0, GREEN);
            }
        }
        y += ls * 1.9;
        let tip = if lv < 8 {
            format!("Siguiente: {}. Borra mas lineas para abrirlo.", game::qlevel_name(lv + 1))
        } else {
            format!("Llegaste al SUPERCUANTICO, nivel {}.", lv)
        };
        text_c(&tip, w / 2.0, y, fit_size(&tip, ls * 0.85, pw * 0.94), alpha(WHITE, 0.85));
    }

    /// MUNDO LATERAL: reglas siempre a la vista y niveles que se abren en orden.
    fn draw_puzzles(&self) {
        let (w, h) = (screen_width(), screen_height());
        block_title("LATERAL", w / 2.0, h * 0.1, h * 0.062, w * 0.8, GOLD);
        let ll = level_rects();
        let r = ll.rules;
        panel(r, GOLD);
        let rules = [
            ("1", "SIN RELOJ: nada cae ni se fija; la fijas con CAIDA"),
            ("2", "PIEZAS CONTADAS y una META sobre el tablero"),
            ("3", "LO OBVIO NO BASTA: guarda, gira o entra de lado"),
            ("4", "MENOS PIEZAS = 3 ESTRELLAS"),
            ("5", "Cada nivel resuelto abre el siguiente"),
        ];
        let ls = (r.h / 8.2).min(w * 0.042);
        let mut y = r.y + ls * 1.5;
        let head = "REGLAS DEL MUNDO LATERAL";
        text_c(head, r.x + r.w / 2.0, y, fit_size(head, ls * 1.0, r.w * 0.9), GOLD);
        y += ls * 1.45;
        for (n, t) in rules.iter() {
            draw_circle(r.x + ls * 1.1, y - ls * 0.3, ls * 0.55, GOLD);
            text_c(n, r.x + ls * 1.1, y, ls * 0.85, Color::new(0.08, 0.04, 0.0, 1.0));
            draw_text(t, r.x + ls * 2.1, y, fit_size(t, ls * 0.88, r.w - ls * 2.6), WHITE);
            y += ls * 1.2;
        }
        let total = puzzles::total();
        let pages = (total + PAGE - 1) / PAGE;
        let open = self.lat_unlocked();
        let n_ok = self.solved.iter().filter(|s| **s).count();
        let n_st: u32 = self.stars.iter().map(|s| *s as u32).sum();
        let prog = format!("RESUELTOS {} / {}    ESTRELLAS {}    PAGINA {}/{}", n_ok, total, n_st, self.page + 1, pages);
        let ps = fit_size(&prog, h * 0.02, w * 0.9);
        text_c(&prog, w / 2.0, ll.cells[0].y - h * 0.011, ps, alpha(GOLD, 0.95));
        for (k, r) in ll.cells.iter().enumerate() {
            let i = self.page * PAGE + k;
            if i >= total {
                continue;
            }
            let done = self.solved[i];
            let locked = i > open;
            let c = if done {
                GREEN
            } else if locked {
                Color::new(0.4, 0.42, 0.55, 1.0)
            } else {
                GOLD
            };
            let pulse = if i == open && !done { (self.time * 4.0).sin() * 0.5 + 0.5 } else { 0.0 };
            button(*r, "", c, !locked, pulse);
            let num = format!("{}", i + 1);
            let ns = fit_size(&num, r.h * 0.5, r.w * 0.7);
            let cxn = r.x + r.w / 2.0;
            let ny = r.y + r.h * if locked { 0.44 } else if done { 0.5 } else { 0.66 };
            let a = if locked { 0.45 } else { 1.0 };
            text_c(&num, cxn + 1.5, ny + 1.5, ns, Color::new(0.0, 0.0, 0.0, 0.7 * a));
            text_c(&num, cxn, ny, ns, alpha(WHITE, a));
            if done {
                let sr = (r.h * 0.11).min(r.w * 0.11);
                for s in 0..3 {
                    draw_star(cxn + (s as f32 - 1.0) * sr * 2.3, r.y + r.h * 0.76, sr, (s as u8) < self.stars[i]);
                }
            } else if locked {
                // Candado
                let u = r.h * 0.09;
                let (lx, ly) = (cxn, r.y + r.h * 0.74);
                draw_circle_lines(lx, ly - u * 0.9, u * 0.75, (u * 0.35).max(1.5), alpha(WHITE, 0.55));
                draw_rectangle(lx - u * 1.1, ly - u * 0.7, u * 2.2, u * 1.7, Color::new(0.75, 0.77, 0.85, 0.75));
            }
        }
        button(ll.prev, "<", CYAN, self.page > 0, 0.0);
        button(ll.next, ">", CYAN, self.page + 1 < pages, 0.0);
        button(ll.back, "MENU", MAGENTA, true, 0.0);
    }

    fn draw_pzwin(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let i = self.puzzle.unwrap_or(0);
        let ts = fit_size("RESUELTO!", w * 0.15, w * 0.85);
        let bounce = (self.time * 4.0).sin().abs() * ts * 0.05;
        block_title("RESUELTO!", w / 2.0, h * 0.28 - bounce, h * 0.09, w * 0.88, GREEN);
        let p = puzzles::get(i);
        // Estrellas: 3 por la solucion lateral, 1 por la obvia
        let sr = (h * 0.035).min(w * 0.07);
        let sy = h * 0.28 + ts * 0.9 + sr * 0.2;
        for k in 0..3 {
            let pop = ((self.time * 3.0 - k as f32 * 0.5).max(0.0)).min(1.0);
            draw_star(w / 2.0 + (k as f32 - 1.0) * sr * 2.6, sy, sr * (0.6 + 0.4 * pop), (k as u8) < self.last_stars);
        }
        let mut y = sy + sr * 1.9;
        let s1 = fit_size("Resuelto con 99 piezas. Hay un camino con 99 piezas.", w * 0.045, w * 0.92);
        let lines: Vec<(String, Color)> = if self.last_stars >= 3 {
            let a = if p.dual {
                format!("SOLUCION LATERAL con {}", pzs(self.last_used))
            } else {
                "SOLUCION LATERAL".to_string()
            };
            vec![
                (a, GOLD),
                (format!("Truco: {}", p.trick), TUNNEL_C),
                ("Pensaste de lado. Bien hecho.".to_string(), alpha(WHITE, 0.8)),
            ]
        } else {
            vec![
                (format!("Resuelto a lo obvio con {}.", pzs(self.last_used)), WHITE),
                (format!("Hay un camino lateral con solo {}.", pzs(p.par)), GOLD),
                (format!("Pista: {}", p.hint), TUNNEL_C),
            ]
        };
        for (t, c) in lines.iter() {
            text_c(t, w / 2.0, y, fit_size(t, s1, w * 0.92), *c);
            y += s1 * 1.5;
        }
        let retry = self.last_stars < 3;
        let rs = overlay_rects(if retry { 3 } else { 2 });
        let next = if i + 1 < puzzles::total() { "SIGUIENTE NIVEL" } else { "VER NIVELES" };
        let pulse = (self.time * 4.0).sin() * 0.5 + 0.5;
        if retry {
            button(rs[0], "BUSCAR LA LATERAL", GOLD, true, pulse);
            button(rs[1], next, GREEN, true, 0.0);
            button(rs[2], "NIVELES", MAGENTA, true, 0.0);
        } else {
            button(rs[0], next, GREEN, true, pulse);
            button(rs[1], "NIVELES", GOLD, true, 0.0);
        }
    }

    fn draw_pzfail(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let i = self.puzzle.unwrap_or(0);
        let ts = fit_size("NO FUNCIONO", w * 0.13, w * 0.85);
        block_title("NO FUNCIONO", w / 2.0, h * 0.27, h * 0.08, w * 0.9, MAGENTA);
        let t1 = "Piensa de lado: lo obvio no es la salida";
        let s1 = fit_size(t1, w * 0.045, w * 0.9);
        text_c(t1, w / 2.0, h * 0.27 + ts * 0.9, s1, alpha(WHITE, 0.85));
        if self.show_hint {
            let hint = format!("Pista: {}", puzzles::get(i).hint);
            let s2 = fit_size(&hint, s1, w * 0.92);
            text_c(&hint, w / 2.0, h * 0.27 + ts * 0.9 + s1 * 1.8, s2, GOLD);
        }
        let rs = overlay_rects(3);
        button(rs[0], "REINTENTAR", GREEN, true, 0.0);
        button(rs[1], "PISTA", GOLD, true, 0.0);
        button(rs[2], "NIVELES", MAGENTA, true, 0.0);
    }

    fn draw_pause(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let n = if self.puzzle.is_some() { 5 } else { 4 };
        let rs = overlay_rects(n);
        let ts = fit_size("PAUSA", (w * 0.16).min(h * 0.09), w * 0.8);
        block_title("PAUSA", w / 2.0, rs[0].y - ts * 0.6, h * 0.09, w * 0.8, CYAN);
        let pad_label = if pad_on() { "BOTONES: SI" } else { "BOTONES: NO" };
        if self.puzzle.is_some() {
            button(rs[0], "CONTINUAR", GREEN, true, 0.0);
            button(rs[1], "PISTA", GOLD, true, 0.0);
            button(rs[2], "REINICIAR", CYAN, true, 0.0);
            button(rs[3], "NIVELES", MAGENTA, true, 0.0);
            button(rs[4], pad_label, VIOLET, true, 0.0);
            return;
        }
        button(rs[0], "CONTINUAR", GREEN, true, 0.0);
        button(rs[1], "REINICIAR", GOLD, true, 0.0);
        button(rs[2], pad_label, VIOLET, true, 0.0);
        button(rs[3], "MENU", MAGENTA, true, 0.0);
    }

    fn draw_over(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let ts = fit_size("COLAPSO TOTAL", (w * 0.11).min(h * 0.06), w * 0.9);
        let ty = h * 0.1;
        block_title("COLAPSO TOTAL", w / 2.0, ty, h * 0.06, w * 0.92, MAGENTA);
        let sc = format!("{}", self.game.score);
        let ss = fit_size(&sc, (w * 0.11).min(h * 0.06), w * 0.85);
        glow_text_c(&sc, w / 2.0, ty + ts * 0.6 + ss, ss, GOLD);
        let mut info = format!("NIVEL CUANTICO {}   LINEAS {}", self.game.level, self.game.lines);
        if self.new_best {
            info.push_str("   NUEVO RECORD!");
        }
        let is = fit_size(&info, w * 0.045, w * 0.9);
        text_c(&info, w / 2.0, ty + ts * 0.6 + ss + is * 1.6, is, alpha(if self.new_best { GREEN } else { WHITE }, 0.85));
        let rs = over_rects();
        button(rs[0], "REINTENTAR", GREEN, true, 0.0);
        button(rs[1], "MENU", MAGENTA, true, 0.0);
        let last = rs[1];
        if RETOS_EN_PARTIDA {
            self.draw_training(last.y + last.h * 1.45);
        } else {
            self.draw_qsummary(last.y + last.h * 1.45);
        }
    }

    fn draw_fx(&self) {
        for p in self.particles.iter() {
            let k = p.life / p.max;
            draw_rectangle(p.p.x - p.s / 2.0, p.p.y - p.s / 2.0, p.s, p.s, alpha(p.c, k));
        }
        // Con una tarjeta de explicacion abierta los avisos flotantes la taparian
        let card = self.state == State::Playing && (self.intro || self.explain.is_some());
        for p in self.popups.iter().filter(|_| !card) {
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
