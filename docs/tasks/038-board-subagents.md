# 038 — Tablero: los subagentes, bajo su agente y plegados

- Estado: en curso (falta la comprobación en vivo, CA6, y los checks, CA7)
- Depende de: 015
- Riesgo: medio — cambia el orden del tablero (015), que decidió una lista plana; depende de que el contrato dé el padre
  de cada panel.
- Rama: `feat/038-board-subagents` · PR: `feat: fold an agent's subagents under it on the board`

## Objetivo
Que los subagentes no llenen el tablero. Hoy la 015 pinta una fila por panel ("Flat, one agent per pane", en
`design/board.md`), así que los agentes que lanza otro (un flow, `start`, `split`, un orquestador) salen como filas
sueltas mientras trabajan o han terminado. El usuario lo vio en su iPhone el 2026-11-09: "mientras están working o
done, salen en agente de panes. No sé cómo podemos gestionarlo".

Propuesta: hacer lo mismo que cornercase-gpui en su sección de agentes (cornercase-terminal `design/layout/agents-section.md`
› The agents section is a tree, Folding): árbol con los hijos bajo su padre, y plegado.

## Criterios de aceptación
- [x] CA1 — Árbol: cada panel con padre vivo y listado (`parent` del snapshot de `status`; comprobar en el contrato y en
  los fixtures de 0.13.19) va justo debajo de su padre, sangrado; sin padre listado es raíz. El orden de las raíces sigue
  el de la 015 (los que esperan arriba); los hijos, el suyo dentro del padre.
- [x] CA2 — Plegado por defecto: un agente con hijos sale plegado, con una flecha para desplegar. La fila plegada
  enseña, alineado a la derecha, lo que necesita atención de los ocultos (waiting, failed, pending, done; los que
  trabajan no dicen nada) y `+n`, como gpui. Un hijo que espera **nunca** queda escondido sin aviso: el padre lo dice y
  sube en el orden como si esperara él.
- [x] CA3 — El estado del plegado: si upstream expone el `collapsed` del panel (gpui lo guarda en `Term::collapsed`,
  compartido por todos los clientes), usarlo y, si hay comando, cambiarlo desde el móvil; si no, plegado local en la
  app y anotar el hueco en `design/upstream-gaps.md`. Decidido: plegado local por máquina, `boot` y panel; el
  `collapsed` de upstream nace desplegado y es compartido con el Mac, así que no cumple CA2 (`design/board.md` › Tree
  and folding, `design/upstream-gaps.md`).
- [x] CA4 — Los equipos de un flow se nombran por su rol (`coder`, `reviewer`) bajo su orquestador, como gpui, si el
  snapshot lo da; si no, por su título.
- [x] CA5 — Abrir un hijo desde el árbol abre su chat como cualquier agente; en el chat del padre, una línea con sus
  subagentes (n trabajando, n esperando) que lleva a ellos.
- [ ] CA6 — Comprobado en el simulador con un agente que lanza subagentes (o un flow) contra cornercase real; tests del
  árbol sin DOM (orden, padre ausente, ciclos, plegado, atención de los ocultos).
- [ ] CA7 — `hx check full` y mutation en verde.

## Referencias
- cornercase-terminal (`origin/main`): `design/layout/agents-section.md` (tree, folding, roles), `design/agent-status.md`
  (subagentes en segundo plano mantienen al agente en working).
- `design/board.md` (015): Flat, one agent per pane; el orden y las secciones.

## design/ que actualiza
- `design/board.md`: el árbol, el plegado, la atención de los ocultos; quitar "Flat" o explicar por qué cambió.
