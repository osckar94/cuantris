# CUANTRIS — bloques cuánticos para tu celular (Rust + macroquad)

## Cómo jugar
- **Deslizar a los lados**: mover la pieza.
- **Tocar**: girar (mitad derecha = horario, mitad izquierda = antihorario).
- **Deslizar rápido hacia abajo**: caída total. Arrastre lento: bajar poco a poco.
- **Deslizar hacia arriba** o botón **GUARDAR**: guardar la pieza.
- **SUPERPOSICIÓN**: la pieza aparece en 2 lugares a la vez (50 % / 50 %). Al tocar fondo colapsa al azar en uno.
  El botón **OBSERVAR** gasta una carga y la colapsa en la posición que cae más profundo.
- **ENTRELAZAMIENTO**: piezas marcadas con un punto blanco se enlazan en pareja. Si borras una línea que toca
  una, la otra desaparece también, esté donde esté.
- Dificultad: FÁCIL (nivel 1), NORMAL (nivel 4), EXPERTO (nivel 8 + líneas de "ruido cuántico" que suben).
  La velocidad aumenta cada 10 líneas.

## Obtener el enlace de descarga (APK) — sin instalar nada en tu PC
1. Entra a https://github.com y crea una cuenta (gratis) si no tienes.
2. Arriba a la derecha: **+ → New repository**. Nombre: `cuantris`. Marca **Public** (necesario para que el
   enlace se descargue sin iniciar sesión). Pulsa **Create repository**.
3. En la página nueva pulsa **uploading an existing file**.
4. Desde esta carpeta arrastra: `Cargo.toml`, `.gitignore`, `LEEME.md`, la carpeta `src` y la carpeta `.github`.
   (No hace falta subir `CONTRATO.md`.) Pulsa **Commit changes**.
5. Ve a la pestaña **Actions**: verás "Compilar APK Android" en marcha (unos 5–10 minutos).
6. Cuando termine en verde, tu enlace fijo será:

   `https://github.com/TU_USUARIO/cuantris/releases/latest/download/cuantris.apk`

   (cambia `TU_USUARIO` por tu usuario de GitHub). Ese enlace lo pegas en WhatsApp.

## Instalar en el celular (Android)
1. Toca el enlace en WhatsApp → se descarga `cuantris.apk`.
2. Ábrelo. Si Android avisa de "origen desconocido", pulsa **Configuración → Permitir de esta fuente**.
3. Si Play Protect avisa, pulsa **Instalar de todas formas** (es una app propia, no está en la tienda).

> Solo Android. iPhone no permite instalar APK.

## Teclado (si lo pruebas en PC)
Flechas: mover / ↑ o X: girar / Z: girar al revés / Espacio: caída / C: guardar / E: observar / P: pausa.
