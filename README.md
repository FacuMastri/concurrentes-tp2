<h1>
Coffeewards

<a href="docs/consigna.md">
  <img align="right" height="40"
  alt="consigna" src="https://cdn-icons-png.flaticon.com/512/2541/2541984.png">
</a>

</h1>

![Coffeewards](docs/coffeewards.png)

Coffeewards, es un sistema de puntos para fidelización de los clientes.

Por cada compra que realizan los clientes, suman puntos que luego pueden canjear por cafes gratuitos.

> **Nota:** Los gráficos y diagramas aquí presentes son son ilustrativos y apuntan a transmitir el concepto del sistema. Puede haber _abreviaciones_ o _alteraciones_ tanto de las _entidades_ como de las _operaciones_ en pos de _simplificar_ el entendimiento de las características esenciales.

<!--
- [x] explicación del diseño y de las decisiones tomadas para la implementación
- [x] diagramas de threads y procesos, y la comunicación entre los mismos
- [x] diagramas de las entidades principales
-->

## Diseño

### Arquitectura

<!--
- resumen de arquitectura
- cliente-servidor ( coffee_maker-server )
- txs distribuidas en servidores
- puntos disponibles/reservados -> 2txs de uso simultaneas
- supuestos -> los servers locales no pierden conexión con la cafetera
> Detalles de implementación
-->

El sistema se implementa como una arquitectura **cliente-servidor**, donde las cafeteras se conectan a su servidor local para sumar o usar puntos a medida que recibe pedidos.

Los **servidores** se comunican entre si para mantener consistente el estado de las cuentas de forma **distribuida**. Esto se implementa mediante **transacciones en 2 fases**.

Se distinguen 4 acciones principales:

- **Reservar** puntos `Lock`
- **Soltar** puntos reservados `Free`
- **Consumir** puntos reservados `Consume`
- **Añadir** puntos `Add`

Para reservar puntos se **requiere** que por lo menos la **mitad** de los servidores estén **disponibles**.
En cambio, las otras transacciones ( asumiendo que los puntos fueron previamente reservados si fuese necesario ) no deberían fallar y quedan pueden quedar pendientes hasta que sea posible resolverlas.

Al **procesar una orden**, primero se reservan los puntos necesarios y al finalizarla se añaden/liberan/consumen los puntos reservados.
La respectiva cuenta solo se bloquea mientras se procesan estas transacciones y no mientras se prepara el café, lo cual permite que se puedan procesar pedidos de una misma cuenta **concurrente**.

#### Supuestos

- Se asume que las cafeteras no pierden conexión con el servidor local.
- Se asume que los servidores pueden perder conexión con la red, pero siguen siendo parte de la misma durante toda la ejecución.
- Se asume que no habrá agentes externos al sistema que intenten afectarlo.
- El proceso del servidor no es interrumpido de manera inesperada.

### Cafetera `coffee_maker`

<!--
- esquema de actores
- diagrama de flujo de handle order
> Detalles de implementación
-->

El programa de la cafetera se encarga de recibir pedidos de los clientes de un archivo y procesar los.

Este se implementa utilizando un esquema de actores:

```mermaid
flowchart LR
    ot(OrderTaker) --> sa{ }
    sa --> oh1(OrderHandler)
    sa --> oh2(OrderHandler)
    sa --> oh3(OrderHandler)
    oh1 --> ps(PointStorage)
    oh2 --> ps(PointStorage)
    oh3 --> ps(PointStorage)
```

<!--
![ActorsDiagram](docs/actors.svg)
-->

- `OrderTaker`: Recibe los pedidos y los delega
- `OrderHandler`: Prepara los cafes. Hay uno por dispenser.
- `PointStorage`: Se encarga de las operaciones de puntos, comunicándose con el servidor local.

<details>

<summary><h4>Detalles de Implementación</h4></summary>

##### Error reservando puntos

```mermaid
sequenceDiagram
  participant oh as OrderHandler
  participant ps as PointStorage

  oh ->> ps: reservar puntos
  ps -->> oh: Err

  note over oh,ps: Error
```

##### Orden exitosa

```mermaid
sequenceDiagram
  participant oh as OrderHandler
  participant ps as PointStorage

  oh ->> ps: reservar puntos
  ps -->> oh: Ok

  note over oh: hace cafe correctamente

  oh ->> ps: consumir puntos
  ps -->> oh: Ok

  note over oh,ps: Éxito
```

##### Orden fallida

```mermaid
sequenceDiagram
  participant oh as OrderHandler
  participant ps as PointStorage

  oh ->> ps: reservar puntos
  ps -->> oh: Ok

  note over oh: falla en hacer cafe

  oh ->> ps: liberar puntos
  ps -->> oh: Ok

  note over oh,ps: Error
```

</details>

### Servidor local `server`

<!--
- como maneja clientes
- como maneja msgs ( con, sync, ping, tx )
- offline -> pending
> Detalles de implementación

-->

El servidor local se encarga tanto de recibir y procesar los mensajes de los **clientes** como de **comunicarse** con el resto de los **servidores** para mantener **consistente** el estado de las cuentas.

Toda la comunicación se realiza mediante **TCP**.

#### Servicio a Clientes

Cuando un cliente abre una conexión, el servidor crea un **hilo** para manejarla.
En este recibe **pedidos** (`order`) y los maneja secuencialmente hasta que el cliente se desconecta.
El servidor le **responderá** al cliente si el pedido fue exitoso o no.

#### Comunicación entre Servidores

Los servidores abren una **conexión** para cada comunicación con otro servidor.
Una comunicación entrante se resuelve en un nuevo **hilo** y puede implicar el intercambio de **varios mensajes**.

Los **tipos** de comunicación son:

- `PING`
  - Se utiliza para verificar si el servidor tiene conexión.
  - Secuencia: `PingRequest` , `PingResponse`
- `CONNECT`
  - Se utiliza para conectar un nuevo servidor a la red.
  - Secuencia: `ConnectRequest(new_server)` , `ConnectResponse(servers)`
- `SYNC`
  - Se utiliza para sincronizar el estado de las cuentas
  - Secuencia: `SyncRequest` , `SyncResponse(point_map)`
- `TRANSACTION`
  - Se utiliza para realizar una [transacción distribuida](#transacciones_distribuidas).

#### Perdida de Conexión

Cuando un servidor **no recibe respuesta de ningún otro** , tanto al realizar una transacción como al enviar pings, detecta que esta **desconectado**.

Por otro lado, **al recibir** algún mensaje o respuesta detecta que esta **conectado**.

Cuando una transacción falla, pero podría ser resuelta (eg. una carga de puntos estando desconectado) esta se guarda en una lista de **pendientes**,
que se intentan de procesar en un **hilo** dedicado.

Cuando el servidor se **desconecta** (conectado -> desconectado) **detiene** el procesamiento de pendientes.

Cuando el servidor se **reconecta** (desconectado -> conectado) se **sincroniza** con los demás servidores y **reanuda** el procesamiento de pendientes.

<details >
<summary><h4 id="transacciones_distribuidas">Transacciones Distribuidas</h4></summary>

El servidor que recibe el pedido hace de **coordinador** de la transacción.

Las transacciones se ejecutan en **2 fases**:

1. Preparación [`PREPARE`]
   - El coordinador intenta tomar el recurso necesario
   - Verifica poder realizar la transacción
   - Comienza una comunicación de tipo `TRANSACTION` con los demás servidores
2. Finalización [`COMMIT`/`ABORT`]
   - Al recibir el mensaje, los servidores locales:
     - Intentan tomar el recurso necesario
     - Verifican poder realizar la transacción
     - Responden `Proceed` o `Abort` según corresponda
   - Al recibir las respuestas
     - Si mas de la mitad respondieron `Proceed`, y ninguno `Abort`:
       - El coordinador envía `Proceed` a los demás servidores
       - Todos los servidores aplican la transacción
     - Si faltan suficientes respuestas o alguna es `Abort`:
       - El coordinador envía `Abort` a los demás servidores
       - Agrega la transacción a la lista de pendientes, si puede ser resuelta mas adelante

Para prevenir deadlocks, se implementa **wait-die**

##### Transacción Exitosa

```mermaid
sequenceDiagram
  participant co as Coordinator
  participant s1 as Server
  participant s2 as Server

  co ->> s1: TRANSACTION
  co ->> s2: TRANSACTION
  s1 -->> co: Proceed
  s2 -->> co: Proceed
  co ->> s1: Proceed
  co ->> s2: Proceed
  note over co,s2: Transacción Exitosa
```

##### Transacción Abortada

```mermaid
sequenceDiagram
  participant co as Coordinator
  participant s1 as Server
  participant s2 as Server

  co ->> s1: TRANSACTION
  co ->> s2: TRANSACTION

  s1 -->> co: Proceed
  s2 -->> co: Abort

  co ->> s1: Abort
  co ->> s2: Abort

  note over co,s2: Transacción Fallida
```

##### Transacción Abortada por falta de respuestas

```mermaid
sequenceDiagram
  participant co as Coordinator
  participant s1 as Server
  participant s2 as Server
  participant s3 as Server

  co ->> s1: TRANSACTION
  co -x s2: TRANSACTION
  co -x s3: TRANSACTION

  s1 -->> co: Proceed

  note over s2: Timeout
  note over s3: Timeout

  co ->> s1: Abort
  co -->> s2: Abort
  co -->> s3: Abort
  note over co,s3: Transacción Fallida
```

</details>

<details >

<summary><h4>Detalles de Implementación</h4></summary>

##### Diagrama de Clases

```mermaid

classDiagram
  direction LR

  class Server {
    listener: TcpListener

    listen()
    handle_stream(TcpStream)
  }
  class PointStorage {
    servers : Addr[]
    pending : Transaction[]

    coordinate(Message)
    handle(Message)
  }

  class PointRecord {
    available : Int
    locked : Int

    coordinate(Transaction)
    handle(Transaction)
    apply(Transaction)
  }
  class Transaction {
    client : Id
    amount : Int
    action : TxAction
    timestamp : Timestamp

    olderThan(Transaction)
  }

  Server -- PointStorage : points
  PointStorage *-- PointRecord : points
  PointStorage o-- Transaction : pending
  PointRecord -- Transaction : holder

```

##### Threads

```mermaid
flowchart LR
    subgraph Local Server
      s[Server]
      c1(Client) --> s
      c2(Client) --> s

      s --> c(CoordinateTx)
      s --> h(HandleTx)
    end
    subgraph External Processes
      c -.- eh1(HandleTx)
      c -.- eh2(HandleTx)
      h -.- ec(CoordinateTx)

      eh1 --- s1[Server]
      eh2 --- s2[Server]
      ec --- s2
    end
```

##### Secuencia de una orden

```mermaid
sequenceDiagram
    participant c as Client
    participant s as Server
    participant ps as PointStorage
    participant pr as PointRecord
    participant e as External

    c->>+s: Fill ( id: 1, amount: 1)

    s ->>+ ps: coordinate( fill, 1, 1 )
    ps --> pr: wait-die
    ps ->> pr : coordinate( Transaction )

    note over pr,e : Successful Transaction

    pr ->> ps: Ok
    ps ->> s: Ok

    s ->>-c: Ok
```

##### Secuencia de una transacción

```mermaid
sequenceDiagram
    participant e as ExternalServer
    participant s as Server
    participant ps as PointStorage
    participant pr as PointRecord

    e->>+s: Transaction

    s ->> ps: handle( Transaction )
    ps --> pr: wait-die
    ps ->> pr: handle( Transaction )

    pr -->> e : Proceed
    e ->> pr: Proceed | Abort

    note over pr : Apply | Abort

    s -->-e: end connection
```

</details>

### Controlador `controller`

<!--
- que es
> Detalles de implementación
-->

El controlador es un programa que esta por fuera del sistema principal.
Se utiliza para enviar mensajes de **control** a los servidores.

Estos mensajes pueden ser:

- `Disconnect` : El servidor descartara todos los mensajes recibidos por otro servidor y fallara en enviar mensajes a otros servidores.
- `Connect` : El servidor recuperara la capacidad de enviar y recibir mensajes a otros servidores.

El programa escucha constantemente por `stdin` por comandos indicando la acción a realizar y la dirección del servidor.

## Desarrollo

- `make` en el directorio raíz corre `fmt`, `test` y `clippy` para el espacio de trabajo.
- `coffee_maker <local_server> [<orders>]`
- `local_server <address> [<known_server_address>]`
- `controller`
  - `<Disconnect/Connect> <address>`

> las direcciones son de la forma `ip:puerto` o `puerto` (en cuyo caso se usa `localhost`)
