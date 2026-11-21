# Volante no celular — testes no jogo

Instalado em 2026-11-19 (app + plugin + `ui`, **bridge 47**: o app e o plugin só conversam se os dois forem desta
versão, e o jogo precisa ser reaberto para carregar o plugin novo). Nada disto foi testado no jogo ainda: só os
testes automáticos (curvas, filtro do giroscópio, mensagens, certificado) e a página num navegador de PC com sensor
simulado.

## Segunda versão (2026-11-19, noite) — reabra o jogo antes de testar

O primeiro teste (23:25) mostrou no log que o celular conectou e o plugin entregou os valores ao jogo
(`commands: phone wheel driving`). O que mudou depois do retorno do dono:

- [ ] **Tamanho dos pedais**: Ajustes → Pedais → *Largura do acelerador*, *Largura do freio* (cada um de 15% a 40%
      da tela) e *Altura dos pedais* (50% a 100%).
- [ ] **Voltar ao padrão** agora fica no topo do painel de Ajustes, sempre à vista (e também no fim da lista);
      pergunta "Voltar todos os ajustes do volante ao padrão?" antes de apagar.
- [ ] **Atraso do volante do jogo**: a suavização padrão caiu de 30% para 10%. Quem já mexeu nos ajustes guarda os
      30% antigos: baixe *Suavização* para 0–10% (ou toque em Voltar ao padrão) e compare.
- [ ] Depois de dirigir um pouco, o `plugin.log` mostra linhas `commands: phone wheel gives steering +0.40 (the
      game's wheel at +0.12)…`: o primeiro número é o celular, o segundo é onde o volante do jogo está. Se o do jogo
      fica bem atrás do celular por meio segundo ou mais, o atraso é do próprio jogo (o jogo gira o volante devagar
      para controles que não são volante): me mande essas linhas. O `app.log` mostra a cada 5 s quantos valores
      chegaram do celular (`[wheel] driving: N values in 5 s`): uns 300 é o normal (60 por segundo).

## Sétima versão (2026-11-20) — mais botões do caminhão

Segunda fileira embaixo do volante desenhado (todos são comandos do próprio jogo, com o perfil de hoje):
**◀** seta esquerda, **⚠** pisca-alerta, **▶** seta direita, **(P)** freio de mão, **Farol**, **Alto** (farol
alto), **Ignição** (a chave na primeira posição: a parte elétrica), **Câmera ⟲** (volta a câmera para o lugar
certo) e **Câm. segue** (a câmera segue o volante).

- [ ] Cada botão faz o mesmo que a tecla do jogo.
- [ ] Os botões acendem com o estado do jogo: setas em verde (piscando junto), freio de mão em vermelho, farol, alto
      e ignição em amarelo. Se o pisca-alerta piscar as duas setas, é normal.
- [ ] **Farol**: cada toque passa para a próxima posição do farol (como a tecla do jogo: apagado → lanterna → farol).
- [ ] **Câm. segue**: diga se liga e desliga com um toque, ou se só funciona enquanto o botão está apertado (não deu
      para saber pelo arquivo de controles).
- [ ] App e plugin novos (os comandos ficam no plugin): **reabra o jogo** para valer.

## Sexta versão (2026-11-20) — a página não gira mais

Antes, ao girar muito o celular, o navegador virava a página (para em pé ou de cabeça para baixo). Agora:

- [ ] **Android em tela cheia**: a página fica travada no lado em que começou (não só "deitada", que ainda deixava
      virar de cabeça para baixo). Gire o celular bastante para os dois lados: a página não se mexe.
- [ ] **Sem tela cheia / iPhone**: o navegador ainda pode virar, mas a página se desvira sozinha e continua do mesmo
      jeito na tela do celular. Gire até ficar em pé: os pedais e botões continuam no mesmo lugar e funcionando
      (tocar no alto do pedal = pedal todo). **Se aparecer de cabeça para baixo ou de lado errado, me avise** (cada
      celular conta o giro de um jeito; não deu para testar num celular de verdade).
- [ ] A direção não muda com isso (o sensor conta pelos eixos do próprio celular).
- [ ] Recarregue a página do volante no celular para pegar esta versão (só a página mudou; o app e o plugin são os
      da quinta versão).

## Quinta versão (2026-11-20, madrugada) — o volante do jogo acompanha o celular; tela cheia

**Por que o volante do jogo não acompanhava:** o log do teste mostrou que o perfil do jogo usa direção *relativa*
(`c_relatsteer 1`, o padrão para teclado e controle): o jogo entende o valor da direção como *velocidade* de giro,
não como posição. Com o celular em −0,04 o volante do jogo foi andando sozinho de −0,12 até −0,20, e uma hora ficou
parado no fim do giro com o celular no centro. Agora o plugin lê a cada quadro onde está o volante do jogo e gira
ele até o ângulo do celular (mais rápido quanto mais longe, e para quando chega).

- [ ] Virar o celular e o volante do caminhão ir até o mesmo ponto e **parar ali** (não continuar girando sozinho).
- [ ] Voltar o celular ao centro e o volante do caminhão voltar ao centro.
- [ ] Ainda tem atraso? O máximo que o jogo gira por segundo é o do teclado: se for lento, aumente a
      **sensibilidade da direção** nas opções de controle do jogo e teste de novo.
- [ ] Ajustes → Direção → **Direção no jogo**: *Teclado/controle* (padrão) ou *Volante*. Use *Volante* só se um dia o
      perfil do jogo for de volante (direção absoluta); com o perfil de hoje, *Volante* deixa o caminhão girando sozinho.
- [ ] O `plugin.log` mostra `commands: phone wheel at +0.30 relative gives steering … (the game's wheel at +0.28)`:
      o primeiro número é o celular, o último é o volante do jogo; eles devem ficar perto um do outro.
- [ ] **Tela cheia**: botão ⛶ no topo (liga/desliga) e Ajustes → Acessibilidade → *Preencher a tela*. No Android a
      página ocupa a tela toda e volta para tela cheia no próximo toque se sair. No iPhone o botão explica como pôr o
      Volante na tela de início, de onde ele abre em tela cheia e deitado.
- Testei e desfiz: subir o `c_jrspeed` do perfil (é a velocidade da câmera com controle, não da direção). Os
  `controls.sii` dos dois perfis do ETS2 voltaram idênticos ao backup `controls.sii.antes-volante`.

## Terceira versão (2026-11-19, noite) — ligar/desligar e marchas

Embaixo do volante desenhado: **Ligar/Desligar**, a marcha atual e os botões **R**, **N** e **D**. São os comandos
do próprio jogo (`ignitionstrt`/`ignitionoff`, `gearreverse`/`gear0`/`geardrive`), como o app Comandos do celular.

- [ ] Com o motor desligado, o botão mostra **Ligar**: segurar dá a partida (fica girando a chave pelo menos 1,5 s,
      ou enquanto você segura). Ligado, vira **Desligar** (borda vermelha) e um toque desliga.
- [ ] O botão muda sozinho quando o motor liga ou desliga por outro meio (teclado, celular do jogo).
- [ ] **D** engata para frente e **R** a ré; **N** põe em neutro. A marcha aparece ao lado (D1, D2…, R1, N), em
      vermelho na ré.
- [ ] Com câmbio **manual** no jogo, D/R/N podem não fazer nada (são os comandos do câmbio automático do jogo): diga
      qual câmbio você usa.

## Como abrir

1. Abra o MaxDevMod no PC → aba da tela → **Seu celular** → QR code **Volante**.
2. O endereço é `https://<ip do PC>:47801/wheel.html#<chave>`. Na primeira vez o navegador avisa do certificado:
   toque em **Avançado → Continuar** (é um certificado que este PC mesmo criou).
3. Toque em **Tocar para começar** (no iPhone ele pede permissão de movimento: aceite).
4. Segure o celular deitado, como um volante, tela virada para você.

- [ ] O QR code do Volante aparece no app do PC (se não aparecer: o HTTPS não subiu — ver o log, linhas `[wheel]`).
- [ ] Android/Chrome: abre depois do aviso, e o ponto fica verde "Conectado · N ms".
- [ ] iPhone/Safari: abre depois do aviso. **Se ficar "Conectando…" para sempre**, é o Safari recusando o WebSocket
      com certificado próprio: abra `https://<ip>:47801/volante.cer` no Safari, instale o perfil (Ajustes → Geral →
      VPN e Gerenciamento de Dispositivos) e ative a confiança total em Ajustes → Geral → Sobre → Ajustes de
      Confiança de Certificados ("MaxDevMod Volante (este PC)"). Depois recarregue a página.
- [ ] Se o celular abrir pelo endereço `http://…:47800/wheel.html`, a página mostra o botão "Abrir pelo endereço
      seguro" e ele leva para o HTTPS.

## Direção

- [ ] Girar o celular para a **direita** vira o caminhão para a **direita**. Se for ao contrário: Ajustes →
      **Inverter direção** (e me avise, para eu trocar o padrão).
- [ ] O volante desenhado no meio gira junto e no mesmo sentido do volante do caminhão.
- [ ] "Centralizar" zera a direção na posição em que o celular está.
- [ ] Sem atraso perceptível entre girar o celular e o caminhão virar (o número em ms no topo é ida e volta pela
      rede; abaixo de ~30 ms é bom).
- [ ] Ajustes mudam na hora: sensibilidade, giro até o fim, suavização, zona morta, curva.
- [ ] Ao chegar no fim do giro o celular vibra (Android).

## Pedais

- [ ] Acelerador (direita) e freio (esquerda): tocar mais **alto** no pedal = mais força; o pedal "desce" na tela e
      mostra a %.
- [ ] Os dois pedais ao mesmo tempo funcionam (dois dedos).
- [ ] Soltar o dedo solta o pedal na hora.
- [ ] Modo **Inclinar o celular**: inclinar para frente acelera, para trás freia (se for ao contrário: Inverter
      inclinação). "Centralizar" marca a posição neutra.
- [ ] Modo **Liga/desliga**: qualquer toque = pedal todo.
- [ ] Sensibilidade e curva de cada pedal mudam a resposta.
- [ ] O caminhão dá ré com o freio parado como no teclado (o jogo trata o freio/ré como sempre trata).

## Segurança (tudo solta)

- [ ] Bloquear a tela do celular, trocar de app ou fechar a aba: o caminhão para de virar/acelerar na hora.
- [ ] Desligar o Wi-Fi do celular com o pedal apertado: solta em menos de meio segundo.
- [ ] Fechar o app do PC com o pedal apertado: solta.
- [ ] Com o volante do celular **fechado**, o teclado / volante de verdade funcionam normalmente (direção,
      acelerador e freio). **Importante**: se o teclado parar de funcionar com o plugin novo, me avise.
- [ ] Com o celular conectado, o teclado ainda funciona junto? (Esperado: sim, o jogo soma os dois.)

## Acessibilidade e ajustes

- [ ] Canhoto: os pedais trocam de lado.
- [ ] Botões grandes e Alto contraste mudam a tela.
- [ ] Os ajustes ficam salvos ao reabrir a página; "Voltar ao padrão" volta tudo.
- [ ] Vibração liga/desliga e força (iPhone não vibra pelo navegador — normal).

## Se algo der errado

Já conferido no PC (2026-11-19, sem jogo): a página e o `volante.cer` abrem pelo HTTPS, o WebSocket do volante
conecta e responde em ~1 ms, e uma chave errada é recusada. O certificado fica em
`%LOCALAPPDATA%\app.westplay\volante` (apagar a pasta faz o app criar outro).

O log do app mostra `[wheel] … connected`, `[wheel] the phone went quiet: let go`;
o do plugin mostra `commands: input device registered (0), N inputs (3 analog)` e `commands: phone wheel driving` /
`let go`. Se o registro mostrar um número diferente de 0, o jogo recusou o dispositivo (aí nem os comandos do
celular funcionam) — me mande essa linha.
