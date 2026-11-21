# Fadiga de freio: testes pendentes

Instalado em 21/11/2026 (MaxDevMod 2.4.5 beta, ponte 54). **Reabra o jogo** depois de instalar.

O log do plugin fica em `Documentos\WestPlay\logs` (o arquivo mais novo). As linhas deste teste começam com
`brake fade`.

## A1. A temperatura do freio que o jogo dá

1. Dirija normalmente, depois desça uma serra **só no freio** (sem freio-motor, sem retarder) por 1 ou 2 minutos.
2. Pare o caminhão e espere 1 minuto (o log só escreve com o caminhão andando ou freando; pise de leve no freio
   parado para ele escrever a temperatura esfriando).
3. Mande o log.

O que eu quero ver: se `temp` **sobe** freando e **desce** parado (aí usamos a temperatura do jogo), ou se fica
parada num número (aí o app calcula a própria).

## A2. Tirar força do freio (teste)

1. Na pasta `Documentos\WestPlay` (a mesma que tem a pasta `logs`), crie um arquivo de texto chamado
   **`fade-probe.txt`** com o conteúdo `-0.4`.
2. Espere uns 3 segundos (o log diz `brake fade probe: abackward -0.40 (on)`).
3. A uns 60 km/h, pise o freio **até o fim** por uns 3 segundos.
4. **Apague o arquivo** (o log diz `(off)`) e pise o freio de novo, até o fim, para comparar.
5. Mande o log e diga se sentiu o caminhão frear menos com o arquivo.

O que eu quero ver: com o arquivo, `effective` perto de **0.60** com `pedal 1.00` → dá para tirar freio por aqui.
Se ficar em **1.00** → o jogo não aceita, e vamos por outro caminho.

Obs.: com o volante do celular ligado, o teste não age (o volante manda no freio).
