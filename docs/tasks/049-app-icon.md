# 049 — El icono de la app

- Estado: hecha (ver el icono en el iPhone del usuario queda pendiente)
- Depende de: 030
- Rama: `chore/049-app-icon` · PR: `chore: give the app cornercase's mark as its icon`

## Objetivo
Del usuario (2026-11-10): "molaría poner el icono de la app". Hoy la app lleva los iconos de ejemplo de Tauri.

## Criterios de aceptación
- [x] CA1 — El icono es la marca de cornercase: la misma geometría que su favicon (cornercase-terminal
  `site/public/favicon.svg`: la esquina en `#005fff` y el cursor en `#eceef1` sobre `#0c0e11`) y que el `Mark` de la
  app (`src/lib/ui/Mark.svelte`), redibujada como SVG fuente de 1024×1024 en `src-tauri/icons/` (en git), con el
  margen que piden las guías de iOS y Android para que no la corten las esquinas redondeadas ni la máscara adaptativa.
- [x] CA2 — `pnpm tauri icon <fuente>` genera los iconos de escritorio, iOS (`gen/apple/Assets.xcassets/AppIcon.appiconset`)
  y Android (`gen/android/.../mipmap-*`, adaptativo con fondo y primer plano); iOS sin transparencia. Se versiona lo
  generado.
- [x] CA3 — Comprobado en el simulador de iOS (pantalla de inicio y la app abierta), en el emulador de Android y, pendiente,
  en el iPhone del usuario.
- [x] CA4 — `design/look.md` (o `architecture.md`) dice de dónde sale el icono y cómo se regenera; `hx check full` verde.
