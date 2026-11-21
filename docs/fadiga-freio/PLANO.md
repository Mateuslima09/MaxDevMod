# Fadiga de freio (plano, não iniciado)

Pedido do dono (21/11): os freios esquentam com o uso e, superaquecidos, perdem força até esfriar. Um HUD próprio, à
parte da barra de teclas, com dois discos de freio (no estilo do BG Drive), a temperatura em °C ao lado e a cor
mudando conforme esquenta. Esfria mais rápido com chuva e à noite. Sons de caminhão "sem freio" quando a força cai.

Combinado: fazer **depois**, numa sessão nova (esta conversa ficou longa e cara por mensagem), de preferência depois
que o limite semanal zerar (25/11).

## 1. A temperatura do freio

- **Ver primeiro o que o jogo dá:** o SDK tem o canal `truck.brake.temperature` (°C, conferir em
  `tools/scs_sdk_1_15/include/common/scssdk_telemetry_truck_common_channels.h`). O `def/vehicle/physics.sii` do jogo
  tem `max_brake_temperature: 200`, `brake_cooling_rate` e `brake_friction_warming_rate`: o jogo já esquenta e esfria o
  freio, mas (a conferir) não tira força dele.
- **Se o canal servir:** usar a temperatura do jogo e só acrescentar os efeitos (chuva, noite, perda de força).
- **Se não servir:** um modelo próprio no app (`crates/host`, como a `auto_suspension.rs`):
  - aquece com `freio × velocidade` (energia jogada no disco; mais com carga, pelo peso do frete);
  - esfria sempre um pouco, mais com o vento (velocidade) e mais ainda **com chuva** e **à noite** (pedido do dono);
  - o freio-motor e o retarder **não** esquentam o disco: descer a serra no retarder é o jeito certo de poupar o freio.
- **Chuva:** o SDK não diz se chove. Pistas: o limpador de para-brisa ligado (`truck.wipers`), ou achar na memória do
  jogo (pesquisa, mais cara). Começar pelo limpador.
- **Noite:** pela hora do jogo (`game.time`, como o relógio da cabine já faz).

## 2. Perder força quando superaquece

- O `controls.sii` do dono soma `semantical.abackward?0` ao pedal de freio. **Testar** se o plugin consegue mandar um
  valor **negativo** ali e tirar freio (ex.: a 100% do pedal, o plugin manda −0,4 → freia 60%).
- Se o jogo cortar o valor em 0 (não deixar negativo), outro caminho: o `brake_torque_factor` do `physics.sii` (fixo,
  só muda ao reabrir o jogo), ou pesquisa na memória do jogo. Decidir depois do teste.
- Curva sugerida: até ~250 °C freio cheio; de 250 a 400 °C cai aos poucos até ~40% da força; acima disso, quase
  nada. Volta conforme esfria. (Números para ajustar no teste.)

## 3. O HUD dos discos

- Desenhado pelo plugin como a barra de teclas (`crates/plugin/src/hud.rs` `HudQuad`, a textura pintada só quando
  muda), mas **separado**: liga, desliga e se esconde sozinho.
- Desenho: **dois discos de freio** (dianteiro e traseiro, ou eixo do caminhão e da carreta), com a **temperatura em °C**
  ao lado de cada um.
- **Cor** pela temperatura: azul (frio) → verde (normal) → amarelo (quente) → vermelho → vermelho vivo, pulsando
  quando está perdendo força. Volta às cores conforme esfria.
- **Lugar e tamanho escolhidos pelo jogador**, num canto que não atrapalhe (ex.: embaixo à esquerda); tamanho pequeno,
  médio ou grande.
- **Esconder separado:** uma tecla própria e uma opção no app/celular, independente da barra de teclas.

## 4. Sons

- Quando a força cai: chiado de ar/freio e o rangido de disco superaquecido, mais forte quanto pior.
- Como nos minigames, o dono pode escolher gravações (ElevenLabs, Pixabay); registrar em `THIRD_PARTY_NOTICES.md`.
- Tocar pelo app (como os passos e a catraca) ou pelo plugin (o mesmo fluxo de som dos passos, `steps.rs`).

## 5. Ajustes (app e celular)

Fadiga ligada/desligada; HUD ligado, posição, tamanho; mostrar °C; sons ligados e volume; intensidade da fadiga
(leve/realista/dura).

## 6. Ordem sugerida

1. Ler `truck.brake.temperature` e escrever no log a cada segundo; o dono desce uma serra freando e manda o log.
2. Testar o freio negativo (`semantical.abackward`) com um valor fixo.
3. Com os dois respondidos: modelo de temperatura (do jogo ou próprio), chuva e noite.
4. HUD dos discos.
5. Perda de força, sons e ajustes.
6. Testes em `docs/fadiga-freio/TESTES.md`.

Os passos 1 e 2 são baratos e decidem o resto: vale fazê-los primeiro, numa instalação só.

## Cuidados

- Mexer na ponte (`BRIDGE_VERSION`) instala app e plugin juntos; não instalar enquanto outra sessão estiver com
  mudanças pela metade na mesma pasta.
- Nunca deixar o caminhão **sem nenhum** freio: um mínimo sempre (segurança da jogabilidade).
