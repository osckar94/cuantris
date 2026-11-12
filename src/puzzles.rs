//! Modo ACERTIJOS: tableros armados que exigen pensamiento lateral.
//!
//! Los 8 primeros estan hechos a mano. El resto (BANK) los genera y comprueba
//! el programa de herramientas/generador con un buscador que usa las mismas
//! reglas del juego: cada reto tiene solucion y se sabe cuantas piezas pide.
//!
//! Hay dos clases:
//!   - solo lateral (dual = false): sin el truco NO se puede resolver.
//!   - dual (dual = true): se puede resolver "a lo obvio" usando todas las
//!     piezas (1 estrella), pero con el truco bastan `par` piezas (3 estrellas).
//!
//! Tablero (filas de arriba hacia abajo, pegadas al fondo):
//!   '.' vacio, '#' bloque, 'A'..'Z' bloque entrelazado (mitad 1),
//!   'a'..'z' su gemelo (mitad 2).
//! Piezas: letra I O T S Z J L + sufijos: t = tunel, e = entrelazada, s = superposicion.

pub enum Goal {
    ClearAll,
    Lines(u32),
}

pub struct Puzzle {
    #[allow(dead_code)]
    pub name: &'static str,
    pub rows: &'static [&'static str],
    pub pieces: &'static [&'static str],
    pub goal: Goal,
    /// Grupo entrelazado cuyo gemelo sera la proxima pieza entrelazada.
    pub pending: Option<char>,
    pub charges: u32,
    pub hint: &'static str,
    pub trick: &'static str,
    /// true: tiene una salida obvia pero peor (mas piezas) y otra lateral.
    pub dual: bool,
    /// Piezas que pide la solucion lateral (solo importa si dual).
    pub par: u32,
    /// Solucion lateral guardada por el generador (la repite Jev): "H;P1,4,19".
    pub sol: &'static str,
}

pub const PUZZLES: &[Puzzle] = &[
    Puzzle {
        name: "Atajo",
        rows: &[
            "#########.",
            "####..####",
            "####..####",
        ],
        pieces: &["Ot"],
        goal: Goal::Lines(2),
        pending: None,
        charges: 0,
        hint: "Lo imposible por arriba quizas no lo sea por dentro.",
        trick: "TUNEL",
        dual: false,
        par: 1,
        sol: "",
    },
    Puzzle {
        name: "Muro",
        rows: &[
            "#.#.######",
            "##.#.#####",
            "######.###",
            "######.###",
            "######.###",
            "######.###",
        ],
        pieces: &["It"],
        goal: Goal::Lines(4),
        pending: None,
        charges: 0,
        hint: "Un muro no detiene a quien se pone de pie.",
        trick: "GIRAR + TUNEL",
        dual: false,
        par: 1,
        sol: "",
    },
    Puzzle {
        name: "Al reves",
        rows: &[
            "#########.",
            "#########.",
            "#########.",
            "#########.",
            "###..#####",
            "###..#####",
        ],
        pieces: &["O", "I"],
        goal: Goal::ClearAll,
        pending: None,
        charges: 0,
        hint: "No siempre hay que jugar la primera carta.",
        trick: "GUARDAR",
        dual: false,
        par: 2,
        sol: "",
    },
    Puzzle {
        name: "El observador",
        rows: &[
            "#######.##",
            "#######.##",
            "#######.##",
            "#######.##",
        ],
        pieces: &["Is"],
        goal: Goal::ClearAll,
        pending: None,
        charges: 1,
        hint: "Mirar cambia la realidad.",
        trick: "OBSERVAR",
        dual: false,
        par: 1,
        sol: "",
    },
    Puzzle {
        name: "A distancia",
        rows: &[
            "#######a..",
            "########..",
            "AA......AA",
            "..########",
            "..########",
        ],
        pieces: &["O", "O"],
        goal: Goal::ClearAll,
        pending: None,
        charges: 0,
        hint: "Para abrir abajo, completa arriba.",
        trick: "ENTRELAZADO",
        dual: false,
        par: 2,
        sol: "",
    },
    Puzzle {
        name: "La llave",
        rows: &[
            "#########.",
            "#########.",
            "#########.",
            "#########.",
            "BB......BB",
            "..########",
            "..########",
        ],
        pieces: &["Ie", "O"],
        goal: Goal::ClearAll,
        pending: Some('B'),
        charges: 0,
        hint: "La llave esta en tus manos: esa pieza esta enlazada al techo.",
        trick: "ENTRELAZADO",
        dual: false,
        par: 2,
        sol: "",
    },
    Puzzle {
        name: "Reaccion en cadena",
        rows: &[
            "AA#####.##",
            "##a#..####",
            "####..####",
            "..########",
            "..########",
        ],
        pieces: &["Ot", "O"],
        goal: Goal::Lines(4),
        pending: None,
        charges: 0,
        hint: "Una cosa lleva a la otra.",
        trick: "TUNEL + ENTRELAZADO",
        dual: false,
        par: 2,
        sol: "",
    },
    Puzzle {
        name: "Gran final",
        rows: &[
            "AA#####.##",
            "##a#..####",
            "####..####",
            "..########",
            "..########",
        ],
        pieces: &["O", "Ot"],
        goal: Goal::Lines(4),
        pending: None,
        charges: 0,
        hint: "Guarda, atraviesa y deja que el enlace haga el resto.",
        trick: "GUARDAR + TUNEL + ENTRELAZADO",
        dual: false,
        par: 2,
        sol: "",
    },
];

// Banco de retos generados y comprobados (lo escribe herramientas/generador).
include!("puzzles_gen.rs");

/// Niveles del MUNDO LATERAL: el banco generado (puro, sin piezas cuanticas),
/// ordenado de facil a dificil. Los 8 de PUZZLES usan piezas cuanticas y quedan
/// solo como referencia para el verificador.
pub fn total() -> usize {
    BANK.len().max(1)
}

pub fn get(i: usize) -> &'static Puzzle {
    if BANK.is_empty() {
        &PUZZLES[2]
    } else {
        &BANK[i.min(BANK.len() - 1)]
    }
}

impl Goal {
    pub fn text(&self) -> String {
        match self {
            Goal::ClearAll => "Meta: deja el tablero VACIO".to_string(),
            Goal::Lines(n) => format!("Meta: borra {} lineas", n),
        }
    }
}
