# Orquestación OpenCode del proyecto

Configuración local del 2026-10-04 para OpenCode 2.0.22. La conexión Go ya
registrada en la cuenta se reutiliza; no se guardan claves ni credenciales aquí.

## Equipo

| Perfil | Modelo | Responsabilidad |
| --- | --- | --- |
| `orchestrator` | `opencode-go/deepseek-v4.1-flash` | Coordinar y delegar; no implementar |
| `general` | `opencode-go/glm-5.3` | Contratos, arquitectura, formatos y aceptación |
| `scout` | `opencode-go/deepseek-v4.1-flash` | Localizar archivos, símbolos y pruebas |
| `builder` | `opencode-go/deepseek-v4.1-flash` | Implementar y corregir por archivos asignados |
| `explore` | `opencode-go/glm-5.3` | Auditoría independiente y validación |
| `local-coder` | `strata/qwen3.8-flash-next-coder#equilibrado` | Constructor local opcional y acotado |

Solo el coordinador es un agente principal. Los demás son subagentes.
Los perfiles incorporados `build` y `plan` se deshabilitan en este proyecto
para que la sesión inicial use la orquestación. La configuración global de
OpenCode y los otros proyectos se conservan.

El constructor local no es el predeterminado: en la prueba anterior, el contexto
casi lleno de 64K tardó unos 11 minutos y el borrador Rust requirió reparación
remota. Este proyecto usa la elección posterior del usuario: acelerar con
DeepSeek. No se utilizan Kimi ni GPT como fallback automático.

## Arranque

Desde este repositorio:

```bash
./.opencode/start
```

Desde cualquier directorio:

```bash
/home/joe/projects/dream/wow/rustycore-extractors/.opencode/start
```

Para una tarea sin interfaz:

```bash
./.opencode/start run "Revisa el issue y los cambios actuales; planifica y delega el trabajo acotado, conserva los cambios existentes y termina con pruebas y auditoría."
```

Para mantener una sesión, se admiten los argumentos de OpenCode como
`--continue` o `--session`. Cada invocación inicia un servidor privado que carga
los perfiles y hereda las rutas de compilación. `--auto` aprueba las acciones
permitidas; los rechazos explícitos siguen vigentes.

## Flujo de una tarea

1. Leer `AGENTS.md`, el issue elegido y `git status`; identificar la versión
   objetivo y los cambios preexistentes. No cambiar una rama sucia sin resolver
   primero la pertenencia de esos cambios.
2. `scout` localiza el área concreta. `general` fija invariantes, formatos,
   fallos esperados y pruebas de aceptación si la tarea lo requiere.
3. El coordinador asigna archivos explícitos a `builder`. Un único escritor
   por archivo. No delegar a todos los perfiles por rutina si no hacen falta.
4. `builder` aplica cambios y valida de forma acotada. `explore` revisa el diff
   y verifica las pruebas de forma independiente.
5. Los hallazgos vuelven al constructor y se revisan tras corregirlos. El
   informe final distingue tests, skips, lectura estática y aceptación pendiente.

Los handoffs incluyen objetivo, archivos propios, cambios que preservar,
contrato aplicable, casos positivos/negativos y comandos de validación. No se
llena el contexto de 64K artificialmente ni se envía el repositorio entero
para una modificación pequeña.

## Validación y rutas

El lanzador y cada comando de pruebas fuerzan estas rutas externas:

- Cargo: `../rustycore-extractor-work/builds/cargo-opencode/`.
- C++: `../rustycore-extractor-work/builds/cpp-opencode/`.
- Temporales: `~/.local/share/rustycore-extractor-work/tmp/opencode/`, fuera del
  repositorio Git padre `wow`, también para las fixtures sintéticas de extracción.
- Caché Python: `../rustycore-extractor-work/cache/python-opencode/`.
- Informes: `../rustycore-extractor-work/reports/opencode/`.

Comandos disponibles para los agentes:

```bash
.opencode/checks fmt-check
.opencode/checks rust
.opencode/checks clippy
.opencode/checks db2
.opencode/checks synthetic
.opencode/checks contracts
.opencode/checks conformance
.opencode/checks provenance
.opencode/checks cpp
.opencode/checks native
```

Rust se ejecuta con `--offline --locked`; no se instala nada ni se descargan
fuentes automáticamente. `synthetic` compila el inspector y ejecuta la suite
Python con el binario seleccionado. En los modos ordinarios el helper elimina la selección heredada
de cliente real: las pruebas live quedan desactivadas. Los modos explícitos
`live` y `live-extract` configuran y ejecutan la aceptación local autorizada. Los skips nativos sin
backend no equivalen a aceptación del backend.

`native` necesita `RCE_FOREVER_CASC_REFERENCE` apuntando a una referencia local
ya auditada y fijada; el contrato nativo verifica su procedencia. Las opciones
OpenSSL del proyecto se pueden suministrar por su entorno existente. `cpp`
configura, compila y ejecuta CTest fuera del checkout. Ninguno inicia extracción
completa ni ejecuta contra el cliente real.

Solo `builder` puede usar `.opencode/checks fmt`; comprobar primero y evitar
formatear archivos ajenos. El auditor dispone únicamente de `fmt-check`.

## Límites y datos

Los agentes remotos trabajan con código, fixtures sintéticas y reportes
estructurados revisados y redactados. No enviar bytes del cliente, DB2/DBC
reales, CASC, claves, secretos ni valores de `.env`. Los perfiles excluyen la
lectura de rutas habituales de datos y permiten las fixtures sintéticas
preparadas deliberadamente. El coordinador o el constructor asignado pueden crear un
commit local tras revisar el estado, el diff preparado, el alcance del issue y los checks
obligatorios. Los agentes de investigación y auditoría no hacen commits. Ningún agente
puede hacer push o publicar sin aprobación explícita del usuario. La configuración de
agentes se administra fuera del circuito de implementación; los agentes no la modifican.

Los permisos de OpenCode son reglas para sus herramientas, no un sandbox del
sistema operativo. Las pruebas ejecutan código del repositorio: revisar sus
entradas y sus efectos sigue siendo necesario. Las instrucciones de `AGENTS.md`,
las licencias y los gates de aceptación continúan aplicándose.

Para un experimento local explícito, arrancar `strata-iniciar` y pedir al
coordinador que use `local-coder` para una tarea pequeña. No se arranca Strata
al abrir este proyecto. Los modelos están fijados y un fallo de cuota/acceso
se comunica; no se cambia automáticamente a otro proveedor.

## Verificación de instalación

OpenCode resolvió los seis perfiles con sus modelos y modos esperados. La
prueba de delegación de solo lectura obtuvo respuesta de scout, general,
builder y explore. No hubo herramientas de escritura en esas delegaciones,
cambios de rama ni ejecución de pruebas de la aplicación. `bash -n` pasó
para el lanzador, el entorno y los helpers. Rust 1.98.0, rustfmt y clippy ya
estaban instalados.

Evidencia local: `../rustycore-extractor-work/reports/opencode/`, archivos
`agents-resolved.json`, `orchestration-smoke.jsonl` y `verification.json`.
La sintaxis y la delegación verificadas no constituyen aceptación de los
extractores ni de los cambios preexistentes del proyecto.

## Consultas Git y órdenes auxiliares

Los seis perfiles pueden consultar status, diff, log, show, ls-files, ramas,
stash list/show y worktree list, además de ls y echo. Los comandos compuestos
requieren que cada componente esté permitido. Se prefieren llamadas separadas
y las herramientas glob/grep para localizar archivos. Esto corrige la lista
inicial, que rechazaba consultas legítimas de stash, worktrees y separadores.
Las operaciones que modifican historial, stashes o worktrees no se conceden
como consultas de lectura.

## Goals persistentes

Está instalado `@prevalentware/opencode-goal-plugin` 0.1.54 para servidor y
terminal de OpenCode. El instalador integrado falló con un error HTTPS; se
instaló la versión fijada con npm, fuera del checkout, en
`/home/joe/.local/share/opencode/local-plugins/goal-0.1.54/`. Dos adaptadores
locales reexportan los entrypoints originales para la resolución por directorio
de OpenCode 2.0.22; el código del plugin no se modificó. La configuración global
`opencode.json` y `cli.json` referencia esos adaptadores, por lo que el plugin
está disponible también en otros proyectos.

Reabrir la interfaz si estaba abierta antes de instalarlo. En el chat:

```text
/goal Completar la tarea acordada, conservar los cambios existentes y terminar con pruebas y auditoría independiente.
/goal
/goal pause
/goal resume
/goal stop
```

También se ofrecen `/pause_goal` y `/resume_goal`, y la paleta Ctrl+P incluye
`Goal`. El orquestador tiene permisos explícitos para las herramientas del
ciclo de vida; los subagentes siguen reportando a él. Un goal se crea solo
por petición explícita del usuario, y su cierre exige evidencia de validación.
Los permisos y el alcance del proyecto se conservan.

La continuación automática está habilitada con los valores predeterminados:
espera a los subagentes activos y permite 25 continuaciones por ventana. Tras
ese límite se puede reanudar con `/goal resume`. El estado se guarda por sesión
fuera del repositorio en `~/.local/share/opencode-goal-plugin/goals.json`.

Verificación de instalación: plugin servidor `local.goal-mode.server` activo;
comandos goal/pause_goal/resume_goal registrados; herramienta `get_goal`
ejecutada correctamente por el orquestador (resultado `goal: null`); entrada
Goal visible en la paleta de la terminal. No se creó un objetivo de desarrollo
ni se probó una ejecución autónoma completa. Registros en
`../rustycore-extractor-work/reports/opencode/goal-smoke.jsonl`.

## Autonomía y aceptación real — actualización 2026-10-04

### Directorios de continuación — actualización 2026-10-17

Los seis perfiles autorizan `external_directory` para
`/mnt/nvme2/datos/forever-product-runs` y sus descendientes, y para los
directorios del proyecto `~/.local/share/rustycore-*`. Esto permite recuperar
los checkpoints y las copias de trabajo privadas usados en la continuación.
`builder` y `local-coder` tienen en
`~/.local/share/rustycore-compatible-world-coverage-source-v1/source/` los
mismos permisos de edición por archivo que en el checkout. Las restricciones
existentes de cliente, entradas, configuración y dependencias se conservan.

Después de modificar perfiles, ejecuta `opencode reload` desde este checkout
para recargar el servidor compartido. Una instancia privada iniciada con
`--standalone` requiere dirigir la recarga a su URL mediante
`opencode reload --server URL`.

La revisión general de esta fecha también habilita skills, Code Mode y los MCP
`rea` y `typesafe-jev` en los seis perfiles. Se eliminan los límites `steps`
de 10–32 pasos y se actualizan las instrucciones antiguas para respetar la
autorización persistente de compilación, extracción local, commit/push y fusión
de implementaciones validadas. El coordinador conserva la propiedad de las
operaciones Git compartidas. Los límites de recursos de los extractores y la
ventana de continuación del plugin Goal se mantienen independientes.

REA se registra en el `opencode.json` del proyecto mediante el formato MCP de
OpenCode V2: ejecuta `~/.local/bin/rea mcp`, usa Code Mode y reutiliza las rutas
locales existentes de Ghidra, Java y temporales. El timeout de ejecución es
1.860.000 ms, igual al configurado en Codex; el registro no instala motores ni
inicia análisis de binarios. La skill `reverse-engineer-anything` ya instalada
en `~/.agents/skills` se descubre por compatibilidad, sin copiarla ni sustituirla.
`gh` usa la autenticación existente del usuario; no se guardan credenciales en
el repositorio. Publicar releases y datos del cliente sigue fuera de esta
autorización de desarrollo.

El usuario autorizó que los agentes completen el circuito y ejecuten los
comandos necesarios. Los seis perfiles permiten ejecución shell para tareas
del proyecto, además de los helpers: compilación, tests, binarios propios y
consulta de las referencias externas. El coordinador puede ejecutar validación
directa; los cambios de código siguen delegados con propiedad por archivo.
No se pide al usuario que ejecute comandos o pegue metadatos disponibles localmente.

`external_directory` admite la integración RustyCore, la referencia C++ fijada,
el laboratorio y el cliente local. Las herramientas read/edit deniegan bytes
privados del cliente y los artefactos crudos de las corridas. Las herramientas
locales sí pueden consumirlos y devolver reportes estructurados redactados.
La referencia/client/inputs se conservan sin modificaciones y no se publica nada.

`environment.sh` suministra los defaults de RCE_FOREVER_CASC_REFERENCE y los
tres RCE_OPENSSL_* para las dependencias ya presentes; los helpers los cargan
cada vez, aunque una sesión antigua arrancase sin esas variables. El cliente
se selecciona mediante RCE_OPENCODE_CLIENT. No se descarga ninguna dependencia.

```bash
.opencode/checks native
.opencode/checks live
.opencode/checks live-extract
```

- `native`: tests y compilación nativa, más CLI sintéticas, sin cliente real.
- `live`: compilación de los binarios y suite CLI, incluidas pruebas locales.
- `live-extract`: compilación del extractor y una corrida acotada a las dos
  tablas por defecto, esES, con rechazo de syscalls de red. No extracción completa.

El laboratorio vecino está dentro del repositorio Git padre `wow`, por lo que
el extractor rechaza publicar DB2 allí. Las salidas reales y sus temporales se
guardan en RCE_OPENCODE_RUN_ROOT, por defecto
`~/.local/share/rustycore-extractor-work/runs/candidate/`, fuera de cualquier
checkout. Los reportes redactados se copian al laboratorio habitual bajo
`reports/acceptance/`. No se mueve ni elimina ninguna corrida anterior.

Validación de esta configuración: lectura/grep de la referencia y git -C
funcionaron sin permission denied. El HEAD es
02245dcd245e7433e524577656177723d3e4992e. DB2Metadata.h de ese worktree es un
stub vacío; git show HEAD:src/server/game/DataStores/DB2Metadata.h ofrece el
blob fijado: ChrClassesMeta declara 43 campos, ChrRacesMeta 51.

La corrida real 70205 ejecutada devolvió incomplete/exit 8, sin intentos de
red y con .build.info sin cambios. ChrClasses: invalid_record_ids (schema true).
ChrRaces: unsupported_table_schema (schema false). El modo native encontró
además errores Rust en el binario probe, incluido un import crate::Locale no
resuelto. Son fallos del código actual que el constructor debe corregir, no
restricciones de configuración. No se afirma aceptación ni decoder correcto.

Evidencia: reports/opencode/external-permissions-smoke.jsonl,
reports/opencode/native-config-verification.log,
reports/opencode/live-config-verification.json. El JSON acotado es legible por
los agentes; nunca se necesita enviar el DB2 ni los bytes del cliente al modelo.
