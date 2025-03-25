/* Implementación de una cola de mensajes en rust usando solo la librería estándar

Cada mensaje se almacena en un formato binario de la siguiente forma

[Estado (1 byte)] [Timestamp (8 bytes)] [Tamaño (4 bytes)] [Mensaje (variable)]


Al ser una estructura de cola, los consumidores leen el último mensaje.

MAX_MESSAGE_SIZE es 256KB para evitar que se guarden mensajes muy largos.

*/


use std::fs::OpenOptions;
use std::io::{Read, Write, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};


const MAX_MESSAGE_SIZE: usize = 256 * 1024; // 256 KB



struct Queue {
    file: String,
    wlock: Arc<Mutex<()>>, // Mutex para evitar condiciones de carrera en escritura WRITE- WRITE
    rlock: Arc<Mutex<()>>,   
    /* 
    Este patrón de locking permite sincronizar el acceso a recursos sin encapsular directamente el recurso (file, en este caso, en el mutex)
    - Al ser un mutex vacío funciona sólo como señalizador de concurrencia sin bloquear directamente el acceso a la estructura completa.
    - Esto permite separar el lock de lectura del lock de escritura, lo que aporta flexibilidad y personalmente lo prefiero frente a usar bloqueos implícitos como se haría con Arc<Mutex<File>>
    - No sería recomendable si el recurso (File) no se cerrase después de cada acceso.
    */
}

impl Queue {

    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            rlock: Arc::new(Mutex::new(())),
            wlock: Arc::new(Mutex::new(())),

        }

    }

    fn enqueue(&self, mssg: &str) -> Result<(), &'static str>// Static es más óptimo aquí ya que no quiero crear un nuevo string en el heap cada vez que se produzca un error
    {
        // Validar el input
        let mbytes = mssg.as_bytes();
        let size = mbytes.len();

        if size > MAX_MESSAGE_SIZE {
            return Err("ERROR. El tamaño del mensaje supera el máximo permitido de 256KB")

        }

        //LOCK DE LECTURA. SI HAY ALGUIEN LEYENDO NO PUEDO PERMITIR QUE ESCRIBA (CONDICIONES DE CARRERA)
        let _rguard = self.rlock.lock().unwrap();

        // Abrir el archivo en modo append, creándolo si no existe.
        let mut file = OpenOptions::new().create(true).append(true).open(&self.file).expect("ERROR. No se puedo abrir el archivo.");
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("ERROR obteniendo el timestamp del sistema.").as_secs();

        
        // LOCK DE ESCRITURA PARA EVITAR QUE DOS PRODUCTORES ESCRIBAN A LA VEZ
        let _wguard = self.wlock.lock().unwrap();

        // Bit de estado. Inicialmente es 0 pero se pone a 1 cuando el mensaje es leído por un consumidor.
        file.write_all(&(0 as u8).to_le_bytes()).expect("Error escribiendo byte de estado");
        // Uso LITTLE ENDIAN porque es lo más habitual.
        file.write_all(&timestamp.to_le_bytes()).expect("Error escribiendo timestamp");
        file.write_all(&(size as u32).to_le_bytes()).expect("Error escribiendo tamaño");
        file.write_all(mbytes).expect("Error escribiendo mensaje");

        Ok(())
    }

    /// Obtener el último mensaje de la cola
    fn dequeue(&self, offset: u64) -> Option<(u64, String)> {
        // Obtengo lock de lectura.
        let _rguard = self.rlock.lock().unwrap();

        // Abrir archivo en modo lectura.
        let mut file = OpenOptions::new().read(true).write(true).open(&self.file).ok()?;
        // Establecer el offset donde empezaremos a leer
        file.seek(SeekFrom::Start(offset)).expect("Error posicionando el puntero del buffer");

        // Buffers de estado, tamaño y marca de tiempo
        let mut state_buffer = [0u8; 1];// buffer para el byte de estado.
        let mut timestamp_buffer = [0u8; 8];
        let mut size_buffer = [0u8; 4];

        // Leer el estado
        if file.read_exact(&mut state_buffer).is_err() {
            return None; // Archivo vacío
        }
        let state = u8::from_le_bytes(state_buffer);

        // Leer el timestamp (8 bytes)
        if file.read_exact(&mut timestamp_buffer).is_err() {
            return None; // Archivo vacío
        }
        let timestamp = u64::from_le_bytes(timestamp_buffer);
        
        // Leer el tamaño del mensaje (4 bytes)
        if file.read_exact(&mut size_buffer).is_err() {
            return None;
        }
        let size = u32::from_le_bytes(size_buffer) as usize;

        if state != 0 {
            // Antes de llamar recursivamente, liberamos el lock
            drop(_rguard);
            // MENSAJE YA PROCESADO, COMPUTAMOS EL OFFSET Y LEER EL SIGUIENTE
            let next_offset = offset + 1 + 4 + 8 + size as u64; // Estado (1 byte), tamaño (4 bytes), timestamp (8 bytes), mensaje
            return self.dequeue(next_offset);

        }


        if size > MAX_MESSAGE_SIZE {
            return None; // Seguridad extra: evitar lecturas corruptas
        }

        // Leer el mensaje. Gracias a la marca de tamaño puedo leer el mensaje usando read_exact.
        let mut msg_bytes = vec![0; size];
        if file.read_exact(&mut msg_bytes).is_err() {
            return None; // Error al leer el mensaje
        }

        let new_state = 1u8; // Marcamos como procesado
        file.seek(SeekFrom::Start(offset)).expect("Error posicionando el puntero del buffer para actualizar el estado");
        file.write_all(&new_state.to_le_bytes()).expect("Error actualizando el estado del mensaje");
        
        // Convertir el mensaje en String
        let message = String::from_utf8(msg_bytes).ok()?;

        Some((timestamp, message))
    }

    

}


use std::{thread, time::Duration};

fn main() {
    // Crear una nueva instancia de la cola con un archivo para almacenar los mensajes.
    let queue = Queue::new("message_queue.bin");

    // Encolar algunos mensajes.
    let messages = vec!["Mensaje 1", "Mensaje 2", "Mensaje 3", "Mensaje 4"];
    
    for msg in messages {
        if let Err(e) = queue.enqueue(msg) {
            eprintln!("Error al encolar el mensaje: {}", e);
        } else {
            println!("Mensaje encolado: {}", msg);
        }
    }

    // Simular un pequeño retraso para que los mensajes sean encolados antes de desencolar.
    thread::sleep(Duration::from_secs(2));

    loop {
        match queue.dequeue(0) {
            Some((timestamp, message)) => {
                println!("Mensaje desencolado: {} (timestamp: {})", message, timestamp);
            },
            None => {
                println!("No hay más mensajes en la cola.");
                break;
            }
        }
    }

    // Simulamos un pequeño retraso para permitir que el consumidor termine de procesar.
    thread::sleep(Duration::from_secs(2));


}