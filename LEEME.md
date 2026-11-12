# CUANTRIS — bloques cuánticos para tu celular (Rust + macroquad)

## Los dos mundos (versión actual)
El juego tiene dos mundos puros. Se elige uno en el menú.

**MUNDO CUÁNTICO** — las piezas caen y cada nivel abre un concepto. Subes de nivel borrando líneas
(la barra de arriba dice cuántas faltan). La primera vez que aparece cada concepto, el juego se detiene
y lo explica con un dibujo.
1. SUPERPOSICIÓN: la pieza está en 2 lugares; OBSERVAR elige la copia que cae más hondo.
2. EFECTO TÚNEL: la pieza con flecha atraviesa bloques.
3. ENTRELAZAMIENTO: piezas gemelas; borras una y desaparece la otra.
4. DIMENSIONES: el pozo existe en dos dimensiones (A y B). Juegas en una y la otra se ve como holograma.
   Los hologramas no chocan con tu pieza, pero una fila se borra si entre las dos dimensiones la llenan.
   El botón DIMENSIÓN (o la tecla D) salta de una a otra: un hueco tapado aquí está abierto allá.
5. TERCERA DIMENSIÓN: tres pozos (A azul, B rosa, C dorada); los hologramas de las otras dos se suman para completar filas.
6. SIMETRÍA: una línea central parte el pozo y tu pieza tiene un reflejo. Una fila simétrica
   (mitad izquierda igual a la derecha) con 6 o más bloques se borra aunque no esté llena.
7. CUARTA DIMENSIÓN: cuatro pozos (se añade la D verde).
8. SUPERCUÁNTICO: todo junto, nivel tras nivel.

**JEV (IA)** — en el menú hay dos botones JEV: uno juega el Mundo Cuántico y el otro resuelve los
niveles del Mundo Lateral desde el 1, mientras tú miras. Jev usa las mismas reglas y botones que un
jugador (en el lateral repite la solución que comprobó el generador). PAUSA para salir.

**MUNDO LATERAL** — 100 niveles sin piezas cuánticas, comprobados por el generador. Reglas:
sin reloj (nada cae solo), piezas contadas, una meta escrita sobre el tablero, y lo obvio no basta:
hay que guardar, girar o entrar de lado bajo un saliente. Menos piezas = 3 estrellas. Cada nivel
resuelto abre el siguiente. El botón PISTA da una pista.

Los niveles laterales se regeneran con `cargo run --release -- generar 100 10 150 puro` en
`herramientas/generador`. El progreso no se guarda al cerrar el juego.

Las secciones de abajo describen versiones anteriores (modos FÁCIL/NORMAL/EXPERTO y acertijos con
piezas cuánticas), que ya no están en el menú.

## Cómo jugar
- **¿Qué son las ondas?** Onda entre DOS copias de una pieza = SUPERPOSICIÓN (está en 2 sitios; pulsa
  OBSERVAR para quedarte con la que cae más hondo). Onda violeta entre bloques con punto = ENTRELAZADAS
  (borra una línea con una y desaparece su gemela). Flecha con anillos = TÚNEL (atraviesa bloques sola).
  La primera vez que aparece cada una, el juego se detiene y lo explica con un dibujo.
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
- **ACERTIJO AL AZAR (nuevo)**: un banco de retos generados por un programa que juega con las mismas
  reglas del juego (`herramientas/generador`). Cada reto se comprobó con una búsqueda completa: se sabe si
  tiene solución, cuántas piezas pide y qué truco exige. Salen primero los que aún no resolviste.
- **DOS SOLUCIONES (nuevo)**: algunos retos se pueden resolver "a lo obvio" usando todas las piezas
  (★ una estrella), pero con el truco bastan menos piezas (★★★ tres estrellas = solución lateral). El
  reto avisa cuántas piezas pide la solución lateral. Si te quedas con una estrella, puedes
  "BUSCAR LA LATERAL" y reintentar. Los retos "solo lateral" (sin truco no hay salida) dan siempre ★★★.
- Para regenerar el banco: `cd herramientas/generador` y
  `cargo run --release -- generar 120 12 300` (cuántos, hilos, segundos). Escribe `src/puzzles_gen.rs`.
  `cargo run --release -- validar` revisa todos los retos con la búsqueda completa.
- **Los 3 niveles entrenan pensamiento lateral**: al empezar, una tarjeta explica cuándo y cómo. Cada reto
  detiene el juego con un anuncio grande. FÁCIL = retos frecuentes, +2 piezas y pista (se ilumina el botón
  clave); NORMAL = pista a mitad del reto; EXPERTO = retos seguidos, menos piezas y sin pistas.
- **MODO VERTICAL y MODO LATERAL**: juegas en MODO VERTICAL (frío, todo cae) y un medidor arriba avisa
  cuántas piezas faltan. Cada 4–9 piezas saltas al MODO LATERAL: una ola cruza la pantalla, el mundo gira
  90° y fluye de lado, todo pasa a oro y coral, cambia la música, el tiempo va más lento (FÁCIL x0.50,
  NORMAL x0.65, EXPERTO x0.80) y el tablero muestra ecos "?" con otras opciones de caída.
- **RETOS LATERALES**: cada salto trae una pregunta "¿Y si…?" y pocas piezas para lograrlo. Entrena 6
  técnicas: CAMBIA EL ORDEN (guardar), CUESTIONA EL SUPUESTO (túnel), INVIERTE EL PROBLEMA (entrelazado),
  MIRA DISTINTO (observar), USA LO INESPERADO (pieza boca abajo) y ENTRA DE LADO (deslizar una pieza bajo
  un saliente). Superarlo congela el juego con "PENSASTE DE LADO", la lección, puntos y una carga de
  OBSERVAR. Al terminar ves tu perfil por técnica y un consejo; los retos insisten en tu técnica más floja.
  Los ACERTIJOS ocurren siempre en MODO LATERAL.
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
