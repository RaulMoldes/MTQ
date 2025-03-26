# !/bin/bash

if [ $# -ne 2 ]; then
  echo "Uso: $0 <host> <puerto>"
  exit 1
fi

# Configuración
HOST=$1
PORT=$2
NUM_REQUESTS=10000  # Número total de mensajes a enviar
CONCURRENT_JOBS=50  # Número de conexiones concurrentes
MESSAGE="Test message for stress test"

send_message() {
    echo -n "enqueue $MESSAGE" | nc -q 1 $HOST $PORT
}

receive_message() {
    echo -n "dequeue" | nc -q 1 $HOST $PORT
}



echo "Realizando tests de carga en $HOST, puerto $PORT"
echo "Número de peticiones concurrentes: $NUM_REQUESTS"


for i in {1..1000000}; do
    send_message
    receive_message
    send_message
    receive_message
    receive_message

done;