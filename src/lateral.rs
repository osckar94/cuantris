//! Entrenamiento de PENSAMIENTO LATERAL dentro de cada partida.
//!
//! Cada pocas piezas aparece un reto ligado a una tecnica de pensamiento
//! lateral (Edward de Bono): cambiar el orden, cuestionar supuestos, invertir
//! el problema, mirar distinto y usar lo inesperado. El juego mete las piezas
//! necesarias para que el reto siempre sea posible.

use macroquad::rand::gen_range;

#[derive(Clone, Copy, PartialEq)]
pub enum Tech {
    Orden,
    Supuesto,
    Invertir,
    Mirar,
    Inesperado,
    Lado,
}

pub const ALL: [Tech; 6] = [Tech::Orden, Tech::Supuesto, Tech::Invertir, Tech::Mirar, Tech::Inesperado, Tech::Lado];

impl Tech {
    pub fn idx(self) -> usize {
        match self {
            Tech::Orden => 0,
            Tech::Supuesto => 1,
            Tech::Invertir => 2,
            Tech::Mirar => 3,
            Tech::Inesperado => 4,
            Tech::Lado => 5,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Tech::Orden => "CAMBIA EL ORDEN",
            Tech::Supuesto => "CUESTIONA EL SUPUESTO",
            Tech::Invertir => "INVIERTE EL PROBLEMA",
            Tech::Mirar => "MIRA DISTINTO",
            Tech::Inesperado => "USA LO INESPERADO",
            Tech::Lado => "ENTRA DE LADO",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Tech::Orden => "ORDEN",
            Tech::Supuesto => "SUPUESTO",
            Tech::Invertir => "INVERTIR",
            Tech::Mirar => "MIRAR",
            Tech::Inesperado => "INESPERADO",
            Tech::Lado => "DE LADO",
        }
    }
    /// La pregunta "Y si...?" que dispara el pensamiento lateral.
    pub fn question(self) -> &'static str {
        match self {
            Tech::Orden => "Y si guardas una pieza y la usas despues?",
            Tech::Supuesto => "Y si un hueco tapado se llena por dentro?",
            Tech::Invertir => "Y si borras lejos completando cerca?",
            Tech::Mirar => "Y si mirar cambia el resultado?",
            Tech::Inesperado => "Y si la pieza va al reves?",
            Tech::Lado => "Y si no entras desde arriba?",
        }
    }
    /// Que hay que lograr para superar el reto.
    pub fn goal(self) -> &'static str {
        match self {
            Tech::Orden => "Borra una linea con una pieza GUARDADA",
            Tech::Supuesto => "Haz que una pieza TUNEL atraviese bloques",
            Tech::Invertir => "Haz desaparecer bloques ENTRELAZADOS",
            Tech::Mirar => "OBSERVA la superposicion y borra linea con ella",
            Tech::Inesperado => "Borra una linea con una T, J o L boca abajo",
            Tech::Lado => "Mete una pieza BAJO un saliente, deslizandola",
        }
    }
    /// Leccion que se muestra al fallar / en el resumen.
    pub fn lesson(self) -> &'static str {
        match self {
            Tech::Orden => "Lo primero no siempre va primero: guardar no es perder tiempo.",
            Tech::Supuesto => "Crea tu mismo el techo: el tunel convierte un error en ventaja.",
            Tech::Invertir => "Para cambiar algo lejos, actua cerca: el enlace hace el resto.",
            Tech::Mirar => "Observar elige la copia que cae mas hondo: mira antes de soltar.",
            Tech::Inesperado => "Gira dos veces: la forma 'incorrecta' a veces encaja mejor.",
            Tech::Lado => "El camino recto no es el unico: a veces se llega por un costado.",
        }
    }
    pub fn pieces(self) -> u32 {
        match self {
            Tech::Orden => 6,
            Tech::Supuesto => 6,
            Tech::Invertir => 7,
            Tech::Mirar => 3,
            Tech::Inesperado => 7,
            Tech::Lado => 7,
        }
    }
}

/// Como cambia el entrenamiento segun el nivel elegido.
pub struct Level {
    /// Piezas extra (o menos) para cada reto
    pub extra: i32,
    /// Rango de piezas entre retos
    pub every: (u32, u32),
    /// 0 = pista siempre, 1 = pista a mitad del reto, 2 = nunca
    pub hint: u8,
    pub label: &'static str,
    /// Velocidad del tiempo dentro del modo lateral (1 = normal)
    pub slow: f32,
}

pub fn level(mode: crate::game::Mode) -> Level {
    use crate::game::Mode;
    match mode {
        Mode::Facil => Level { extra: 2, every: (5, 7), hint: 0, label: "Retos frecuentes, con pista y piezas de sobra", slow: 0.5 },
        Mode::Normal => Level { extra: 0, every: (6, 9), hint: 1, label: "La pista aparece a mitad del reto", slow: 0.65 },
        Mode::Experto => Level { extra: -1, every: (4, 7), hint: 2, label: "Retos seguidos, pocas piezas y SIN pistas", slow: 0.8 },
    }
}

impl Tech {
    /// Pista concreta: que herramienta usar (sin dar la jugada exacta).
    pub fn hint(self) -> &'static str {
        match self {
            Tech::Orden => "Pista: pulsa GUARDAR y juega la siguiente primero",
            Tech::Supuesto => "Pista: deja un hueco tapado y suelta encima la pieza con flecha",
            Tech::Invertir => "Pista: pon las piezas con punto en lineas que puedas completar",
            Tech::Mirar => "Pista: pulsa OBSERVAR antes de soltar la pieza doble",
            Tech::Inesperado => "Pista: gira 2 veces una T, J o L y encajala",
            Tech::Lado => "Pista: baja junto al saliente y muevela de lado antes de fijarla",
        }
    }
    /// Que boton conviene iluminar como pista (indice en la fila de acciones o botonera).
    pub fn key_button(self) -> Option<KeyBtn> {
        match self {
            Tech::Orden => Some(KeyBtn::Hold),
            Tech::Mirar => Some(KeyBtn::Observe),
            Tech::Inesperado => Some(KeyBtn::Rotate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum KeyBtn {
    Hold,
    Observe,
    Rotate,
}

/// Segundos que dura el anuncio grande de un reto (el juego espera).
pub const ANNOUNCE: f32 = 8.0;

pub struct Challenge {
    pub tech: Tech,
    pub left: u32,
    pub total: u32,
    pub t: f32,
    /// Mostrando el anuncio grande (el juego espera)
    pub announcing: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Score {
    pub ok: u32,
    pub tried: u32,
}

/// Estadisticas de entrenamiento (de la partida o de toda la sesion).
#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub s: [Score; 6],
}

impl Stats {
    pub fn total(&self) -> (u32, u32) {
        self.s.iter().fold((0, 0), |(a, b), x| (a + x.ok, b + x.tried))
    }
    pub fn add(&mut self, t: Tech, ok: bool) {
        let e = &mut self.s[t.idx()];
        e.tried += 1;
        if ok {
            e.ok += 1;
        }
    }
    /// Tecnica mas floja (menor % de exito; las no probadas cuentan como flojas).
    pub fn weakest(&self) -> Tech {
        let mut best = ALL[0];
        let mut best_v = f32::MAX;
        for t in ALL.iter() {
            let e = self.s[t.idx()];
            let v = if e.tried == 0 { -1.0 } else { e.ok as f32 / e.tried as f32 };
            if v < best_v {
                best_v = v;
                best = *t;
            }
        }
        best
    }
    /// Tecnica a practicar segun lo ya intentado (menor % de exito entre las probadas).
    pub fn weakest_tried(&self) -> Tech {
        let mut best = self.weakest();
        let mut best_v = f32::MAX;
        for t in ALL.iter() {
            let e = self.s[t.idx()];
            if e.tried == 0 {
                continue;
            }
            let v = e.ok as f32 / e.tried as f32;
            if v < best_v {
                best_v = v;
                best = *t;
            }
        }
        best
    }

    /// Indice lateral 0..100 combinando exito y variedad de tecnicas.
    pub fn index(&self) -> u32 {
        let (ok, tried) = self.total();
        if tried == 0 {
            return 0;
        }
        let rate = ok as f32 / tried as f32;
        let variety = self.s.iter().filter(|x| x.ok > 0).count() as f32 / 6.0;
        ((rate * 0.6 + variety * 0.4) * 100.0).round() as u32
    }
}

/// Elige el proximo reto: mas probable la tecnica mas floja de la sesion,
/// sin repetir la anterior.
pub fn pick(session: &Stats, last: Option<Tech>) -> Tech {
    let weak = session.weakest();
    if Some(weak) != last && gen_range(0.0f32, 1.0f32) < 0.5 {
        return weak;
    }
    loop {
        let t = ALL[(gen_range(0.0f32, 6.0f32) as usize).min(5)];
        if Some(t) != last {
            return t;
        }
    }
}
