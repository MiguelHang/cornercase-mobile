# 061 — Upstream: el servidor arrancado al iniciar sesión no encuentra los agentes

- Estado: pendiente (issue en cornercase-terminal, con la 033)
- Depende de: —

## Objetivo
Visto en el iPhone (2026-11-13): "No agent is installed on this Mac" al arrancar un agente. El servidor de cornercase
lo arrancó la app de escritorio al iniciar sesión (padre `launchd`) con `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, y
`agents::installed` (`process::program::locate`) busca con el `PATH` del servidor, así que `claude` en `~/.local/bin`
sale como no instalado en `account list --check --json`, en la app y en el diálogo de flows del escritorio. Las
pestañas sí lo arrancan: su shell carga el `PATH` del usuario.

## Qué se pide
Que el servidor tome el `PATH` de la shell de login del usuario al arrancar sin uno útil (como hacen las apps de macOS
lanzadas desde el Dock), o que `installed` busque además en `~/.local/bin`, `~/.cargo/bin` y Homebrew, como ya hace
`--remote-command`.

## Mientras
`cornercase config set agent_commands.claude <ruta absoluta>`, o arrancar el servidor desde una terminal.
