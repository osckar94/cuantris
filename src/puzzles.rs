//! Modo ACERTIJOS: tableros armados que exigen pensamiento lateral.
//!
//! Todos fueron verificados con un simulador de las mismas reglas del juego:
//! tienen solucion y NO se pueden resolver jugando "normal" (sin el truco).
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
    pub name: &'static str,
    pub rows: &'static [&'static str],
    pub pieces: &'static [&'static str],
    pub goal: Goal,
    /// Grupo entrelazado cuyo gemelo sera la proxima pieza entrelazada.
    pub pending: Option<char>,
    pub charges: u32,
    pub hint: &'static str,
    pub trick: &'static str,
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
    },
];

impl Goal {
    pub fn text(&self) -> String {
        match self {
            Goal::ClearAll => "Meta: deja el tablero VACIO".to_string(),
            Goal::Lines(n) => format!("Meta: borra {} lineas", n),
        }
    }
}
