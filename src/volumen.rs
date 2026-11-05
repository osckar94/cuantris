//! Ayudas de dibujo con volumen: degradados, formas redondeadas y fuente de bloques.

use macroquad::prelude::*;
use std::f32::consts::PI;

fn mesh(vertices: Vec<Vertex>, indices: Vec<u16>) {
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// Rectangulo con un color por esquina (sup-izq, sup-der, inf-der, inf-izq).
pub fn grad4(x: f32, y: f32, w: f32, h: f32, tl: Color, tr: Color, br: Color, bl: Color) {
    mesh(
        vec![
            Vertex::new(x, y, 0.0, 0.0, 0.0, tl),
            Vertex::new(x + w, y, 0.0, 1.0, 0.0, tr),
            Vertex::new(x + w, y + h, 0.0, 1.0, 1.0, br),
            Vertex::new(x, y + h, 0.0, 0.0, 1.0, bl),
        ],
        vec![0, 1, 2, 0, 2, 3],
    );
}

/// Degradado vertical.
pub fn grad_v(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    grad4(x, y, w, h, top, top, bottom, bottom);
}

/// Degradado horizontal.
pub fn grad_h(x: f32, y: f32, w: f32, h: f32, left: Color, right: Color) {
    grad4(x, y, w, h, left, right, right, left);
}

fn lerp_color(a: Color, b: Color, k: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * k,
        a.g + (b.g - a.g) * k,
        a.b + (b.b - a.b) * k,
        a.a + (b.a - a.a) * k,
    )
}

/// Rectangulo de esquinas redondeadas con degradado vertical (una sola malla,
/// sin solapes: la transparencia no se acumula).
pub fn rounded_grad(x: f32, y: f32, w: f32, h: f32, r: f32, top: Color, bottom: Color) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let seg = 5;
    let col = |py: f32| lerp_color(top, bottom, ((py - y) / h.max(1.0)).max(0.0).min(1.0));
    let corners = [
        (x + w - r, y + r, -PI / 2.0),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, PI / 2.0),
        (x + r, y + r, PI),
    ];
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let mut v = Vec::with_capacity(4 * (seg + 1) + 1);
    v.push(Vertex::new(cx, cy, 0.0, 0.0, 0.0, col(cy)));
    for &(ox, oy, a0) in corners.iter() {
        for i in 0..=seg {
            let a = a0 + (i as f32 / seg as f32) * PI / 2.0;
            let (px, py) = (ox + r * a.cos(), oy + r * a.sin());
            v.push(Vertex::new(px, py, 0.0, 0.0, 0.0, col(py)));
        }
    }
    let n = (v.len() - 1) as u16;
    let mut idx = Vec::with_capacity(n as usize * 3);
    for i in 0..n {
        idx.push(0);
        idx.push(i + 1);
        idx.push((i + 1) % n + 1);
    }
    mesh(v, idx);
}

/// Rectangulo de esquinas redondeadas de un solo color.
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    rounded_grad(x, y, w, h, r, c, c);
}

/// Grosor del canto de una tecla de alto h.
pub fn key_depth(h: f32) -> f32 {
    (h * 0.2).max(4.0)
}

/// Fuente de bloques 5x7: cada fila es una mascara de 5 bits.
fn glyph(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b11001, 0b10101, 0b10011, 0b10011, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        _ => [0; 7],
    }
}

/// Ancho total de un texto en fuente de bloques.
pub fn block_text_width(t: &str, cell: f32) -> f32 {
    let n = t.chars().count() as f32;
    if n <= 0.0 {
        0.0
    } else {
        (n * 6.0 - 1.0) * cell
    }
}

/// Texto de bloques con volumen, centrado en cx; y es la parte superior.
pub fn block_text_c(t: &str, cx: f32, y: f32, cell: f32, c: Color) {
    let x0 = cx - block_text_width(t, cell) / 2.0;
    let d = (cell * 0.32).max(1.0);
    let side = Color::new(c.r * 0.32, c.g * 0.32, c.b * 0.32, c.a);
    let ground = Color::new(0.0, 0.0, 0.03, 0.4 * c.a);
    let light = Color::new(1.0, 1.0, 1.0, 0.55 * c.a);
    let top = lerp_color(c, Color::new(1.0, 1.0, 1.0, c.a), 0.45);
    let bottom = Color::new(c.r * 0.72, c.g * 0.72, c.b * 0.72, c.a);
    let mut cells: Vec<(f32, f32, usize)> = Vec::new();
    for (i, ch) in t.chars().enumerate() {
        let g = glyph(ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits & (0b10000 >> col) != 0 {
                    let px = x0 + (i * 6 + col) as f32 * cell;
                    cells.push((px, y + row as f32 * cell, row));
                }
            }
        }
    }
    // Tres pasadas: el grosor nunca tapa la cara de una celda vecina.
    for &(px, py, _) in cells.iter() {
        draw_rectangle(px + d * 1.7, py + d * 1.7, cell, cell, ground);
    }
    for &(px, py, _) in cells.iter() {
        draw_rectangle(px + d, py + d, cell, cell, side);
    }
    let face = cell * 0.94;
    for &(px, py, row) in cells.iter() {
        draw_rectangle(px, py, face, face, lerp_color(top, bottom, row as f32 / 6.0));
        draw_rectangle(px, py, face, (cell * 0.2).max(1.0), light);
    }
}
