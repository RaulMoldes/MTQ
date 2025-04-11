import subprocess
import time
import matplotlib.pyplot as plt
import numpy as np
from concurrent.futures import ThreadPoolExecutor
import argparse

def send_command(command, host="localhost", port=9090):
    """Envía un comando al servidor y devuelve la respuesta y el tiempo que tardó"""
    start_time = time.time()
    try:
        result = subprocess.run(
            ["echo", "-n", command],
            stdout=subprocess.PIPE,
            check=True
        )
        
        response = subprocess.run(
            ["nc", "-q", "1", host, str(port)],
            input=result.stdout,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True
        )
        
        elapsed = time.time() - start_time
        return response.stdout.decode('utf-8').strip(), elapsed
    except subprocess.CalledProcessError as e:
        print(f"Error ejecutando comando: {e}")
        return None, time.time() - start_time

def benchmark_enqueue(message_sizes, repetitions=5, host="localhost", port=9090):
    """Prueba de rendimiento para enqueue con diferentes tamaños de mensaje"""
    results = {}
    
    for size in message_sizes:
        message = "a" * size
        times = []
        
        for _ in range(repetitions):
            _, elapsed = send_command(f"enqueue {message}", host, port)
            times.append(elapsed * 1000)  # Convertir a milisegundos
            
        results[size] = times
        
    return results

def benchmark_dequeue(count, repetitions=5, host="localhost", port=9090):
    """Prueba de rendimiento para dequeue"""
    times = []
    
    for _ in range(repetitions):
        for _ in range(count):
            send_command("enqueue test_message", host, port)
            
        start_time = time.time()
        for _ in range(count):
            send_command("dequeue", host, port)
        total_time = time.time() - start_time
        times.append((total_time / count) * 1000)  # Tiempo promedio por dequeue en ms
        
        # Limpiar la cola
        send_command("purge", host, port)
        
    return times

def benchmark_concurrent_enqueue(message_size, concurrent_requests, host="localhost", port=9090):
    """Prueba de rendimiento para enqueue concurrente"""
    message = "a" * message_size
    command = f"enqueue {message}"
    
    results = []
    
    def worker():
        _, elapsed = send_command(command, host, port)
        return elapsed * 1000  # Convertir a milisegundos
    
    with ThreadPoolExecutor(max_workers=concurrent_requests) as executor:
        results = list(executor.map(lambda _: worker(), range(concurrent_requests)))
    
    return results

def plot_results(enqueue_results, dequeue_avg_times, concurrent_results, message_sizes, concurrent_requests):
    """Genera gráficos con los resultados"""
    plt.figure(figsize=(16, 12))
    
    # Gráfico 1: Tiempo de enqueue por tamaño de mensaje
    plt.subplot(2, 2, 1)
    
    for size, times in enqueue_results.items():
        plt.boxplot(times, positions=[size], widths=size*0.15)
    
    plt.title('Tiempo de Enqueue por Tamaño de Mensaje')
    plt.xlabel('Tamaño del mensaje (bytes)')
    plt.ylabel('Tiempo (ms)')
    plt.xscale('log')
    plt.grid(True, linestyle='--', alpha=0.7)
    
    # Gráfico 2: Tiempo promedio de dequeue
    plt.subplot(2, 2, 2)
    plt.bar(['Dequeue'], [np.mean(dequeue_avg_times)], yerr=[np.std(dequeue_avg_times)])
    plt.title('Tiempo Promedio de Dequeue')
    plt.ylabel('Tiempo (ms)')
    plt.ylim(0,2000)
    plt.grid(True, linestyle='--', alpha=0.7)
    
    # Gráfico 3: Distribución de tiempos en solicitudes concurrentes
    plt.subplot(2, 2, 3)
    plt.boxplot([concurrent_results[req] for req in concurrent_requests], 
                labels=[f"{req} req" for req in concurrent_requests])
    plt.title('Tiempos en Solicitudes Concurrentes (Enqueue)')
    plt.xlabel('Número de solicitudes concurrentes')
    plt.ylabel('Tiempo (ms)')
    plt.grid(True, linestyle='--', alpha=0.7)
    
    # Gráfico 4: Tiempo promedio por operación
    plt.subplot(2, 2, 4)
    
    enqueue_avg = np.mean([np.mean(times) for times in enqueue_results.values()])
    dequeue_avg = np.mean(dequeue_avg_times)
    concurrent_avg = np.mean([np.mean(times) for times in concurrent_results.values()])
    
    plt.bar(['Enqueue', 'Dequeue', 'Concurrente'], [enqueue_avg, dequeue_avg, concurrent_avg])
    plt.title('Tiempo Promedio por Operación')
    plt.ylim(0,2000)
    plt.ylabel('Tiempo (ms)')
    plt.grid(True, linestyle='--', alpha=0.7)
    
    plt.tight_layout()
    plt.savefig("benchmark_results.png")
    plt.close()
    
    print(f"Gráfico guardado como 'benchmark_results.png'")

def main():
    parser = argparse.ArgumentParser(description='Benchmark para sistema de cola')
    parser.add_argument('--host', default='localhost', help='Dirección del servidor')
    parser.add_argument('--port', type=int, default=9090, help='Puerto del servidor')
    parser.add_argument('--repetitions', type=int, default=5, help='Repeticiones por prueba')
    args = parser.parse_args()
    
    print("Iniciando benchmark de sistema de cola...")
    
    # Configuración de pruebas
    message_sizes = [10, 100, 1000, 10000]
    dequeue_count = 50
    concurrent_requests = [5, 10, 20]
    
    # Purgar la cola antes de comenzar
   # print("Purgando la cola...")
   # send_command("purge", args.host, args.port)
    
    # Ejecutar pruebas
    print("Ejecutando prueba de enqueue...")
    enqueue_results = benchmark_enqueue(message_sizes, args.repetitions, args.host, args.port)
    
    print("Ejecutando prueba de dequeue...")
    dequeue_avg_times = benchmark_dequeue(dequeue_count, args.repetitions, args.host, args.port)
    
    print("Ejecutando pruebas concurrentes...")
    concurrent_results = {}
    for req in concurrent_requests:
        print(f"  - Con {req} solicitudes concurrentes...")
        concurrent_results[req] = benchmark_concurrent_enqueue(100, req, args.host, args.port)
    
    # Generar gráficos
    print("Generando gráficos...")
    plot_results(enqueue_results, dequeue_avg_times, concurrent_results, message_sizes, concurrent_requests)
    
    print("Benchmark completado!")

if __name__ == "__main__":
    main()