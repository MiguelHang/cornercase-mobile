# 026 — Pantalla 5: ficheros (`files list/read/search`)

- Estado: pendiente
- Depende de: 025
- Riesgo: bajo
- Rama: `feat/026-files` · PR: `feat: browse, read and search a workspace's files`

## Objetivo
Navegar el árbol de un workspace, leer ficheros y buscar por nombre.

## Criterios de aceptación
- [ ] CA1 — `files list --json`, `files read --json`, `files search --json` tipados en el núcleo con los fixtures de
  upstream `tests/fixtures/control/files/v1/` (`listing`, `search`, `image`).
- [ ] CA2 — Árbol navegable (carpetas que se abren bajo demanda), lectura con números de línea y scroll horizontal;
  ficheros binarios o enormes muestran un aviso en vez del contenido; imágenes se ven si `files read` las da.
- [ ] CA3 — Búsqueda por nombre con resultados al escribir (con espera de 200 ms y cancelando la anterior).
- [ ] CA4 — Enlace desde un fichero de la pantalla de cambios a su contenido completo.
- [ ] CA5 — Resaltado de sintaxis opcional: solo si cabe en el presupuesto de bundle (anotarlo); si no, texto plano.

## Referencias
- cornercase-gpui: `design/files-panel.md`, `src/panels/files/there.rs`, `src/panels/files/tree.rs`,
  `src/panels/files/search.rs`, `src/panels/files/viewer.rs`.
- cornercase-terminal (`origin/main`): `design/commands-for-scripts-and-agents/files.md`, `design/files-panel.md`.

## design/ que actualiza
- `design/changes-and-files.md`.

## Verificación
En el simulador se abre un fichero del workspace y se encuentra otro buscando (criterio 5, parte ficheros).
