# MTQ - Cola de Mensajes Persistente en Rust

MTQ es una implementación ligera de una cola de mensajes persistente, escrita completamente en Rust utilizando únicamente la biblioteca estándar. Esta cola de mensajes está diseñada para facilitar la comunicación entre microservicios, permitiendo la transmisión de datos de manera desacoplada, confiable y eficiente.

## Características

- **Persistencia**: Los mensajes se almacenan en el sistema de archivos para asegurar su disponibilidad incluso después de caídas del sistema.
- **Control de concurrencia**: Utiliza mecanismos eficientes de concurrencia en Rust para permitir la manipulación de mensajes en entornos multi-hilo sin bloqueos innecesarios.
- **Purga**: La cola permite realizar purgas periódicas de los mensajes que ya han sido procesados, asegurando que no se acumule información innecesaria.
- **Desacoplamiento**: Ideal para arquitecturas basadas en microservicios, ya que permite que los servicios se comuniquen de forma desacoplada y asíncrona.
- **Facilidad de uso**: No necesitas ninguna configuración externa ni dependencias adicionales para empezar a usarla, solo la biblioteca estándar de Rust.

## Uso:

###  Requisitos

* **Rust**: Se requiere tener instalado Rust (compilador y gestor de dependencias cargo). Puedes instalarlo desde https://www.rust-lang.org/.

* **Sistema operativo**: Funciona en cualquier sistema operativo que sea compatible con Rust, incluyendo Linux, macOS y Windows (requiere WSL)

### Instalación

1. Clona el repositorio

2. Navega a la carpeta del proyecto.

3. Ejecutar tests unitarios

```bash
cargo test
```

4. Compila e inicia el servidor local:

```bash
cargo run -- <archivo-de-la-cola> <puerto>
```

Ejemplo:
```bash
cargo run -- "my_queue.bin" 9090
```

5. Tipos de peticiones posibles:

Puedes enviar mensajes a la cola de manera sencilla usando net-cat. El servidor detectará el tipo de mensaje en función de la cabecera.

- **Encolar un mensaje**

```bash
echo -n "enqueue <mensaje>" | nc -q 1 localhost <puerto>
```

- **Des-encolar un mensaje**

```bash
echo -n "dequeue" | nc -q 1 localhost <puerto>
```

- **Purgar el sistema**

```bash
echo -n "purge" | nc -q 1 localhost <puerto>
```