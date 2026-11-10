# Verificación del MVP en un iPhone (029)

- Fecha: 2026-11-10.
- iPhone 17 Pro (iPhone18,1), iOS 27.0, perfil de desarrollo gratuito (Personal Team `22FWTVV94S`).
- App: `main` @ `6e57039`, instalada con el frontend embebido (`CLAUDE.md` › Commands › On an iPhone).
- Mac: cornercase 0.13.19, puente `cargo run -p cornercase-bridge` (LAN y Tailscale, un solo código de máquina, 035).
- Probó: el usuario, con la lista de abajo; las capturas que mandó están en la conversación de la sesión.

## Criterios de éxito (`docs/IDEA.md`)

| # | Criterio | Resultado | Notas |
|---|---|---|---|
| 1 | Tablero en vivo de todos los proyectos, los que esperan arriba | sí | Los subagentes salen plegados bajo su agente (038), pero el chevron de plegar parece un "ir": tarea 040 (con un buscador de agentes). |
| 2 | Contestar a un agente (texto, tecla, imagen) y verle seguir | sí | Texto con su burbuja enviando → enviado y la línea "working" (036); teclas con vibración (017); imagen adjunta llegada entera (018). La pregunta de `AskUserQuestion` sale en el chat (039) pero con scroll horizontal: tarea 042. Barra de escribir estilo Claude, teclas tras un icono, quitar una imagen y vaciar el campo al pulsar `Send`: tarea 041. |
| 3 | Terminal en vivo, escribir y desplazar sin cambiar el Mac; "ajustar" al ancho del móvil | parcial | Funciona; el usuario quiere la terminal ajustada por defecto y en modo ajustado el scroll va mal: tarea 043. |
| 4 | Arrancar un agente en un worktree desde una issue | parcial | Se arranca desde una issue (024). Crear worktrees desde el móvil no le sirve (luego no se usan) y el filtro de proyecto de las issues no filtra: tarea 045. Lanzar una issue con un flow: tarea 047. |
| 5 | Cambios y ficheros de un workspace, marcar vistos | sí | Pantallas de la 025 y la 026. |
| 6 | Volver de segundo plano y wifi → datos: todo se reconecta solo | sí | Con Tailscale en el iPhone (028 › CA4). Tiempo hasta `connected` no medido: "va todo bien". Con las dos rutas, en casa ganó Tailscale y no la LAN: tarea 046. |
| 7 | El formato del puente, propuesta de `/cli` | pendiente | Borradores de la épica y sus 13 sub-issues hechos (033); se crean tras esta prueba, con el repo público. |

## Otras comprobaciones en el dispositivo

- Firma e instalación con el perfil gratuito: sí. La primera vez hay que confiar en el certificado (Ajustes › General ›
  VPN y gestión de dispositivos).
- Identidad en el Keychain (009 › CA3): la app arranca y conecta sin avisos de identidad.
- Un solo código de máquina (035): conectó por la ruta de Tailscale (`via [fd7a:115c:a1e0::…]`).
- Gesto de volver desde el borde (037): funciona con el dedo; el R12 de su revisión (el ángulo del primer
  `touchmove`) no se notó.
- Vibración de las teclas (017): sí.
- Al abrir un agente `done` no se le quita el estado en el Mac ni en el tablero: tarea 044.

## Fallos y mejoras convertidos en tareas

040, 041, 042, 043, 044, 045, 046 y 047 (`docs/ROADMAP.md`).
