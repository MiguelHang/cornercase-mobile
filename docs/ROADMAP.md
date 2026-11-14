# Roadmap — cornercase-mobile

Fuente: `docs/IDEA.md`. Decisiones: `DESIGN.md` y `design/`. Cada tarea de `docs/tasks/` es una rama y una PR
(secuencial: cada una sale de `main` cuando la anterior está mergeada). Estado: `pendiente` · `en curso` · `hecha`.

Referencias (siempre del `main` de upstream; la copia local de cornercase-terminal va por detrás):
- `gh api repos/usecornercase/cornercase-gpui/contents/<ruta> -H "Accept: application/vnd.github.raw"`
- `git -C ~/me/cornercase-terminal fetch origin main && git -C ~/me/cornercase-terminal show origin/main:<ruta>`

Leyenda: ⚠ = tarea con riesgo (plan B en la propia tarea). Tamaño = líneas añadidas estimadas (+30 %), tests incluidos.

## Hito 0 — Base del repo
La 034 va aquí aunque es UI: no necesita red ni la 013, y deja la app con el aspecto de cornercase en el simulador
antes de empezar el puente. Su id no sigue el orden para no renumerar el roadmap.

| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 001 | [Scaffolding Tauri 2 + Svelte 5 + workspace Cargo + CI](tasks/001-scaffolding.md) | — | ~600 (+ ficheros generados) |
| 002 | [Guardarraíles: límites de clippy, 1.000 líneas, sin comentarios](tasks/002-guardrails.md) | 001 | ~250 |
| 034 | [UI look: tema de gpui, cristal, logo y pantalla "sin máquinas" (sin red, se ve en el simulador)](tasks/034-ui-look.md) | 002 | ~650 |

## Hito 1 — Puente de desarrollo (fase 0)
El puente se usa ya desde el portátil (`websocat`) antes de que exista la app.

| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 003 | [Puente: protocolo y lista permitida](tasks/003-bridge-protocol-allowlist.md) | 001 | ~450 |
| 004 | [Puente: ejecutar `cornercase` línea a línea y matar su grupo](tasks/004-bridge-process.md) | 003 | ~500 |
| 005 | [Puente: servidor WebSocket con token](tasks/005-bridge-websocket.md) | 004 | ~450 |
| 006 | [Puente: solo LAN/Tailscale, token, URL y QR](tasks/006-bridge-listen-qr.md) | 005 | ~400 |

## Hito 2 — Núcleo Rust de la app
| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 007 | [Núcleo: `Transport` y cliente del puente](tasks/007-core-transport.md) | 005 | ~500 |
| 008 | [Núcleo: máquinas, rutas y elección de ruta](tasks/008-core-machines-routes.md) | 007 | ~550 |
| 009 | [Identidad Ed25519 en el Keychain de iOS](tasks/009-identity-keychain.md) ⚠ | 001 | ~350 |
| 010 | [Núcleo: `contract` y `boot`](tasks/010-core-contract-boot.md) | 007 | ~400 |
| 011 | [Núcleo: cola ordenada (`requests --follow`, `input --follow`)](tasks/011-core-ordered-queue.md) | 007 | ~650 |
| 012 | [Núcleo: `Store` y streams que se reabren solos](tasks/012-core-store-streams.md) | 008, 010, 011 | ~700 |
| 013 | [App: comandos y eventos de Tauri con tipos (tauri-specta) y ciclo de vida](tasks/013-tauri-bindings-lifecycle.md) | 009, 012 | ~550 |

## Hito 3 — UI del MVP (iOS), en el orden de IDEA.md
| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 014 | [UI base: navegación, avisos, estado de la conexión, primera máquina pegando la URL](tasks/014-ui-base-theme.md) | 013, 034 | ~550 |
| 015 | [Pantalla 1 — Tablero de agentes](tasks/015-ui-agents-board.md) | 014 | ~550 |
| 016 | [Pantalla 2 — Chat: último mensaje y responder](tasks/016-ui-chat-reply.md) | 015 | ~500 |
| 017 | [Pantalla 2 — Chat: fila de teclas](tasks/017-ui-chat-keys.md) | 016 | ~300 |
| 018 | [Pantalla 2 — Chat: adjuntar imágenes con `paste-file`](tasks/018-ui-chat-paste-image.md) ⚠ | 017 (y decisión D1) | ~400 |
| 019 | [Terminal en el núcleo: frames, patches y `projection`](tasks/019-core-terminal-grid.md) | 012 | ~750 |
| 020 | [Pantalla 3 — Terminal pasiva: `view` + `surface --patches`, zoom y scroll horizontal](tasks/020-ui-terminal-passive.md) | 019, 014 | ~650 |
| 021 | [Pantalla 3 — Terminal: entrada, teclas que faltan, desplazar y teclado del WebView](tasks/021-ui-terminal-input-keyboard.md) ⚠ | 020 | ~600 |
| 022 | [Pantalla 3 — "Ajustar a este móvil" (`view-set --claim`)](tasks/022-ui-terminal-fit.md) | 021 | ~300 |
| 023 | [Pantalla 4 — Arrancar un agente (`start --worktree`, `new-workspace`)](tasks/023-ui-start-agent.md) | 015 | ~550 |
| 024 | [Pantalla 4 — Arrancar desde una issue (`issue list/read`, `start --issue`)](tasks/024-ui-start-from-issue.md) | 023 | ~550 |
| 025 | [Pantalla 5 — Cambios del workspace y marcarlos como vistos](tasks/025-ui-changes.md) | 015 | ~600 |
| 026 | [Pantalla 5 — Ficheros (`files list/read/search`)](tasks/026-ui-files.md) | 025 | ~550 |
| 027 | [Pantalla 6 — Máquinas: lista, QR, varias rutas](tasks/027-ui-machines-qr.md) | 014, 008 | ~550 |
| 028 | [Cambio de red en iOS (NWPathMonitor)](tasks/028-ios-network-change.md) ⚠ | 013 | ~300 |
| 035 | [Una máquina, un código: el puente da todas sus rutas y la app elige](tasks/035-one-code-per-machine.md) | 027, 028 | ~500 |
| 036 | [Chat: ver lo que has enviado y qué está haciendo el agente](tasks/036-chat-feedback.md) | 016, 017, 018 | ~400 |
| 037 | [Volver atrás con el gesto de iOS y un chevron en todas las pantallas](tasks/037-back-gesture.md) | 036 | ~350 |
| 038 | [Tablero: los subagentes, bajo su agente y plegados](tasks/038-board-subagents.md) | 015 | ~450 |
| 039 | [Chat: ver las preguntas y menús del agente sin ir a la terminal](tasks/039-chat-shows-questions.md) | 036, 020 | ~450 |
| 040 | [Tablero: plegado que no parezca un "ir" y buscador de agentes](tasks/040-board-fold-and-search.md) | 038 | ~350 |
| 041 | [Chat: barra de escribir estilo Claude/WhatsApp y quitar una imagen adjunta](tasks/041-chat-composer.md) | 036, 039 | ~400 |
| 042 | [Chat: la pregunta del agente se lee sin scroll horizontal](tasks/042-chat-question-layout.md) | 039 | ~1.550 |
| 043 | [Terminal: ajustada al móvil por defecto, y su scroll](tasks/043-terminal-fit-default.md) | 022 | ~250 |
| 044 | [Abrir un agente done lo da por visto](tasks/044-done-seen.md) | 016 | ~250 |
| 045 | [Arrancar: sin workspaces nuevos y las issues sin un filtro de proyecto que no filtra](tasks/045-start-simpler.md) | 023, 024 | ~1.500 |
| 046 | [Elegir la LAN cuando la LAN y Tailscale contestan](tasks/046-prefer-lan.md) | 035 | ~150 |
| 047 | [Arrancar una issue con un flow](tasks/047-start-issue-with-flow.md) | 024, 045 | ~350 |
| 048 | [El chat como conversación: historial en el móvil, y el de cornercase cuando exista](tasks/048-chat-history.md) | 036, 041 | ~450 |
| 049 | [El icono de la app](tasks/049-app-icon.md) | 030 | ~50 (+ generados) |
| 050 | [Partir por temas los ficheros de `design/` de más de 40 KB](tasks/050-split-design-by-topic.md) | 043 | ~0 (solo docs) |
| 051 | [Tablero: las filas aprovechan todo el ancho](tasks/051-board-full-width.md) | 040 | ~80 |
| 052 | [Empezar: elegir cuenta al arrancar un agente o un flow](tasks/052-start-account.md) | 045, 047 | ~250 |
| 053 | [Partir por temas `design/starting-work.md`](tasks/053-split-starting-work.md) | 052 | ~0 (solo docs) |
| 054 | [Tablero: el triangulito, entre el punto y el texto](tasks/054-board-triangle-offset.md) | 051 | ~20 |
| 055 | [Pantalla de carga al arrancar, sin parpadeo](tasks/055-launch-screen.md) | — | ~120 |
| 056 | [Cerrar un agente desde el teléfono](tasks/056-close-agent.md) | 051 | ~250 |
| 029 | [Verificación del MVP en un iPhone por Tailscale](tasks/029-mvp-check-iphone.md) | 015–028, 030–032 (se deja para el final; decidido el 2026-11-09) | ~150 (docs) |

## Hito 4 — Android
| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 030 | [Android: proyecto, arranque en emulador y dispositivo](tasks/030-android-project.md) | 015–028 | ~250 (+ generados) |
| 031 | [Android: identidad en el Keystore](tasks/031-android-keystore.md) ⚠ | 030, 009 | ~450 |
| 032 | [Android: teclado del WebView y cambio de red](tasks/032-android-keyboard-network.md) ⚠ | 030, 021, 028 | ~400 |

## Hito 5 — Upstream
| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 033 | [Propuesta de `/cli` para cornercase-terminal](tasks/033-cli-endpoint-proposal.md) | 029 (al final; issues en una épica de upstream, sin PR). El borrador 01 pasa a la 057; el 07 ya está en upstream (0.13.72) | — |

## Hito 6 — Primera versión (0.1.0), sin bridge
Decidido por el usuario el 2026-11-14: quitar el bridge con un QR para la app que enseña el propio cornercase (rutas de
LAN y Tailscale, como el del bridge) y un `/cli` en cornercase; 0.1.0 por TestFlight y APK en GitHub Releases.
Comprobado contra upstream `main` (cornercase 0.13.72): no había `/cli`, y el QR del navegador abre la versión web. La 057
está hecha: upstream lo publicó en cornercase 0.13.78 (#851, 2026-11-14), con el QR de **Phone app** en Settings ›
Remote, el canje por un secreto del dispositivo y `/cli`.

| # | Tarea | Depende de | Tamaño |
|---|---|---|---|
| 057 | [Upstream: el QR para emparejar la app y `/cli` en cornercase-terminal](tasks/057-upstream-pair-app-cli.md) (#838 de upstream, hecha en #851: cornercase 0.13.78) | — (repo de upstream) | ~800 en upstream |
| 058 | [La app habla con el `/cli` de cornercase y empareja con su QR](tasks/058-app-on-cornercase-cli.md) | 057 | ~450 |
| 059 | [Quitar el bridge](tasks/059-remove-bridge.md) | 058 | ~−1.500 |
| 060 | [Primera versión: 0.1.0 por TestFlight y APK](tasks/060-release-0.1.0.md) | 059 (la 057 ya está publicada) | ~300 (+ CI) |
| 061 | [Upstream: el servidor arrancado al iniciar sesión no encuentra los agentes](tasks/061-upstream-server-path.md) | — (issue) | — |

## Camino crítico
`001 → 003 → 004 → 005 → 007 → (008, 010, 011) → 012 → 013 → 014 → 015 → 016 → … → 029`.
Orden de trabajo del Hito 0 y el arranque del 1: `001 → 002 → 034 → 003`; la 034 solo pide 002 y la 014 la necesita.
Ramas laterales: `009` (tras 001, antes de 013) · `019` (tras 012, antes de 020) · `006` (tras 005, antes de 029)
· `028` (tras 013) · `035` (tras 027 y 028, antes de Android; pedida por el usuario el 2026-11-09) · `036` (tras 018, antes de Android; pedida por el usuario el 2026-11-09) · `037`, `038`, `039` (antes de Android; pedidas por el usuario el 2026-11-09) · `040`–`049` (de la prueba del usuario en su iPhone, 029, el 2026-11-10) · `050` (tras 043, de su review) · `051`, `052`, `054`, `055`, `056` (de la segunda prueba en el iPhone, 2026-11-12) · `053` (tras 052, de su review) · `030 → 031, 032` (tras 015–028; la 029, la prueba en el iPhone, va al final, justo antes de la 033) · `033` (al final, tras 029: borradores de issues en una épica de cornercase-terminal, sin PR).

## Qué puede ir en paralelo (si algún día hay más de un implementador)
- 002, 003 y 009 tras 001; 034 tras 002. 008, 010 y 011 tras 007. 019 en cuanto exista 012.
- Las pantallas 023–027 son independientes entre sí tras 015 (027 solo pide 014 y 008).

## Riesgos y dónde se tratan
| Riesgo | Tarea | Plan B |
|---|---|---|
| Keychain de iOS desde Rust (`keyring` en iOS, simulador sin entitlements) | 009 | plugin propio de Tauri en Swift (`SecItemAdd`) |
| Keystore de Android desde Rust | 031 | plugin de Tauri en Kotlin con `AndroidKeyStore` que cifra la semilla Ed25519 |
| Detectar cambios de red (NWPathMonitor / ConnectivityManager) | 028 (hecha; en un iPhone, la 029), 032 (hecha en el emulador; en un Android real, pendiente) | reintentar al volver a primer plano y al fallar un stream (ya en 013/012) |
| iOS suspende los sockets en segundo plano | 012, 013 | reabrir streams al volver, reconstruir con `boot` |
| Teclado y gestos del WebView (foco, teclado virtual sobre la página, swipe) | 021, 032 (en la 016, hecha, el campo del chat ya queda sobre el teclado en el simulador; en la 036, hecha, la cabecera fija del chat y la barra de conexión salen de la vista con el teclado abierto y no tapan el campo; falta el ajuste fino; en la 032, hecha, en Android la página encoge con el teclado gracias a los insets del IME en `MainActivity.kt`, comprobado en el emulador) | `visualViewport` + `interactive-widget=resizes-content`, como `browser-access.md` de cornercase-terminal |
| tauri-specta en versión RC para Tauri 2 | 013 | tipos TS escritos a mano + test de que coinciden con un JSON de ejemplo |
| Rendimiento de la terminal con patches frecuentes | 019, 020 | aplicar en Rust y emitir solo filas cambiadas; canvas si el DOM no llega |
| Versión de cornercase en el Mac | 010 | la comprobación de `contract` dice qué falta |
| Bytes binarios por el puente (`paste-file`) | 003, 018 | decisión D1 |
| Perfil de Apple gratuito (caduca cada 7 días) | 029 | anotado; no bloquea |

## Decisiones (tomadas por el usuario el 2026-11-08)
- **D1**: el puente acepta también `{"stdin_base64": "…"}` (bytes) y `{"eof": true}` (fin de stdin), cliente → puente.
- **D2**: la rejilla de la terminal se aplica en Rust (`crates/core`, port de `model/grid.rs` de gpui); al WebView
  solo llegan filas resueltas.
- **D3**: 014 trae un formulario mínimo "pegar URL"; 027 la pantalla completa (QR, varias rutas, editar).
- **D4**: `src-tauri/gen/apple` y `gen/android` se versionan.
- **D5**: la app comprueba operaciones y `features` de `contract`, no una versión mínima.
- **D6** (2026-11-08): la parte visual de la 014 (tema, cristal, logo, movimiento, pantalla "sin máquinas" estática)
  se adelanta a la 034, justo tras la 002, para ver la app en el simulador cuanto antes; la 014 se queda con
  navegación, avisos, estado de la conexión y el formulario de la primera máquina.
