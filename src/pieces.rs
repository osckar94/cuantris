use macroquad::prelude::*;

// Orden: I, O, T, S, Z, J, L. Coordenadas (x, y) con y hacia abajo.
const BASE: [[(i32, i32); 4]; 7] = [
    [(0, 1), (1, 1), (2, 1), (3, 1)],
    [(1, 0), (2, 0), (1, 1), (2, 1)],
    [(1, 0), (0, 1), (1, 1), (2, 1)],
    [(1, 0), (2, 0), (0, 1), (1, 1)],
    [(0, 0), (1, 0), (1, 1), (2, 1)],
    [(0, 0), (0, 1), (1, 1), (2, 1)],
    [(2, 0), (0, 1), (1, 1), (2, 1)],
];

pub fn cells(kind: usize, rot: usize) -> [(i32, i32); 4] {
    let mut c = BASE[kind];
    if kind == 1 {
        return c;
    }
    let n = if kind == 0 { 4 } else { 3 };
    for _ in 0..(rot % 4) {
        for p in c.iter_mut() {
            *p = (n - 1 - p.1, p.0);
        }
    }
    c
}

pub fn color(kind: usize) -> Color {
    match kind {
        0 => Color::from_rgba(0, 235, 255, 255),
        1 => Color::from_rgba(255, 225, 40, 255),
        2 => Color::from_rgba(200, 70, 255, 255),
        3 => Color::from_rgba(60, 255, 130, 255),
        4 => Color::from_rgba(255, 55, 100, 255),
        5 => Color::from_rgba(70, 120, 255, 255),
        6 => Color::from_rgba(255, 150, 30, 255),
        _ => Color::from_rgba(110, 115, 140, 255),
    }
}

/// Tablas de "wall kicks" del sistema de rotacion estandar (y hacia arriba),
/// convertidas a y hacia abajo al devolverlas.
pub fn kicks(kind: usize, from: usize, to: usize) -> [(i32, i32); 5] {
    let t: [(i32, i32); 5] = if kind == 0 {
        match (from, to) {
            (0, 1) => [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
            (1, 0) => [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
            (1, 2) => [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
            (2, 1) => [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
            (2, 3) => [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
            (3, 2) => [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
            (3, 0) => [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
            _ => [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
        }
    } else {
        match (from, to) {
            (0, 1) | (2, 1) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            (1, 0) | (1, 2) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (2, 3) | (0, 3) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            _ => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        }
    };
    let mut out = t;
    for p in out.iter_mut() {
        p.1 = -p.1;
    }
    out
}
