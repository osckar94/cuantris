mod game;
mod lateral;
mod pieces;
mod puzzles;
mod sfx;

use lateral::{Challenge, Stats, Tech};
use puzzles::{Goal, PUZZLES};

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
    /// Botonera: izquierda, derecha, girar izq, girar der, bajar, caida total.
    pad: Option<[Rect; 6]>,
    sound: Rect,
}

/// Mostrar botonera en pantalla (se cambia desde PAUSA).
static PAD_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

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
    let by = top + ((avail_h - cell * 20.0) / 2.0).max(0.0);
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
        pad: padr,
        sound,
    }
}

fn menu_rects() -> [Rect; 4] {
    let (w, h) = (screen_width(), screen_height());
    let bw = (w * 0.74).min(h * 0.5);
    let bh = (h * 0.075).min(bw * 0.22);
    let x = (w - bw) / 2.0;
    let y0 = h * 0.31;
    let st = bh * 1.28;
    [
        Rect::new(x, y0, bw, bh),
        Rect::new(x, y0 + st, bw, bh),
        Rect::new(x, y0 + st * 2.0, bw, bh),
        Rect::new(x, y0 + st * 3.0, bw, bh),
    ]
}

/// Tarjetas de la lista de acertijos (2 columnas) + boton VOLVER.
fn puzzle_rects() -> (Vec<Rect>, Rect) {
    let (w, h) = (screen_width(), screen_height());
    let area = (w * 0.92).min(h * 0.62);
    let x0 = (w - area) / 2.0;
    let gap = area * 0.04;
    let cw = (area - gap) / 2.0;
    let ch = (h * 0.12).min(cw * 0.62);
    let y0 = h * 0.25;
    let mut v = Vec::new();
    for i in 0..PUZZLES.len() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        v.push(Rect::new(x0 + col * (cw + gap), y0 + row * (ch + gap * 1.2), cw, ch));
    }
    let rows = ((PUZZLES.len() + 1) / 2) as f32;
    let by = y0 + rows * (ch + gap * 1.2) + gap;
    let bw = area * 0.6;
    let back = Rect::new((w - bw) / 2.0, by, bw, (h * 0.065).min(bw * 0.25));
    (v, back)
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

/// Bloque con volumen: sombra proyectada, biseles iluminados desde arriba a la
/// izquierda, cara con degradado y brillo especular.
fn draw_block(x: f32, y: f32, s: f32, c: Color, a: f32) {
    // Halo de luz suave
    let g = s * 0.14;
    draw_rectangle(x - g, y - g, s + 2.0 * g, s + 2.0 * g, alpha(c, 0.07 * a));
    // Sombra proyectada (abajo a la derecha)
    let so = s * 0.12;
    draw_rectangle(x + so, y + so, s, s, Color::new(0.0, 0.0, 0.02, 0.35 * a));

    let b = (s * 0.17).max(1.5);
    let (x0, y0, x1, y1) = (x, y, x + s, y + s);
    let (ix0, iy0, ix1, iy1) = (x + b, y + b, x + s - b, y + s - b);
    let white = Color::new(1.0, 1.0, 1.0, a);
    // Biseles: arriba (mas claro), izquierda, derecha, abajo (mas oscuro)
    quad(vec2(x0, y0), vec2(x1, y0), vec2(ix1, iy0), vec2(ix0, iy0), alpha(mix(c, white, 0.55), a));
    quad(vec2(x0, y0), vec2(ix0, iy0), vec2(ix0, iy1), vec2(x0, y1), alpha(mix(c, white, 0.25), a));
    quad(vec2(x1, y0), vec2(x1, y1), vec2(ix1, iy1), vec2(ix1, iy0), shade(c, 0.55, a));
    quad(vec2(x0, y1), vec2(ix0, iy1), vec2(ix1, iy1), vec2(x1, y1), shade(c, 0.35, a));
    // Cara con degradado vertical
    let fw = ix1 - ix0;
    let fh = iy1 - iy0;
    let bands = 4;
    for k in 0..bands {
        let f = k as f32 / (bands - 1) as f32;
        let col = mix(mix(c, white, 0.18), shade(c, 0.8, 1.0), f);
        draw_rectangle(ix0, iy0 + fh * k as f32 / bands as f32, fw, fh / bands as f32 + 0.5, alpha(col, a));
    }
    // Brillo especular
    draw_rectangle(ix0 + fw * 0.08, iy0 + fh * 0.1, fw * 0.45, fh * 0.14, Color::new(1.0, 1.0, 1.0, 0.45 * a));
    draw_circle(ix0 + fw * 0.2, iy0 + fh * 0.42, (s * 0.05).max(0.8), Color::new(1.0, 1.0, 1.0, 0.35 * a));
    // Contorno fino
    draw_rectangle_lines(x0, y0, s, s, (s * 0.04).max(1.0), Color::new(0.0, 0.0, 0.0, 0.45 * a));
}

/// Marca de pieza TUNEL: flecha hacia abajo con anillos que bajan.
fn tunnel_mark(cx: f32, cy: f32, s: f32, t: f32) {
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

/// Panel hundido (vitrina): sombra interior arriba/izquierda, luz abajo/derecha.
fn panel(r: Rect, c: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.08, 0.85));
    let d = (r.h.min(r.w) * 0.08).max(2.0);
    for i in 0..3 {
        let k = i as f32 / 3.0;
        let a = 0.35 * (1.0 - k);
        draw_rectangle(r.x, r.y + d * k, r.w, d / 3.0, Color::new(0.0, 0.0, 0.0, a));
        draw_rectangle(r.x + d * k, r.y, d / 3.0, r.h, Color::new(0.0, 0.0, 0.0, a));
    }
    draw_line(r.x, r.y + r.h, r.x + r.w, r.y + r.h, 2.0, alpha(c, 0.55));
    draw_line(r.x + r.w, r.y, r.x + r.w, r.y + r.h, 2.0, alpha(c, 0.35));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, alpha(c, 0.3));
}

/// Boton con volumen: canto inferior (grosor), cara con degradado y brillo.
fn button(r: Rect, label: &str, c: Color, enabled: bool, pulse: f32) {
    let a = if enabled { 1.0 } else { 0.35 };
    let depth = (r.h * 0.13).max(3.0);
    // Sombra en el suelo y canto del boton
    draw_rectangle(r.x + depth * 0.6, r.y + depth * 1.6, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35 * a));
    draw_rectangle(r.x, r.y + depth, r.w, r.h, shade(c, 0.28, 0.95 * a));
    draw_rectangle_lines(r.x, r.y + depth, r.w, r.h, 1.5, shade(c, 0.5, 0.8 * a));
    // Cara
    let bands = 5;
    for k in 0..bands {
        let f = k as f32 / (bands - 1) as f32;
        let col = mix(shade(c, 0.42, 1.0), shade(c, 0.16, 1.0), f);
        draw_rectangle(r.x, r.y + r.h * k as f32 / bands as f32, r.w, r.h / bands as f32 + 0.5, alpha(col, 0.95 * a));
    }
    if pulse > 0.0 {
        draw_rectangle(r.x, r.y, r.w, r.h, alpha(c, 0.2 * pulse));
    }
    // Brillo superior (vidrio)
    draw_rectangle(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h * 0.32, Color::new(1.0, 1.0, 1.0, 0.10 * a));
    draw_line(r.x + 2.0, r.y + 1.5, r.x + r.w - 2.0, r.y + 1.5, 2.0, alpha(mix(c, WHITE, 0.6), 0.8 * a));
    draw_rectangle_lines(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + depth + 6.0, 3.0, alpha(c, 0.12 * a));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, alpha(c, 0.9 * a));
    if !label.is_empty() {
        let size = fit_size(label, r.h * 0.42, r.w * 0.86);
        let ty = r.y + r.h / 2.0 + size * 0.32;
        text_c(label, r.x + r.w / 2.0 + 1.5, ty + 1.5, size, Color::new(0.0, 0.0, 0.0, 0.6 * a));
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
    if spec.tunnel {
        for &(x, y) in cells.iter() {
            tunnel_mark(ox + (x as f32 + 0.5) * ms, oy + (y as f32 + 0.5) * ms, ms, t);
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
    active: Vec<u64>,
    /// Acertijo en juego (indice en PUZZLES)
    puzzle: Option<usize>,
    solved: Vec<bool>,
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
            active: Vec::new(),
            das_dir: 0,
            das_t: 0.0,
            arr_t: 0.0,
            sfx,
            menu_pieces,
            puzzle: None,
            solved: vec![false; PUZZLES.len()],
            show_hint: false,
            challenge: None,
            next_ch: 5,
            last_tech: None,
            game_stats: Stats::default(),
            session_stats: Stats::default(),
            ent_flag: false,
            last_result: None,
            pad_held: Vec::new(),
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
            Tech::Orden | Tech::Inesperado => &[],
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
        self.challenge = Some(Challenge {
            tech,
            left: tech.pieces(),
            t: 0.0,
        });
        self.last_result = None;
        self.popup(l, "RETO LATERAL!", GOLD, true);
        self.sfx.play(S::Entangle, 0.6);
    }

    /// Revisa el reto activo tras colocar una pieza.
    #[allow(clippy::too_many_arguments)]
    fn on_placed(&mut self, l: &Lay, kind: usize, rot: usize, from_hold: bool, observed: bool, tunneled: bool, lines: usize) {
        if self.puzzle.is_some() || self.state != State::Playing {
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
            self.popup(l, &format!("PENSASTE DE LADO! +{}", bonus), GREEN, true);
            self.sfx.play(S::Level, 0.6);
            for k in 0..20 {
                let p = vec2(l.bx + l.cell * (k % 10) as f32 + l.cell * 0.5, l.by + l.cell * 2.0);
                self.burst(p, GOLD, 5, l.cell * 7.0, l.cell * 0.18);
            }
            self.challenge = None;
            self.last_result = Some((tech, true, 0.0));
            self.next_ch = 7 + game::rand_int(4) as u32;
        } else if left == 0 {
            self.game_stats.add(tech, false);
            self.session_stats.add(tech, false);
            self.popup(l, "RETO NO SUPERADO", MAGENTA, false);
            self.challenge = None;
            self.last_result = Some((tech, false, 0.0));
            self.next_ch = 6 + game::rand_int(3) as u32;
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
        let i = i.min(PUZZLES.len() - 1);
        self.puzzle = Some(i);
        self.game = Game::from_puzzle(&PUZZLES[i]);
        self.clear_fx();
        self.show_hint = hint;
        self.new_best = false;
        self.sfx.play(S::Click, 0.5);
        self.set_state(State::Playing);
        let l = lay();
        let name = format!("ACERTIJO {}", i + 1);
        self.popup(&l, &name, GOLD, true);
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
        let done = match PUZZLES[i].goal {
            Goal::ClearAll => self.game.board_empty(),
            Goal::Lines(n) => self.game.lines >= n,
        };
        if done || self.game.out_of_pieces || self.game.over {
            // Los avisos flotantes taparian la ventana de resultado.
            self.popups.clear();
        }
        if done {
            self.solved[i] = true;
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
        self.pad_held.clear();
        self.gestures.clear();
        self.block_input = true;
    }

    fn start(&mut self, mode: Mode) {
        self.puzzle = None;
        self.challenge = None;
        self.next_ch = 5;
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
            State::Puzzles => self.update_puzzles(&ptrs, &l),
            State::PzWin => self.update_pzwin(&ptrs, &l),
            State::PzFail => self.update_pzfail(&ptrs, &l),
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
                        if i < 3 {
                            self.start(MODES[i]);
                        } else {
                            self.sfx.play(S::Click, 0.5);
                            self.set_state(State::Puzzles);
                        }
                        return;
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::Key4) {
            self.set_state(State::Puzzles);
        } else if is_key_pressed(KeyCode::Key1) {
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
        let soft = is_key_down(KeyCode::Down) || pad_soft;
        self.game.update(dt, soft);
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
        let (cards, back) = puzzle_rects();
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                for (i, r) in cards.iter().enumerate() {
                    if r.contains(pos) {
                        self.start_puzzle(i, false);
                        return;
                    }
                }
                if back.contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Menu);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Escape) {
            self.set_state(State::Menu);
        }
    }

    fn update_pzwin(&mut self, ptrs: &[Ptr], l: &Lay) {
        let rs = overlay_rects(2);
        let i = self.puzzle.unwrap_or(0);
        for p in ptrs {
            if let Ptr::Down(_, pos) = *p {
                if self.sound_hit(pos, l) {
                    continue;
                }
                if rs[0].contains(pos) {
                    if i + 1 < PUZZLES.len() {
                        self.start_puzzle(i + 1, false);
                    } else {
                        self.set_state(State::Puzzles);
                    }
                    return;
                }
                if rs[1].contains(pos) {
                    self.sfx.play(S::Click, 0.5);
                    self.set_state(State::Puzzles);
                    return;
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            self.start_puzzle(i + 1, false);
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
                    let m = self.mode;
                    self.start(m);
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
                Event::Placed { kind, rot, from_hold, observed, tunneled, lines } => {
                    self.on_placed(l, kind, rot, from_hold, observed, tunneled, lines);
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
        draw_bg(self.time);
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
                }
            }
        }
        self.draw_fx();
        draw_sound_icon(l.sound, self.sfx.on);    }

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
        let ts = fit_size("CUANTRIS", (w * 0.2).min(h * 0.11), w * 0.86);
        let ty = h * 0.15;
        let wob = (self.time * 2.0).sin() * ts * 0.02;
        text_3d_c("CUANTRIS", w / 2.0, ty + wob, ts, CYAN);
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
            ("ACERTIJOS", "Pensamiento lateral  -  8 retos", GOLD),
        ];
        for (i, r) in rs.iter().enumerate() {
            let (name, desc, c) = info[i];
            let pulse = ((self.time * 2.0 + i as f32).sin() * 0.5 + 0.5) * 0.5;
            button(*r, "", c, true, pulse);
            draw_rectangle(r.x + 2.0, r.y + r.h * 0.62, r.w - 4.0, r.h * 0.38 - 2.0, Color::new(c.r * 0.05, c.g * 0.05, c.b * 0.09, 0.75));
            draw_line(r.x + 2.0, r.y + r.h * 0.62, r.x + r.w - 2.0, r.y + r.h * 0.62, 1.0, alpha(c, 0.5));
            // Mini bloque 3D decorativo a cada lado del nombre
            let bs = r.h * 0.3;
            draw_block(r.x + r.h * 0.18, r.y + r.h * 0.16, bs, c, 1.0);
            draw_block(r.x + r.w - r.h * 0.18 - bs, r.y + r.h * 0.16, bs, c, 1.0);
            let s1 = fit_size(name, r.h * 0.38, r.w * 0.7);
            text_c(name, r.x + r.w / 2.0 + 2.0, r.y + r.h * 0.31 + s1 * 0.32 + 2.0, s1, Color::new(0.0, 0.0, 0.0, 0.6));
            text_c(name, r.x + r.w / 2.0, r.y + r.h * 0.31 + s1 * 0.32, s1, WHITE);
            let s2 = fit_size(desc, r.h * 0.22, r.w * 0.9);
            text_c(desc, r.x + r.w / 2.0, r.y + r.h * 0.81 + s2 * 0.3, s2, alpha(c, 0.9));
        }

        let help = [
            "Botones o gestos: desliza mover, toca girar",
            "Desliza rapido abajo: caida total",
            "Desliza arriba: guardar pieza",
            "SUPERPOSICION: la pieza esta en 2 lugares",
            "y colapsa al azar al tocar fondo",
            "ENTRELAZADAS: se borran juntas a distancia",
            "TUNEL: la pieza con flecha atraviesa bloques",
            "NUEVO! Retos laterales en cada partida",
        ];
        let last = rs[3];
        let mut y = last.y + last.h + h * 0.045;
        let hs = help
            .iter()
            .map(|t| fit_size(t, (w * 0.04).min(h * 0.021), w * 0.92))
            .fold(f32::MAX, f32::min);
        for (i, t) in help.iter().enumerate() {
            let c = if i == 7 {
                alpha(GOLD, 0.95)
            } else if i >= 6 {
                alpha(TUNNEL_C, 0.95)
            } else if i >= 3 {
                alpha(VIOLET, 0.95)
            } else {
                alpha(WHITE, 0.75)
            };
            text_c(t, w / 2.0, y, hs, c);
            y += hs * 1.45;
        }
        let (ok, tried) = self.session_stats.total();
        if self.best > 0 || tried > 0 {
            let t = format!(
                "RECORD: {}    INDICE LATERAL: {}  ({}/{} retos)",
                self.best,
                self.session_stats.index(),
                ok,
                tried
            );
            let ts2 = fit_size(&t, hs * 1.05, w * 0.94);
            text_c(&t, w / 2.0, (y + hs).min(h * 0.98), ts2, GOLD);
        }
    }

    fn draw_game(&self, l: &Lay) {
        let w = screen_width();
        if let Some(i) = self.puzzle {
            let p = &PUZZLES[i];
            let ts = fit_size(p.name, l.top * 0.5, w * 0.62);
            text_3d_c(p.name, w / 2.0, l.top * 0.7, ts, GOLD);
            let tag = format!("RETO {}/{}", i + 1, PUZZLES.len());
            let ms = fit_size(&tag, l.top * 0.26, w * 0.17);
            draw_text(&tag, w * 0.03, l.top * 0.62, ms, alpha(TUNNEL_C, 0.85));
            // Meta y pista entre el titulo y el tablero
            let fr = (l.cell * 0.3).max(4.0);
            let space = (l.by - fr - l.top).max(l.cell * 1.2);
            let lines_n = if self.show_hint { 2.6 } else { 1.4 };
            let gs = (space / lines_n).min(l.cell * 0.7);
            let goal = p.goal.text();
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
        } else {
            let ts = fit_size("CUANTRIS", l.top * 0.55, w * 0.5);
            text_3d_c("CUANTRIS", w / 2.0, l.top * 0.7, ts, CYAN);
            let ms = fit_size(self.game.mode.name(), l.top * 0.28, w * 0.25);
            draw_text(self.game.mode.name(), w * 0.04, l.top * 0.62, ms, alpha(MAGENTA, 0.8));
            self.draw_lateral_banner(l);
        }
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
        // En acertijo el boton no parpadea: descubrir que hay que observar es parte del reto.
        let pulse = if can_obs && !g.puzzle { (self.time * 7.0).sin() * 0.5 + 0.5 } else { 0.0 };
        button(l.buttons[1], &format!("OBSERVAR {}", g.charges), GOLD, can_obs, pulse);
        button(l.buttons[2], if g.puzzle { "OPCIONES" } else { "PAUSA" }, CYAN, true, 0.0);
        if let Some(pad) = l.pad {
            self.draw_pad(&pad);
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
        draw_rectangle(ox0 + fr * 0.6, oy0 + fr * 0.9, ox1 - ox0, oy1 - oy0, Color::new(0.0, 0.0, 0.0, 0.45));
        let frame = Color::new(0.12, 0.2, 0.38, 1.0);
        quad(vec2(ox0, oy0), vec2(ox1, oy0), vec2(bx + bw, by), vec2(bx, by), mix(frame, WHITE, 0.25));
        quad(vec2(ox0, oy0), vec2(bx, by), vec2(bx, by + bh), vec2(ox0, oy1), mix(frame, WHITE, 0.1));
        quad(vec2(ox1, oy0), vec2(ox1, oy1), vec2(bx + bw, by + bh), vec2(bx + bw, by), shade(frame, 0.6, 1.0));
        quad(vec2(ox0, oy1), vec2(bx, by + bh), vec2(bx + bw, by + bh), vec2(ox1, oy1), shade(frame, 0.4, 1.0));
        draw_rectangle_lines(ox0, oy0, ox1 - ox0, oy1 - oy0, 2.0, Color::new(0.3, 0.8, 1.0, 0.35 + 0.2 * pulse));
        draw_line(ox0 + 2.0, oy0 + 1.5, ox1 - 2.0, oy0 + 1.5, 1.5, Color::new(0.7, 0.95, 1.0, 0.5));
        // Fondo del pozo con degradado (mas oscuro arriba = profundidad)
        let bands = 10;
        for k in 0..bands {
            let f = k as f32 / bands as f32;
            draw_rectangle(bx, by + bh * f, bw, bh / bands as f32 + 0.5, Color::new(0.015 + 0.02 * f, 0.02 + 0.03 * f, 0.06 + 0.06 * f, 0.92));
        }
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
        let stats = if let Some(i) = self.puzzle {
            let left = g.remaining() + g.active.map(|_| 1).unwrap_or(0);
            let goal = match PUZZLES[i].goal {
                Goal::Lines(n) => format!("{}/{}", g.lines.min(n), n),
                Goal::ClearAll => {
                    let n = g.grid.iter().flatten().filter(|c| c.kind != 0).count();
                    format!("{}", n)
                }
            };
            let gl = match PUZZLES[i].goal {
                Goal::Lines(_) => "LINEAS",
                Goal::ClearAll => "BLOQUES",
            };
            [
                ("PIEZAS", left.to_string()),
                (gl, goal),
                ("OBSERVAR", g.charges.to_string()),
            ]
        } else {
            [
                ("PUNTOS", g.score.to_string()),
                ("NIVEL", g.level.to_string()),
                ("LINEAS", g.lines.to_string()),
            ]
        };
        for (name, val) in stats.iter() {
            draw_text(name, x, y + lab * 0.8, lab * 0.9, alpha(WHITE, 0.55));
            let vs = fit_size(val, s * 0.9, sw);
            draw_text(val, x, y + lab + vs * 0.85, vs, WHITE);
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

    /// Botonera de control con iconos dibujados (la fuente no trae flechas).
    fn draw_pad(&self, pad: &[Rect; 6]) {
        let cols = [CYAN, CYAN, VIOLET, VIOLET, GREEN, MAGENTA];
        for (i, r) in pad.iter().enumerate() {
            let held = self.pad_held.iter().any(|h| h.1 == i);
            let c = cols[i];
            // Al pulsar, el boton "se hunde": baja hasta su canto
            let depth = (r.h * 0.13).max(3.0);
            let rr = if held { Rect::new(r.x, r.y + depth * 0.8, r.w, r.h) } else { *r };
            button(rr, "", c, true, if held { 1.0 } else { 0.0 });
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

    /// Aviso del reto lateral entre el titulo y el tablero.
    fn draw_lateral_banner(&self, l: &Lay) {
        let w = screen_width();
        let fr = (l.cell * 0.3).max(4.0);
        let space = (l.by - fr - l.top).max(l.cell * 1.2);
        let gs = (space / 3.5).min(l.cell * 0.62);
        let base = l.by - fr - 6.0;
        if let Some(c) = &self.challenge {
            let pulse = (self.time * 5.0).sin() * 0.5 + 0.5;
            // Marco dorado alrededor del tablero mientras hay reto
            let bw = l.cell * W as f32;
            let bh = l.cell * (H - HIDDEN) as f32;
            draw_rectangle_lines(l.bx - fr - 3.0, l.by - fr - 3.0, bw + 2.0 * fr + 6.0, bh + 2.0 * fr + 6.0, 3.0, alpha(GOLD, 0.35 + 0.45 * pulse));
            let t1 = format!("RETO LATERAL: {}  ({} piezas)", c.tech.name(), c.left);
            let s1 = fit_size(&t1, gs, w * 0.95);
            text_c(&t1, w / 2.0 + 1.0, base - gs * 2.3 + 1.0, s1, Color::new(0.0, 0.0, 0.0, 0.7));
            text_c(&t1, w / 2.0, base - gs * 2.3, s1, GOLD);
            let s2 = fit_size(c.tech.question(), gs * 0.95, w * 0.95);
            text_c(c.tech.question(), w / 2.0, base - gs * 1.15, s2, TUNNEL_C);
            let s3 = fit_size(c.tech.goal(), gs * 0.85, w * 0.95);
            text_c(c.tech.goal(), w / 2.0, base, s3, alpha(WHITE, 0.9));
        } else if let Some((tech, ok, t)) = self.last_result {
            let a = (1.0 - (t - 5.5).max(0.0) / 1.5).max(0.0);
            let t1 = if ok {
                format!("SUPERADO: {}", tech.name())
            } else {
                format!("NO SUPERADO: {}", tech.name())
            };
            let s1 = fit_size(&t1, gs, w * 0.95);
            text_c(&t1, w / 2.0, base - gs * 1.2, s1, alpha(if ok { GREEN } else { MAGENTA }, a));
            let s2 = fit_size(tech.lesson(), gs * 0.85, w * 0.95);
            text_c(tech.lesson(), w / 2.0, base, s2, alpha(WHITE, 0.85 * a));
        } else {
            let t1 = format!("Proximo reto lateral en {} piezas", self.next_ch);
            let s1 = fit_size(&t1, gs * 0.85, w * 0.9);
            text_c(&t1, w / 2.0, base, s1, alpha(WHITE, 0.45));
        }
    }

    /// Resumen del entrenamiento lateral (pantalla de fin de partida).
    fn draw_training(&self, y0: f32) {
        let (w, h) = (screen_width(), screen_height());
        let pw = (w * 0.9).min(h * 0.55);
        let px = (w - pw) / 2.0;
        let ph = (h - y0 - h * 0.02).max(h * 0.2);
        panel(Rect::new(px, y0, pw, ph), GOLD);
        let ls = (ph / 10.5).min(w * 0.045);
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

    fn draw_puzzles(&self) {
        let (w, h) = (screen_width(), screen_height());
        let ts = fit_size("ACERTIJOS", w * 0.15, w * 0.86);
        text_3d_c("ACERTIJOS", w / 2.0, h * 0.11, ts, GOLD);
        let sub = "Pensamiento lateral: cada reto tiene un truco";
        let ss = fit_size(sub, ts * 0.3, w * 0.9);
        text_c(sub, w / 2.0, h * 0.11 + ts * 0.55, ss, alpha(TUNNEL_C, 0.9));
        let sub2 = "Sin gravedad: piensa con calma";
        let ss2 = fit_size(sub2, ss, w * 0.9);
        text_c(sub2, w / 2.0, h * 0.11 + ts * 0.55 + ss * 1.5, ss2, alpha(WHITE, 0.6));
        let (cards, back) = puzzle_rects();
        let n_ok = self.solved.iter().filter(|s| **s).count();
        let prog = format!("Resueltos: {} / {}", n_ok, PUZZLES.len());
        let ps = fit_size(&prog, ss, w * 0.6);
        text_c(&prog, w / 2.0, cards[0].y - ps * 0.7, ps, alpha(GOLD, 0.9));
        for (i, r) in cards.iter().enumerate() {
            let p = &PUZZLES[i];
            let ok = self.solved[i];
            let c = if ok { GREEN } else { [CYAN, VIOLET, MAGENTA, GOLD][i % 4] };
            button(*r, "", c, true, 0.0);
            let num = format!("{}", i + 1);
            let ns = r.h * 0.5;
            text_c(&num, r.x + r.h * 0.32 + 2.0, r.y + r.h * 0.55 + 2.0, ns, Color::new(0.0, 0.0, 0.0, 0.6));
            text_c(&num, r.x + r.h * 0.32, r.y + r.h * 0.55, ns, mix(c, WHITE, 0.5));
            let tx = r.x + r.h * 0.62;
            let tw = r.w - r.h * 0.7;
            let s1 = fit_size(p.name, r.h * 0.26, tw);
            draw_text(p.name, tx, r.y + r.h * 0.42, s1, WHITE);
            let info = if ok { format!("OK - {}", p.trick) } else { p.goal.text().replace("Meta: ", "") };
            let s2 = fit_size(&info, r.h * 0.17, tw);
            draw_text(&info, tx, r.y + r.h * 0.72, s2, alpha(if ok { GREEN } else { c }, 0.9));
        }
        button(back, "VOLVER", MAGENTA, true, 0.0);
    }

    fn draw_pzwin(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let i = self.puzzle.unwrap_or(0);
        let ts = fit_size("RESUELTO!", w * 0.15, w * 0.85);
        let bounce = (self.time * 4.0).sin().abs() * ts * 0.05;
        text_3d_c("RESUELTO!", w / 2.0, h * 0.28 - bounce, ts, GREEN);
        let t1 = format!("Truco: {}", PUZZLES[i].trick);
        let s1 = fit_size(&t1, w * 0.055, w * 0.9);
        text_c(&t1, w / 2.0, h * 0.28 + ts * 0.9, s1, TUNNEL_C);
        let t2 = "Pensaste de lado. Bien hecho.";
        let s2 = fit_size(t2, s1 * 0.85, w * 0.9);
        text_c(t2, w / 2.0, h * 0.28 + ts * 0.9 + s1 * 1.6, s2, alpha(WHITE, 0.8));
        let rs = overlay_rects(2);
        let next = if i + 1 < PUZZLES.len() { "SIGUIENTE" } else { "VER TODOS" };
        button(rs[0], next, GREEN, true, (self.time * 4.0).sin() * 0.5 + 0.5);
        button(rs[1], "ACERTIJOS", GOLD, true, 0.0);
    }

    fn draw_pzfail(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let i = self.puzzle.unwrap_or(0);
        let ts = fit_size("NO FUNCIONO", w * 0.13, w * 0.85);
        text_3d_c("NO FUNCIONO", w / 2.0, h * 0.27, ts, MAGENTA);
        let t1 = "Piensa de lado: lo obvio no es la salida";
        let s1 = fit_size(t1, w * 0.045, w * 0.9);
        text_c(t1, w / 2.0, h * 0.27 + ts * 0.9, s1, alpha(WHITE, 0.85));
        if self.show_hint {
            let hint = format!("Pista: {}", PUZZLES[i].hint);
            let s2 = fit_size(&hint, s1, w * 0.92);
            text_c(&hint, w / 2.0, h * 0.27 + ts * 0.9 + s1 * 1.8, s2, GOLD);
        }
        let rs = overlay_rects(3);
        button(rs[0], "REINTENTAR", GREEN, true, 0.0);
        button(rs[1], "PISTA", GOLD, true, 0.0);
        button(rs[2], "ACERTIJOS", MAGENTA, true, 0.0);
    }

    fn draw_pause(&self) {
        self.draw_overlay_bg();
        let (w, h) = (screen_width(), screen_height());
        let n = if self.puzzle.is_some() { 5 } else { 4 };
        let rs = overlay_rects(n);
        let ts = fit_size("PAUSA", (w * 0.16).min(h * 0.09), w * 0.8);
        text_3d_c("PAUSA", w / 2.0, rs[0].y - ts * 0.6, ts, CYAN);
        let pad_label = if pad_on() { "BOTONES: SI" } else { "BOTONES: NO" };
        if self.puzzle.is_some() {
            button(rs[0], "CONTINUAR", GREEN, true, 0.0);
            button(rs[1], "PISTA", GOLD, true, 0.0);
            button(rs[2], "REINICIAR", CYAN, true, 0.0);
            button(rs[3], "ACERTIJOS", MAGENTA, true, 0.0);
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
        text_3d_c("COLAPSO TOTAL", w / 2.0, ty, ts, MAGENTA);
        let sc = format!("{}", self.game.score);
        let ss = fit_size(&sc, (w * 0.11).min(h * 0.06), w * 0.85);
        glow_text_c(&sc, w / 2.0, ty + ts * 0.6 + ss, ss, GOLD);
        let mut info = format!("NIVEL {}   LINEAS {}", self.game.level, self.game.lines);
        if self.new_best {
            info.push_str("   NUEVO RECORD!");
        }
        let is = fit_size(&info, w * 0.045, w * 0.9);
        text_c(&info, w / 2.0, ty + ts * 0.6 + ss + is * 1.6, is, alpha(if self.new_best { GREEN } else { WHITE }, 0.85));
        let rs = over_rects();
        button(rs[0], "REINTENTAR", GREEN, true, 0.0);
        button(rs[1], "MENU", MAGENTA, true, 0.0);
        let last = rs[1];
        self.draw_training(last.y + last.h * 1.45);
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
