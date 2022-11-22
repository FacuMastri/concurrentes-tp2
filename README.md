<h1>
Coffeewards

<a href="docs/consigna.md)">
  <img align="right" height="40"
  alt="ES" src="https://cdn-icons-png.flaticon.com/512/2991/2991112.png">
</a>

</h1>

![Coffeewards](assets/CoffeeWards.gif)

Coffeewards, es un sistema de puntos para fidelización de los clientes.

Por cada compra que realizan los clientes, suman puntos que luego pueden canjear por cafes gratuitos.

<!--
- [ ] explicación del diseño y de las decisiones tomadas para la implementación
- [ ] diagramas de threads y procesos, y la comunicación entre los mismos
- [ ] diagramas de las entidades principales
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
- Se asume que los servidores pueden perder conexión con la red, pero siguen siendo parte de la misma durante toda la ejecución
- Se asume que no habrá agentes externos al sistema que intenten afectarlo.

### Cafetera `coffee_maker`

<!--
- esquema de actores
- diagrama de flujo de handle order
> Detalles de implementación
-->

### Servidor local `server`

<!--
- como maneja clientes
- como maneja msgs ( con, sync, ping, tx )
- offline -> pending
> Detalles de implementación
-->

### Controlador `controller`

<!--
- que es
> Detalles de implementación
-->

## Desarrollo

- `make` en el directorio raíz corre `fmt`, `test` y `clippy` para el espacio de trabajo.
- `coffee_maker <local_server> [<orders>]`
- `local_server <address> [<known_server_address>]`
- `controller`
  - `<Disconnect/Connect> <address>`

> las direcciones son de la forma `ip:puerto` o `puerto` (en cuyo caso se usa `localhost`)
