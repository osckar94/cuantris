//! Los dos mundos del juego: MODO VERTICAL (frio, todo cae) y MODO LATERAL
//! (calido, todo fluye de lado). `k` va de 0 (vertical) a 1 (lateral).

use macroquad::prelude::*;
use std::f32::consts::PI;

pub const ORO: Color = Color::new(1.0, 0.78, 0.25, 1.0);
pub const CORAL: Color = Color::new(1.0, 0.42, 0.35, 1.0);
const FRIO: Color = Color::new(0.0, 0.85, 1.0, 1.0);
const AZUL: Color = Color::new(0.45, 0.4, 1.0, 1.0);
const CIRUELA: Color = Color::new(0.85, 0.3, 0.8, 1.0);

/// Duracion del barrido de cambio de mundo (segundos).
pub const BARRIDO: f32 = 0.6;

fn a(c: Color, v: f32) -> Color {
    Color::new(c.r, c.g, c.b, v.max(0.0).min(1.0))
}

fn mezcla(x: Color, y: Color, k: f32) -> Color {
    Color::new(x.r + (y.r - x.r) * k, x.g + (y.g - x.g) * k, x.b + (y.b - x.b) * k, 1.0)
}

fn suave(p: f32) -> f32 {
    let p = p.max(0.0).min(1.0);
    p * p * (3.0 - 2.0 * p)
}

/// Direccion del flujo del mundo: abajo en vertical, de lado en lateral.
pub fn direccion(k: f32) -> Vec2 {
    let ang = k * PI * 0.5;
    vec2(ang.sin(), ang.cos())
}

/// Balanceo del tablero al cambiar de mundo (se asienta solo).
pub fn balanceo(t: f32, cell: f32) -> f32 {
    if t > 1.2 {
        0.0
    } else {
        (t * 17.0).sin() * (-t * 4.5).exp() * cell * 0.45
    }
}

/// Fondo completo. `flow` es el desplazamiento acumulado del mundo.
pub fn fondo(t: f32, k: f32, flow: Vec2) {
    let (w, h) = (screen_width(), screen_height());
    let n = 20;
    for i in 0..n {
        let f = i as f32 / n as f32;
        let frio = Color::new(0.02 + 0.03 * f, 0.01 + 0.02 * (1.0 - f), 0.07 + 0.07 * f, 1.0);
        let calido = Color::new(0.14 + 0.08 * f, 0.03 + 0.06 * f, 0.15 - 0.09 * f, 1.0);
        draw_rectangle(0.0, h * f, w, h / n as f32 + 1.0, mezcla(frio, calido, k));
    }
    // Nebulosas: del indigo a ciruela y ambar
    let neb = [(0.2f32, 0.3f32, AZUL, ORO), (0.8, 0.55, FRIO, CORAL), (0.35, 0.85, CIRUELA, CIRUELA)];
    for (i, &(nx, ny, c0, c1)) in neb.iter().enumerate() {
        let cx = w * (nx + 0.06 * (t * 0.07 + i as f32 * 2.0).sin());
        let cy = h * (ny + 0.04 * (t * 0.05 + i as f32).cos());
        let r0 = w.max(h) * 0.32;
        let c = mezcla(c0, c1, k);
        for j in 0..6 {
            let f = 1.0 - j as f32 / 6.0;
            draw_circle(cx, cy, r0 * f, a(c, 0.016 + 0.006 * k));
        }
    }
    // Ondas que giran con el mundo: de verticales a horizontales
    let dir = direccion(k);
    let perp = vec2(dir.y, -dir.x);
    let centro = vec2(w / 2.0, h / 2.0);
    let largo = (w * w + h * h).sqrt() * 0.55;
    let amp = h * 0.025;
    for i in 0..5 {
        let lado = (i as f32 - 2.0) * largo * 0.32;
        let c = mezcla(if i % 2 == 0 { FRIO } else { AZUL }, if i % 2 == 0 { ORO } else { CORAL }, k);
        let vel = 0.6 + 0.15 * i as f32;
        let mut prev: Option<Vec2> = None;
        for s in 0..=28 {
            let u = s as f32 / 14.0 - 1.0;
            let onda = (u * 5.0 + t * vel).sin() * amp + (u * 13.0 - t * 0.9).sin() * amp * 0.4;
            let p = centro + dir * (u * largo) + perp * (lado + onda);
            if let Some(q) = prev {
                draw_line(q.x, q.y, p.x, p.y, 2.0, a(c, 0.07 + 0.07 * k));
            }
            prev = Some(p);
        }
    }
    // Estrellas y estelas arrastradas por el flujo
    let m = h * 0.1;
    let (bw, bh) = (w + m * 2.0, h + m * 2.0);
    for i in 0..60u32 {
        let sp = 0.3 + crate::hash(i + 500);
        let x = (crate::hash(i * 2) * bw + flow.x * sp * h * 0.012).rem_euclid(bw) - m;
        let y = (crate::hash(i * 2 + 1) * bh + flow.y * sp * h * 0.012).rem_euclid(bh) - m;
        let tw = (t * 1.5 + i as f32).sin() * 0.5 + 0.5;
        let r = (0.6 + 1.4 * crate::hash(i + 77)) * (w / 540.0).max(1.0);
        draw_circle(x, y, r, Color::new(0.75 + 0.25 * k, 0.8, 1.0 - 0.45 * k, 0.12 + 0.4 * tw));
    }
    for i in 0..22u32 {
        let sp = 0.5 + crate::hash(i + 900);
        let x = (crate::hash(i * 3 + 1000) * bw + flow.x * sp * h * 0.2).rem_euclid(bw) - m;
        let y = (crate::hash(i * 3 + 1500) * bh + flow.y * sp * h * 0.2).rem_euclid(bh) - m;
        let len = h * (0.02 + 0.05 * crate::hash(i + 300)) * (1.0 + 0.6 * k);
        let c = mezcla(FRIO, if i % 2 == 0 { ORO } else { CORAL }, k);
        draw_line(x, y, x - dir.x * len, y - dir.y * len, 1.5, a(c, 0.1 + 0.14 * k));
    }
}

/// Barrido a pantalla completa, de lado a lado (`entra`: oro hacia la derecha;
/// al salir: frio hacia la izquierda).
pub fn barrido(p: f32, entra: bool) {
    let (w, h) = (screen_width(), screen_height());
    let e = suave(p);
    let bw = w * 0.6;
    let x0 = if entra { -bw + (w + bw) * e } else { w - (w + bw) * e };
    let (c0, c1) = if entra { (CORAL, ORO) } else { (AZUL, FRIO) };
    let n = 14;
    let paso = bw / n as f32;
    for i in 0..n {
        let f = i as f32 / (n - 1) as f32;
        let d = 1.0 - (f * 2.0 - 1.0).abs();
        let x = x0 + i as f32 * paso;
        draw_rectangle(x, 0.0, paso + 0.5, h, a(mezcla(c0, c1, f), d * d * 0.9));
    }
    let cx = x0 + bw / 2.0;
    draw_line(cx, 0.0, cx, h, (w * 0.006).max(2.0), Color::new(1.0, 1.0, 1.0, 0.9));
}

/// Un chevron animado que apunta hacia `lado` (-1 izquierda, 1 derecha).
pub fn chevrones(x: f32, y: f32, tam: f32, lado: f32, n: usize, t: f32, c: Color) {
    let fase = (t * 1.3).fract();
    let grosor = (tam * 0.28).max(1.5);
    for i in 0..n {
        let q = (i as f32 + fase) / n as f32;
        let cx = x + lado * q * tam * 1.3 * n as f32;
        let al = c.a * (q * PI).sin();
        let col = Color::new(c.r, c.g, c.b, al);
        draw_line(cx - lado * tam * 0.5, y - tam * 0.6, cx + lado * tam * 0.3, y, grosor, col);
        draw_line(cx + lado * tam * 0.3, y, cx - lado * tam * 0.5, y + tam * 0.6, grosor, col);
    }
}

/// Efectos del pozo en modo lateral, bajo los bloques.
pub fn pozo(bx: f32, by: f32, bw: f32, bh: f32, s: f32, t: f32, k: f32) {
    if k < 0.01 {
        return;
    }
    draw_rectangle(bx, by, bw, bh, Color::new(0.2, 0.08, 0.14, 0.2 * k));
    // Rejilla: se apagan las verticales y se encienden las horizontales
    let cols = (bw / s).round() as i32;
    let rows = (bh / s).round() as i32;
    for x in 1..cols {
        let xx = bx + x as f32 * s;
        draw_line(xx, by, xx, by + bh, 1.0, Color::new(0.03, 0.03, 0.08, 0.75 * k));
    }
    for y in 1..rows {
        let yy = by + y as f32 * s;
        draw_line(bx, yy, bx + bw, yy, 1.0, a(ORO, 0.13 * k));
    }
    // Franja de luz que barre de lado a lado
    let ancho = s * 2.0;
    let x = bx + (bw - ancho) * ((t * 0.9).sin() * 0.5 + 0.5);
    for i in 0..5 {
        let d = 1.0 - (i as f32 - 2.0).abs() / 2.5;
        draw_rectangle(x + i as f32 * ancho / 5.0, by, ancho / 5.0 + 0.5, bh, a(ORO, 0.075 * d * k));
    }
    // Chevrones que salen hacia los costados
    let cal = a(ORO, 0.5 * k);
    let mut y = by + s * 2.5;
    while y < by + bh {
        chevrones(bx + s * 1.1, y, s * 0.28, -1.0, 2, t, cal);
        chevrones(bx + bw - s * 1.1, y, s * 0.28, 1.0, 2, t, cal);
        y += s * 4.0;
    }
}

/// Parte un texto en dos lineas lo mas parejas posible.
pub fn dos_lineas(t: &str) -> (String, String) {
    let medio = t.len() / 2;
    let mut mejor: Option<usize> = None;
    for (i, ch) in t.char_indices() {
        if ch == ' ' {
            let d = (i as i32 - medio as i32).abs();
            if mejor.map(|m| d < (m as i32 - medio as i32).abs()).unwrap_or(true) {
                mejor = Some(i);
            }
        }
    }
    match mejor {
        Some(i) => (t[..i].to_string(), t[i + 1..].to_string()),
        None => (t.to_string(), String::new()),
    }
}
