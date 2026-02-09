# Whisperdict

## Vision y descripcion para usuarios

Whisperdict es una aplicacion de escritorio para Linux que transcribe audio del microfono y pega el texto automaticamente donde tengas el cursor. Se usa con un atajo global de teclado para iniciar y detener la grabacion. El objetivo es poder dictar texto en cualquier aplicacion (navegador, editor, chat, formularios) sin cambiar de ventana.

### Que hace

- Escucha el microfono y graba cuando activas el atajo global.
- Transcribe el audio localmente (sin nube, sin API keys).
- Pega el resultado en la aplicacion activa (donde esta el cursor).
- Permite elegir y descargar modelos de Whisper.

### Funciones principales

- Atajo global configurable (por defecto Ctrl+Alt+Space).
- Indicador en la bandeja (tray) con luz pulsante al grabar. Se pone en gris cuando no hace nada la app, pulsa en rojo si esta grabando y se queda en fijo naranja si esta transcribiendo.
- Selector de modelos locales (tiny/base/small/medium/large).
- Descarga de modelos con barra de progreso.
- Borrado de modelos y limpieza de parciales.
- Input de area de prueba dentro de la app.

### Interfaz

- Panel superior con estado (reposo/grabando/transcribiendo/error).
- Tarjeta de atajos para cambiar el shortcut.
- Tarjeta de modelos con lista, estado (indica si es el modelo en uso) y botones por modelo (usar/eliminar). Los botones de usar y eliminar solo salen si esta descargado. Si no esta descargado solo sale un boton para descargar.
- Tarjeta de prueba con textarea y botones Copiar/Limpiar.

### Historias de usuario

1) Como usuario quiero dictar texto en cualquier app con un atajo para no perder el foco.
2) Como usuario quiero ver si estoy grabando gracias a un indicador visible.
3) Como usuario quiero elegir un modelo mas rapido o mas preciso.
4) Como usuario quiero descargar y eliminar modelos para ahorrar espacio.
5) Como usuario quiero que cuando descargo un modelo se ponga en uso automaticamente.
6) Como usuario quiero comprobar la transcripcion dentro de la app.

## Diagramas y mapas de flujo

### Flujo principal (dictado)

```text
Usuario pulsa atajo
  -> Whisperdict inicia grabacion
  -> Tray cambia a modo grabando
Usuario pulsa atajo
  -> Whisperdict detiene grabacion
  -> Whisperdict transcribe (local)
  -> Whisperdict pega en el foco actual
```

### Flujo de descarga de modelos

```text
Usuario pulsa "Descargar" en un modelo
  -> Whisperdict inicia descarga
  -> UI muestra progreso
  -> Archivo .part mientras descarga
  -> Si finaliza: renombra a .bin, marca instalado y lo usa
  -> Si falla: elimina .part y muestra error
```

### Flujo de transcripcion en proceso hijo

```text
App (Rust) -> escribe WAV temporal
App (Rust) -> envia ruta WAV al proceso hijo
Child -> transcribe con Whisper
Child -> devuelve texto por stdout
App -> pega texto y actualiza UI
```

### Flujo de auto-deteccion de idioma (scoring)

```text
Tomar 2s de audio
  -> probar idiomas candidatos (es/en/pt/fr/de/it)
  -> calcular probabilidad media de tokens
  -> elegir el mejor idioma
  -> transcribir todo con ese idioma
  -> si sale vacio, fallback a `es`
```

### Flujos clave

- Dictado normal: pulso atajo -> grabacion -> pulso atajo -> transcripcion -> pegado.
- Modelos: elijo uno y lo uso, descargo otros si necesito mas precision.

## Implementacion tecnica (detalle)

### Arquitectura general

- Frontend: react, tailwind, shadcn
- Backend: Rust (Tauri) con captura de audio, transcripcion, pegado y hotkeys.
- Transcripcion: Whisper local via whisper-rs (whisper.cpp embebido).
- Modelos: archivos GGML descargados desde HuggingFace.

### Estructura de archivos

- Modelos: `~/.local/share/Whisperdict/models`.

### Transcripcion (Whisper)

- Motor: `whisper-rs` (whisper.cpp integrado).
- La transcripcion se ejecuta en un **proceso hijo persistente**:
  - Evita que un crash de whisper tumbe la app.
  - Carga el modelo una sola vez y reutiliza el contexto.
- Flujo:
  1) El backend escribe WAV temporal 16kHz/mono.
  2) Envia la ruta al proceso hijo por stdin.
  3) El hijo transcribe y devuelve texto por stdout.
  4) Se elimina el WAV temporal.

### Pegado automatico

- Portapapeles: `arboard`.
- Inyeccion de teclas:
  - X11: `enigo` (Ctrl+V).
  - Wayland: `wtype` si esta disponible.
- Fallback: si no se puede inyectar teclas, queda copiado en clipboard.

### Integraciones del sistema

- X11: `rdev` (hotkeys) + `enigo` (Ctrl+V).
- Wayland: portal `GlobalShortcuts` + `wtype` para pegar.
- Tray: Tauri v2 `TrayIcon`.

### Hotkeys globales

- X11: `rdev` para escuchar teclas globales.
- Wayland: `xdg-desktop-portal` via `ashpd` (GlobalShortcuts).
- Mismo atajo para iniciar/detener grabacion (toggle).

### Tray e indicador visual

- TrayIcon con icono generado en memoria.
- Estado:
  - Idle: gris.
  - Recording: pulsante.
  - Processing: naranja.
- Animacion por timer (actualiza el icono cada 520 ms).

### Modelos y descargas

- Modelos GGML: tiny/base/small/medium/large.
- Descarga con `reqwest` y tiempo maximo configurado.
- Archivos parciales `*.part` detectados y marcados como "Incompleto".
- Validacion por tamano minimo (min_bytes).
- Eventos de progreso hacia UI: `models:progress`.

#### Estados de modelos

- En uso: modelo activo para las transcripciones.

### Configuracion y persistencia

- Archivo JSON en `~/.config/Whisperdict/config.json`.
- Guarda shortcut y modelo activo/preferido.

### UI y estados

- UI actualiza segun eventos y respuestas de comandos Tauri.
- Durante descarga: se eliminan los botones, barra de progreso.
- Texto de transcripcion se pega en el foco actual pudiendo ser el textarea.

### Tests E2E

- Playwright en modo mock (`VITE_E2E=1`).
- Tests cubren:
  - Lista de modelos.
  - Descarga e instalacion.
  - Eliminacion.
  - Cambio de shortcut.
  - Grabacion y transcripcion (con audio real de pruebas).
- Archivos:
  - `playwright.config.js`
  - `tests/e2e/*.spec.js`

#### Alcance E2E

- Flujos completos de UI (lista, descargar, borrar, cambiar atajo).
- Transcripcion simulada con el audio real de pruebas.
