# 058 — La app habla con el `/cli` de cornercase y empareja con su QR

- Estado: en curso (falta el iPhone por la LAN y por Tailscale)
- Depende de: 057 (hecha: cornercase 0.13.78)
- Rama: `feat/058-cornercase-cli` · PR: `feat: talk to cornercase's own /cli and pair with its code`

## Objetivo
Que la app funcione contra cornercase sin bridge: escanear el QR de **phone app** de cornercase, emparejar y hablar por
su `/cli`.

## Criterios de aceptación
- [x] CA1 — "Add machine" acepta el código de cornercase (QR o pegado) con varias rutas, como el del bridge (035), y
  canjea el código de emparejamiento por el secreto del dispositivo, que se guarda en el Keychain (iOS) y el Keystore
  (Android), nunca en claro ni en los logs.
- [x] CA2 — El transporte (`crates/core/src/transport/`) abre `/cli` con ese secreto y el mismo protocolo; la cola,
  los streams, la terminal y el resto no cambian.
- [x] CA3 — `contract` pide `cli-endpoint`; un cornercase sin `/cli` da un error que dice qué versión hace falta.
- [x] CA4 — Las rutas que cornercase anuncia tras conectar se añaden solas (lo que hacía `--bridge-routes`); la
  elección de ruta (035, 046) sigue igual.
- [x] CA5 — Una app olvidada en Settings de cornercase ve "Este Mac ya no reconoce el teléfono" y la opción de volver a
  escanear.
- [ ] CA6 — Fixtures de la versión de cornercase que traiga `/cli`; comprobación en vivo en el simulador y en el iPhone,
  por la LAN y por Tailscale.
  Hechos los fixtures (0.13.78) y el simulador el 2026-11-14 (`design/connection/machines.md` › Checked on 2026-11-14 for
  058); falta el iPhone.

## Decisiones
Tomadas el 2026-11-14 con los valores por defecto propuestos:
- P1: la ruta de cornercase es una variante nueva, `Route::Cli` (`{"kind": "cli", "url": "ws://h:p/"}`), con su secreto
  como `Token`; la 059 borra `Bridge` sin tocarla.
- P2: se empareja antes de buscar la misma máquina, y el secreto espera en memoria (10 min) al `add` o al `merge`; las
  firmas de IPC no cambian. El coste es una fila huérfana en el Mac si el usuario vuelve atrás.
- P3: `cli-endpoint` (y `app-routes`) se piden a todas las rutas, también las del bridge; la barra pide 0.13.78 o
  posterior.
- P4: la `User-Agent` es `cornercase-mobile/<versión> (iOS|Android)`.
- P5: volver a emparejar no olvida el dispositivo viejo en el Mac; queda como siguiente paso.

## design/ que actualiza
`design/connection.md` (transporte, errores, contrato, `ws://` en claro), `design/connection/machines.md` (código y
emparejamiento, fila huérfana, misma máquina, rutas aprendidas), `connection/bar.md` (`not_paired`, `missing` con
versión), `connection/store.md` (`NotPaired`), `connection/identity.md` (el secreto del dispositivo), `design/bridge.md` y
`bridge/machine-code.md` pasan a historia (por qué se hizo y qué se llevó upstream), `design/upstream-gaps.md` (se cierra
`/cli`), `design/look.md` (el aviso de borrar) y `DESIGN.md`.
