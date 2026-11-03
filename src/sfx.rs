// Efectos de sonido y musica sintetizados en codigo (no hacen falta archivos).
use macroquad::audio::{load_sound_from_bytes, play_sound, stop_sound, PlaySoundParams, Sound};
use std::f32::consts::TAU;

const SR: f32 = 22050.0;

#[derive(Clone, Copy)]
pub enum S {
    Move,
    Rotate,
    Lock,
    Drop,
    Clear,
    Tetra,
    Collapse,
    Entangle,
    Hold,
    Level,
    Over,
    Click,
}

pub struct Sfx {
    pub on: bool,
    sounds: Vec<Option<Sound>>,
    music: Option<Sound>,
    music_on: bool,
}

impl Sfx {
    pub async fn load() -> Sfx {
        let defs: Vec<Vec<f32>> = vec![
            snd_move(),
            snd_rotate(),
            snd_lock(),
            snd_drop(),
            snd_clear(),
            snd_tetra(),
            snd_collapse(),
            snd_entangle(),
            snd_hold(),
            snd_level(),
            snd_over(),
            snd_click(),
        ];
        let mut sounds = Vec::new();
        for d in defs.iter() {
            sounds.push(load_sound_from_bytes(&wav(d)).await.ok());
        }
        let music = load_sound_from_bytes(&wav(&music())).await.ok();
        Sfx {
            on: true,
            sounds,
            music,
            music_on: false,
        }
    }

    pub fn play(&self, s: S, volume: f32) {
        if !self.on {
            return;
        }
        if let Some(Some(snd)) = self.sounds.get(s as usize) {
            play_sound(
                snd,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }

    pub fn music(&mut self, want: bool) {
        let want = want && self.on;
        if want == self.music_on {
            return;
        }
        if let Some(m) = &self.music {
            if want {
                play_sound(
                    m,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.35,
                    },
                );
            } else {
                stop_sound(m);
            }
        }
        self.music_on = want;
    }
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(SR as u32).to_le_bytes());
    v.extend_from_slice(&(SR as u32 * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let x = (s.max(-1.0).min(1.0) * 30000.0) as i16;
        v.extend_from_slice(&x.to_le_bytes());
    }
    v
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Square,
    Tri,
    Saw,
    Noise,
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / 16777216.0 * 2.0 - 1.0
    }
}

#[allow(clippy::too_many_arguments)]
fn tone(
    out: &mut Vec<f32>,
    start: f32,
    dur: f32,
    f0: f32,
    f1: f32,
    wave: Wave,
    vol: f32,
    attack: f32,
    decay: f32,
) {
    let s0 = (start * SR) as usize;
    let n = (dur * SR) as usize;
    if out.len() < s0 + n {
        out.resize(s0 + n, 0.0);
    }
    let mut ph = 0.0f32;
    let mut rng = Lcg(12345 + s0 as u32);
    for i in 0..n {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        let f = f0 + (f1 - f0) * k;
        ph += f / SR;
        ph -= ph.floor();
        let v = match wave {
            Wave::Sine => (ph * TAU).sin(),
            Wave::Square => {
                if ph < 0.5 {
                    0.6
                } else {
                    -0.6
                }
            }
            Wave::Tri => 1.0 - 4.0 * (ph - 0.5).abs(),
            Wave::Saw => (2.0 * ph - 1.0) * 0.7,
            Wave::Noise => rng.next(),
        };
        let env = if t < attack {
            t / attack
        } else {
            (-(t - attack) * decay).exp()
        };
        let tail = ((1.0 - k) * 30.0).min(1.0);
        out[s0 + i] += v * env * vol * tail;
    }
}

fn snd_move() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.035, 1250.0, 1100.0, Wave::Square, 0.2, 0.002, 40.0);
    o
}

fn snd_rotate() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.07, 700.0, 1350.0, Wave::Tri, 0.4, 0.003, 25.0);
    o
}

fn snd_lock() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.12, 190.0, 90.0, Wave::Sine, 0.7, 0.002, 30.0);
    tone(&mut o, 0.0, 0.05, 0.0, 0.0, Wave::Noise, 0.15, 0.001, 60.0);
    o
}

fn snd_drop() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.2, 130.0, 45.0, Wave::Sine, 0.9, 0.001, 16.0);
    tone(&mut o, 0.0, 0.1, 0.0, 0.0, Wave::Noise, 0.3, 0.001, 35.0);
    o
}

fn snd_clear() -> Vec<f32> {
    let mut o = Vec::new();
    for (i, f) in [523.25f32, 659.25, 783.99, 1046.5].iter().enumerate() {
        let st = i as f32 * 0.055;
        tone(&mut o, st, 0.2, *f, *f, Wave::Tri, 0.35, 0.003, 14.0);
        tone(&mut o, st, 0.2, f * 2.0, f * 2.0, Wave::Sine, 0.12, 0.003, 18.0);
    }
    o
}

fn snd_tetra() -> Vec<f32> {
    let mut o = Vec::new();
    let notes = [523.25f32, 659.25, 783.99, 1046.5, 1318.5, 1568.0];
    for (i, f) in notes.iter().enumerate() {
        tone(&mut o, i as f32 * 0.06, 0.2, *f, *f, Wave::Tri, 0.3, 0.003, 12.0);
    }
    for f in [523.25f32, 659.25, 783.99, 1046.5].iter() {
        tone(&mut o, 0.36, 0.6, *f, *f, Wave::Saw, 0.12, 0.01, 4.0);
    }
    o
}

fn snd_collapse() -> Vec<f32> {
    let n = (0.32 * SR) as usize;
    let mut o = vec![0.0; n];
    let mut ph = 0.0f32;
    let mut rng = Lcg(999);
    for (i, s) in o.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        let f = 900.0 - 650.0 * k + 160.0 * (t * 55.0).sin();
        ph += f / SR;
        ph -= ph.floor();
        let env = (-t * 9.0).exp() * ((1.0 - k) * 30.0).min(1.0);
        *s = ((ph * TAU).sin() * 0.5 + rng.next() * 0.12 * (1.0 - k)) * env;
    }
    o
}

fn snd_entangle() -> Vec<f32> {
    let n = (0.45 * SR) as usize;
    let mut o = vec![0.0; n];
    for (i, s) in o.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = i as f32 / n as f32;
        let trem = 0.6 + 0.4 * (t * 38.0 * TAU).sin();
        let v = (t * 1568.0 * TAU).sin() + (t * 2093.0 * TAU).sin() * 0.7 + (t * 2637.0 * TAU).sin() * 0.4;
        let env = (t / 0.01).min(1.0) * (-t * 6.0).exp() * ((1.0 - k) * 30.0).min(1.0);
        *s = v * trem * env * 0.22;
    }
    o
}

fn snd_hold() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.09, 520.0, 340.0, Wave::Sine, 0.5, 0.003, 20.0);
    tone(&mut o, 0.05, 0.09, 680.0, 460.0, Wave::Sine, 0.3, 0.003, 20.0);
    o
}

fn snd_level() -> Vec<f32> {
    let mut o = Vec::new();
    for (i, f) in [392.0f32, 523.25, 659.25, 783.99, 1046.5].iter().enumerate() {
        tone(&mut o, i as f32 * 0.07, 0.3, *f, *f, Wave::Saw, 0.2, 0.004, 8.0);
    }
    o
}

fn snd_over() -> Vec<f32> {
    let mut o = Vec::new();
    for (i, f) in [392.0f32, 329.6, 261.6, 196.0].iter().enumerate() {
        tone(&mut o, i as f32 * 0.2, 0.4, *f, *f * 0.97, Wave::Saw, 0.28, 0.005, 4.0);
    }
    tone(&mut o, 0.8, 0.8, 140.0, 40.0, Wave::Sine, 0.6, 0.01, 3.0);
    o
}

fn snd_click() -> Vec<f32> {
    let mut o = Vec::new();
    tone(&mut o, 0.0, 0.04, 950.0, 900.0, Wave::Sine, 0.4, 0.002, 40.0);
    o
}

/// Bucle ambiental de 8 segundos: La menor - Fa - Do - Sol.
fn music() -> Vec<f32> {
    let len = (8.0 * SR) as usize;
    let mut o = vec![0.0; len];
    let chords: [[f32; 3]; 4] = [
        [220.0, 261.63, 329.63],
        [174.61, 220.0, 261.63],
        [261.63, 329.63, 392.0],
        [196.0, 246.94, 293.66],
    ];
    for (ci, ch) in chords.iter().enumerate() {
        let st = ci as f32 * 2.0;
        for f in ch.iter() {
            tone(&mut o, st, 1.98, *f, *f, Wave::Sine, 0.06, 0.35, 0.5);
        }
        for b in 0..4 {
            let r = ch[0] / 2.0;
            tone(&mut o, st + b as f32 * 0.5, 0.48, r, r, Wave::Tri, 0.14, 0.01, 4.0);
        }
        for s in 0..8 {
            let f = ch[s % 3] * 2.0 * if s >= 6 { 2.0 } else { 1.0 };
            tone(&mut o, st + s as f32 * 0.25, 0.24, f, f, Wave::Sine, 0.06, 0.004, 9.0);
        }
    }
    o.truncate(len);
    o
}
