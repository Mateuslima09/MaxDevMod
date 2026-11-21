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

**Resultado (21/11, log do dono):** a temperatura do jogo sobe freando e esfria parada, mas **trava em 200 °C** e chega
lá numa freada forte a 120 km/h. Não serve para a fadiga; o app calcula a própria (ver A3). ✅ feito.

## A3. O modelo do app (só log, não muda o freio ainda)

Instalado 21/11. O **log do app** (`Documentos\WestPlay\logs\app.log`) escreve a cada 2 s, andando:
`[brake fade] model: truck 312 C, trailer 280 C, strength 0.71, game 200 C`.

1. Desça uma serra longa **só no freio**: `truck` deve subir devagar (centenas de graus) e `strength` cair abaixo de 1
   depois de uns 250 °C.
2. Freadas normais na cidade: deve subir pouco e esfriar.
3. Pare uns 3 minutos: deve esfriar e `strength` voltar a 1.00.
4. Se der, repita na chuva ou à noite (esfria mais rápido).
5. Mande o `app.log` e diga se o ritmo parece certo (esquenta rápido/devagar demais?).

## A2. Tirar força do freio (teste)

**Resultado (21/11, log do dono):** com o arquivo (`-0.4`) e o pedal no fundo, `pedal 0.60, effective 0.60` (sem o
arquivo, o pedal no fundo dava 1.00). O valor negativo **é somado** ao pedal do jogador: dá para tirar freio por aqui.
✅ feito (arquivo apagado depois).

1. Na pasta `Documentos\WestPlay` (a mesma que tem a pasta `logs`), crie um arquivo de texto chamado
   **`fade-probe.txt`** com o conteúdo `-0.4`.
2. Espere uns 3 segundos (o log diz `brake fade probe: abackward -0.40 (on)`).
3. A uns 60 km/h, pise o freio **até o fim** por uns 3 segundos.
4. **Apague o arquivo** (o log diz `(off)`) e pise o freio de novo, até o fim, para comparar.
5. Mande o log e diga se sentiu o caminhão frear menos com o arquivo.

O que eu quero ver: com o arquivo, `effective` perto de **0.60** com `pedal 1.00` → dá para tirar freio por aqui.
Se ficar em **1.00** → o jogo não aceita, e vamos por outro caminho.

Obs.: com o volante do celular ligado, o teste não age (o volante manda no freio).

## C. A perda de força de verdade (2.4.6, ponte 55) — **reabra o jogo**

Instalado 21/11. A fadiga já vem **ligada** e no nível **realista** (ainda sem tela de opções no celular).

1. Desça uma serra longa com a carreta **só no freio**, uns 3 a 4 minutos, até o freio começar a ficar fraco.
2. Sinta se o caminhão freia menos. No `plugin.log`, as linhas `brake fade:` mostram `strength` (força que sobrou)
   e `told` (o quanto o plugin tirou).
3. Pare uns 3 minutos: o freio deve voltar ao normal aos poucos (`strength 1.00`).
4. Freio de estacionamento: deve funcionar normal, mesmo com o freio quente.
5. Se usar o volante do celular: o freio dele também deve enfraquecer.

O que me dizer: se ficou fraco demais, de menos, cedo ou tarde demais. Sons, HUD e opções ainda não estão nesta versão.
