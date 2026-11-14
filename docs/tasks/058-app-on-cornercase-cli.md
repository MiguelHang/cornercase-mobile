# 058 — La app habla con el `/cli` de cornercase y empareja con su QR

- Estado: pendiente
- Depende de: 057 (en el fork mientras upstream no lo publique)
- Rama: `feat/058-cornercase-cli` · PR: `feat: talk to cornercase's own /cli and pair with its code`

## Objetivo
Que la app funcione contra cornercase sin bridge: escanear el QR de **phone app** de cornercase, emparejar y hablar por
su `/cli`.

## Criterios de aceptación
- [ ] CA1 — "Add machine" acepta el código de cornercase (QR o pegado) con varias rutas, como el del bridge (035), y
  canjea el código de emparejamiento por el secreto del dispositivo, que se guarda en el Keychain (iOS) y el Keystore
  (Android), nunca en claro ni en los logs.
- [ ] CA2 — El transporte (`crates/core/src/transport/`) abre `/cli` con ese secreto y el mismo protocolo; la cola,
  los streams, la terminal y el resto no cambian.
- [ ] CA3 — `contract` pide `cli-endpoint`; un cornercase sin `/cli` da un error que dice qué versión hace falta.
- [ ] CA4 — Las rutas que cornercase anuncia tras conectar se añaden solas (lo que hacía `--bridge-routes`); la
  elección de ruta (035, 046) sigue igual.
- [ ] CA5 — Una app olvidada en Settings de cornercase ve "Este Mac ya no reconoce el teléfono" y la opción de volver a
  escanear.
- [ ] CA6 — Fixtures de la versión de cornercase que traiga `/cli`; comprobación en vivo en el simulador y en el iPhone,
  por la LAN y por Tailscale.

## design/ que actualiza
`design/connection.md` y `design/connection/machines.md` (el código y el emparejamiento), `design/bridge.md` pasa a
historia (por qué se hizo y qué se llevó upstream).
