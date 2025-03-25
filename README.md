# RMQ - Cola de Mensajes Persistente en Rust

RMQ es una implementación ligera de una cola de mensajes persistente, escrita completamente en Rust utilizando únicamente la biblioteca estándar. Esta cola de mensajes está diseñada para facilitar la comunicación entre microservicios, permitiendo la transmisión de datos de manera desacoplada, confiable y eficiente.

## Características

- **Persistencia**: Los mensajes se almacenan en el sistema de archivos para asegurar su disponibilidad incluso después de caídas del sistema.
- **Control de concurrencia**: Utiliza mecanismos eficientes de concurrencia en Rust para permitir la manipulación de mensajes en entornos multi-hilo sin bloqueos innecesarios.
- **Purga periódica**: La cola realiza purgas periódicas de mensajes que ya han sido procesados, asegurando que no se acumule información innecesaria.
- **Desacoplamiento**: Ideal para arquitecturas basadas en microservicios, ya que permite que los servicios se comuniquen de forma desacoplada y asíncrona.
- **Facilidad de uso**: No necesitas ninguna configuración externa ni dependencias adicionales para empezar a usarla, solo la biblioteca estándar de Rust.