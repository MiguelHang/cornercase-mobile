# 062 — iPad y tablets: dos paneles, terminal ancha y teclado físico

- Estado: pendiente (no es para ahora; decidido por el usuario el 2026-11-15)
- Depende de: 060 (la 0.1.0 sale solo para iPhone, 060 › CA8)
- Rama: `feat/062-ipad-tablet` · PR: `feat: a tablet layout for iPad and Android tablets`

## Objetivo
Que la app sea buena en iPad y en tablets Android, no el diseño del móvil estirado. Hoy el proyecto de iOS ya declaraba
iPhone y iPad (`TARGETED_DEVICE_FAMILY = "1,2"`) y en una tablet la WebView pinta lo mismo que en el móvil, a todo el
ancho. Es trabajo de la interfaz (`src/`) y del proyecto de Xcode; ni la conexión ni `crates/core` cambian. Puede
partirse en 2–3 PRs al planificarla.

## Criterios de aceptación
- [ ] CA1 — Dos paneles en pantallas anchas: el tablero a la izquierda y el agente elegido (chat, terminal, cambios,
  ficheros) a la derecha, como Mail. Por debajo de un ancho, la navegación del móvil de siempre; el cambio sigue al
  tamaño de la ventana (Split View, Stage Manager, multiventana de Android), no al tipo de dispositivo.
- [ ] CA2 — Terminal ancha: en una tablet la terminal usa las columnas que caben, cerca de las del Mac, en vez del
  ajuste al móvil de la 043 (que se queda para pantallas estrechas).
- [ ] CA3 — Teclado físico y puntero: atajos para la terminal (Ctrl, Esc, flechas, Tab) y el chat (enviar), y estados
  de hover y cursor con trackpad o ratón.
- [ ] CA4 — Hojas y menús como popover anclado en pantallas anchas, no como hoja de abajo.
- [ ] CA5 — Vertical y apaisado, con el contenido de ancho limitado donde una sola columna no tiene sentido a 1.300 px.
- [ ] CA6 — iPad de vuelta en el proyecto (`TARGETED_DEVICE_FAMILY: "1,2"` y sus orientaciones), con capturas de iPad
  para App Store Connect.
- [ ] CA7 — Comprobado en el simulador de iPad (incluido Split View) y en un emulador de tablet Android; decisiones en
  `design/` (área nueva o `design/look.md`); `hx check full` verde.
