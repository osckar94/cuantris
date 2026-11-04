# CUANTRIS — bloques cuánticos para tu celular (Rust + macroquad)

## Cómo jugar
- **Botonera en pantalla (nuevo)**: ◀ ▶ mover (mantén para deslizar), ↺ ↻ girar, ▼ bajar (mantén),
  ⤓ caída total; debajo GUARDAR, OBSERVAR y PAUSA. En PAUSA → **BOTONES: SÍ/NO** para jugar solo con gestos.
- **Deslizar a los lados**: mover la pieza.
- **Tocar**: girar (mitad derecha = horario, mitad izquierda = antihorario).
- **Deslizar rápido hacia abajo**: caída total. Arrastre lento: bajar poco a poco.
- **Deslizar hacia arriba** o botón **GUARDAR**: guardar la pieza.
- **SUPERPOSICIÓN**: la pieza aparece en 2 lugares a la vez (50 % / 50 %). Al tocar fondo colapsa al azar en uno.
  El botón **OBSERVAR** gasta una carga y la colapsa en la posición que cae más profundo.
- **ENTRELAZAMIENTO**: piezas marcadas con un punto blanco se enlazan en pareja. Si borras una línea que toca
  una, la otra desaparece también, esté donde esté.
- **TÚNEL CUÁNTICO (nuevo)**: piezas con una flecha y un anillo verde agua. Al tocar fondo atraviesan hasta
  4 filas de bloques y reaparecen en el hueco de abajo (rellenan huecos tapados). Una guía verde punteada
  muestra dónde terminarán. La tercera pieza de cada partida siempre es TÚNEL.
- **ACERTIJOS — pensamiento lateral (nuevo)**: 8 retos con tablero armado, piezas contadas y una meta
  ("deja el tablero vacío" o "borra N líneas"). Sin gravedad: piensa con calma. Jugando "normal" son
  imposibles; cada uno exige un truco (túnel, entrelazado, guardar, observar…). Botón OPCIONES → PISTA.
  Todos están verificados por computadora: tienen solución y no se resuelven sin su truco.
- **RETOS LATERALES en cada partida (nuevo)**: cada 5–10 piezas aparece un reto de pensamiento lateral con
  una pregunta "¿Y si…?" y pocas piezas para lograrlo. Entrena 5 técnicas: CAMBIA EL ORDEN (guardar),
  CUESTIONA EL SUPUESTO (túnel), INVIERTE EL PROBLEMA (entrelazado), MIRA DISTINTO (observar) y USA LO
  INESPERADO (pieza boca abajo). El juego mete las piezas necesarias. Superarlo da puntos y una carga de
  OBSERVAR. Al terminar ves tu perfil por técnica y un consejo; los retos insisten en tu técnica más floja.
- Dificultad: FÁCIL (nivel 1), NORMAL (nivel 4), EXPERTO (nivel 8 + líneas de "ruido cuántico" que suben).
  La velocidad aumenta cada 10 líneas.

## Obtener el enlace de descarga (APK) — sin instalar nada en tu PC
1. Entra a https://github.com y crea una cuenta (gratis) si no tienes.
2. Arriba a la derecha: **+ → New repository**. Nombre: `cuantris`. Marca **Public** (necesario para que el
   enlace se descargue sin iniciar sesión). Pulsa **Create repository**.
3. En la página nueva pulsa **uploading an existing file**.
4. Desde esta carpeta arrastra: `Cargo.toml`, `.gitignore`, `LEEME.md`, `web_index.html`, la carpeta `src`
   y la carpeta `.github`. (No hace falta subir `CONTRATO.md`.) Pulsa **Commit changes**.
5. Una sola vez: **Settings → Pages → Source: GitHub Actions**.
6. Ve a la pestaña **Actions**: verás "Compilar APK Android y version Web" en marcha (unos 5–10 minutos).
   Si falló antes de activar Pages, pulsa **Re-run all jobs**.
7. Cuando termine en verde tienes dos enlaces (cambia `TU_USUARIO` por tu usuario de GitHub):

   **Recomendado: jugar sin instalar nada (Android y iPhone):**
   `https://TU_USUARIO.github.io/cuantris/`

   **Descargar APK (solo Android):**
   `https://github.com/TU_USUARIO/cuantris/releases/latest/download/cuantris.apk`

   Pega el enlace en WhatsApp. El de la versión web abre el juego directamente en el navegador.
   Para tenerlo como app: menú del navegador → **Agregar a la pantalla principal**.

## Instalar en el celular (Android)
1. Toca el enlace en WhatsApp → se descarga `cuantris.apk`.
2. Ábrelo. Si Android avisa de "origen desconocido", pulsa **Configuración → Permitir de esta fuente**.
3. Si Play Protect avisa, pulsa **Instalar de todas formas** (es una app propia, no está en la tienda).

> Solo Android. iPhone no permite instalar APK.

## Teclado (si lo pruebas en PC)
Flechas: mover / ↑ o X: girar / Z: girar al revés / Espacio: caída / C: guardar / E: observar / P: pausa.
