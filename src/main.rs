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
        file.close();
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

        let _wguard = self.wlock.lock().unwrap(); // añadido para bloquear la escritura hasta que termine

        let new_state = 1u8; // Marcamos como procesado
        file.seek(SeekFrom::Start(offset)).expect("Error posicionando el puntero del buffer para actualizar el estado");
        file.write_all(&new_state.to_le_bytes()).expect("Error actualizando el estado del mensaje");
        
        // Convertir el mensaje en String
        let message = String::from_utf8(msg_bytes).ok()?;
        file.close();
        Some((timestamp, message))
    }


    
    fn purge(&self) {
        // Obtengo el lock de lectura
        let _rguard = self.rlock.lock().unwrap();

        // Abrir el archivo original en modo lectura
        let mut file = OpenOptions::new().read(true).open(&self.file).expect("No se pudo abrir el archivo");

        // Crear un archivo temporal para almacenar los mensajes no procesados
        let temp_file_path = format!("{}.temp", self.file);
        let mut temp_file = OpenOptions::new().create(true).write(true).open(&temp_file_path)
            .expect("No se pudo crear el archivo temporal");

        let mut offset = 0u64;

        // Buffers de estado, tamaño y marca de tiempo
        let mut state_buffer = [0u8; 1]; // buffer para el byte de estado
        let mut timestamp_buffer = [0u8; 8];
        let mut size_buffer = [0u8; 4];

        while let Ok(_) = file.seek(SeekFrom::Start(offset)) {
            // Leer el estado
            if file.read_exact(&mut state_buffer).is_err() {
                break; // Fin de archivo
            }

            let state = u8::from_le_bytes(state_buffer);

            // Si el mensaje tiene estado 1 (procesado), lo omitimos
            if state == 1 {
                let mut skip_buffer = [0u8; 8 + 4]; // timestamp + size
                if file.read_exact(&mut skip_buffer).is_err() {
                    break;
                }

                // Saltar el mensaje procesado
                offset += 1 + 8 + 4 + skip_buffer.len() as u64; 
                continue;
            }

            // Leer el timestamp y el tamaño
            if file.read_exact(&mut timestamp_buffer).is_err() {
                break; // Fin de archivo
            }

            if file.read_exact(&mut size_buffer).is_err() {
                break; // Fin de archivo
            }

            let size = u32::from_le_bytes(size_buffer) as u64;

            // Copiar el mensaje no procesado al archivo temporal
            temp_file.write_all(&state_buffer).expect("Error escribiendo estado");
            temp_file.write_all(&timestamp_buffer).expect("Error escribiendo timestamp");
            temp_file.write_all(&size_buffer).expect("Error escribiendo tamaño");

            // Leer el mensaje y escribirlo en el archivo temporal
            let mut msg_bytes = vec![0; size as usize];
            if file.read_exact(&mut msg_bytes).is_err() {
                break; // Fin de archivo
            }
            temp_file.write_all(&msg_bytes).expect("Error escribiendo mensaje");

            // Actualizar el offset para el siguiente mensaje
            offset += 1 + 8 + 4 + size;
        }

        let _wguard = self._wlock.lock().unwrap(); // Aqui faltaba tambien

        // Reemplazar el archivo original con el archivo temporal
        fs::remove_file(&self.file).expect("Error eliminando el archivo original");
        fs::rename(&temp_file_path, &self.file).expect("Error renombrando el archivo temporal");
        
        println!("Purga completada, archivo actualizado.");
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



#[cfg(test)]
mod tests {
    use super::*; // Importa todo lo que está en el scope global (tu implementación)

    use std::{fs, thread, time::Duration};
    use std::path::Path;

    // Test para la función enqueue
    #[test]
    fn test_enqueue() {
        // Crear una ruta temporal para el archivo
        let file_path = "test_queue_1.bin";

        let queue = Queue::new(file_path);

        // Probar encolar un mensaje
        let message = "Mensaje de prueba";
        assert!(queue.enqueue(message).is_ok(), "No se pudo encolar el mensaje");

        // Verificar que el archivo tiene datos después de encolar
        assert!(Path::new(file_path).exists(), "El archivo de mensajes no se creó");
        let file_content = fs::read(file_path).unwrap();
        assert!(file_content.len() > 0, "El archivo está vacío después de encolar un mensaje");

        // Limpiar el archivo después de la prueba
        fs::remove_file(file_path).unwrap();
    }

    // Test para la función dequeue
    #[test]
    fn test_dequeue() {
        // Crear una ruta temporal para el archivo
        let file_path = "test_queue_2.bin";

        let queue = Queue::new(file_path);

        // Encolar algunos mensajes
        let messages = vec!["Mensaje 1", "Mensaje 2", "Mensaje 3"];
        for msg in messages {
            queue.enqueue(msg).unwrap();
        }

        // Probar desencolar
        if let Some((timestamp, message)) = queue.dequeue(0) {
            assert_eq!(message, "Mensaje 1", "El primer mensaje no es el esperado");
            println!("Mensaje desencolado: {} (timestamp: {})", message, timestamp);
        } else {
            panic!("No se pudo desencolar el mensaje");
        }

        // Limpiar el archivo después de la prueba
        fs::remove_file(file_path).unwrap();
    }

    // Test para la función purge
    #[test]
    fn test_purge() {
        // Crear una ruta temporal para el archivo
        let file_path = "test_queue_3.bin";

        let queue = Queue::new(file_path);

        // Encolar algunos mensajes
        let messages = vec!["Mensaje 1", "Mensaje 2", "Mensaje 3"];
        for msg in messages {
            queue.enqueue(msg).unwrap();
        }
        
        
        let file_content_before = fs::read(file_path).unwrap();
        // Dejar que un mensaje sea procesado (cambiando su estado a 1)
        if let Some((_, _)) = queue.dequeue(0) {
            // Simular la purga, eliminando los mensajes procesados (con estado 1)
            queue.purge();
        }

        // Verificar que el archivo tiene solo los mensajes no procesados
        let file_content = fs::read(file_path).unwrap();
        assert!(file_content.len() < file_content_before.len(), "El archivo no se ha purgado");
        assert!(file_content.len() > 0, "El archivo está vacío después de purgar");
        println!("Archivo después de purgar tiene {} bytes", file_content.len());

        // Limpiar el archivo después de la prueba
        fs::remove_file(file_path).unwrap();
    }


}
