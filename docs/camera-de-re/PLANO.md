# Câmera de ré e sensor de estacionamento: plano

Branch: `ccr-e299790b-kv15r8`. A Parte 1 já está nele: compila (plugin e app em `cargo check`; UI em `tsc -b`,
`vite build` e os 464 testes do `vitest`) e ainda não foi testada no jogo. As Partes 2 e 3 são o que falta.

## O que o dono pediu

- Ao engatar a ré, a tela do WestPlay no painel vira a câmera de ré, como num carro ou caminhão de verdade.
- A câmera fica na **traseira da carreta** quando há carreta engatada; sem carreta, na traseira do caminhão.
- Linhas guia de distância desenhadas no chão (vermelho, amarelo, verde) que **viram junto com o volante**,
  mostrando para onde a traseira vai.
- Sensor de estacionamento: bipe lento longe (pi... pi... pi...), cada vez **mais rápido e mais alto** ao se
  aproximar, e tom contínuo quando está colado no objeto.
- Sistema otimizado, "top de linha".

## Limitação principal (decidida com o dono)

O SDK do ETS2/ATS não entrega imagem de câmera nem a distância até os objetos. Por isso a **imagem é virtual**:
o WestPlay desenha em 3D, a partir do mapa do próprio jogo (crates/mapdata: ruas na largura real, áreas
pavimentadas, prédios com altura), a vista para trás. Os sensores só percebem o que o WestPlay conhece: os
**prédios do mapa** e os **caminhões de outros jogadores no TruckersMP** (o SDK do TMP, `live.nearby`). Eles não
percebem a IA do trânsito, cercas, docas e paredes que vêm dentro dos prefabs das empresas, nem carretas
estacionadas. A imagem real do jogo e sensores "de verdade" ficam para a Parte 3, que exige investigar o jogo
rodando.

## Parte 1: feita, neste branch

### Plugin (`crates/plugin`)
- `reversing.rs` (novo): lê da configuração do caminhão e da carreta `wheel.position`, `wheel.steerable`,
  `wheels.count` e `hook.position`, e calcula o eixo dianteiro (o que esterça), o eixo traseiro (o meio dos que
  não esterçam), a quinta roda/pino rei, a traseira (último eixo mais um balanço estimado, `rear_overhang`) e a
  meia largura. Tem testes unitários, que não rodam no Linux (o crate é só Windows).
- `telemetry.rs`: registra `truck.wheel.steering` por roda (índices 0 a 15), `trailer.0.connected`,
  `trailer.0.world.placement` e a configuração `trailer.0`, e publica tudo na telemetria.

### Shared (`crates/shared/src/lib.rs`)
- `Telemetry` ganhou `wheel_steering` (radianos, esquerda positivo), `front_axle_z`, `rear_axle_z`, `hook_z`,
  `rear_z`, `half_width` e `trailer_connected`, `trailer_x/y/z`, `trailer_heading`, `trailer_hook_z`,
  `trailer_axle_z`, `trailer_rear_z`, `trailer_half_width`. Com isso, **`BRIDGE_VERSION` foi de 31 para 32**:
  plugin e app precisam ser instalados juntos.

### Host (`crates/host/src/lib.rs`)
- A telemetria vai para a página a cada 50 ms, e a cada 33 ms (30 por segundo, o `MAX_FPS` da captura) com a
  marcha em ré.

### UI (`ui/src/cabin/reversing/`, novo)
- `geometry.ts`: posição da traseira e da câmera, e `reversePath`, que simula a ré passo a passo com o volante
  como está: o caminhão como bicicleta (curvatura tan(esterço)/entre-eixos) e a carreta puxada pelo pino rei,
  sem deslizar de lado. Assim a carreta abre para o lado oposto e o caminho para antes do "canivete" (65°).
- `sensor.ts`: 4 sensores no para-choque (os dos cantos veem até 2 m e abrem para os lados; os do meio veem até
  3 m), com raios em leque contra os contornos dos obstáculos. `beepingFor`: silêncio além de 3 m; de 950 ms a
  110 ms entre bipes, acelerando ao quadrado; volume de 0,35 a 1; tom contínuo a 0,35 m. Zonas: vermelho abaixo
  de 0,8 m, amarelo abaixo de 1,6 m, verde além.
- `beeper.ts`: bipe em Web Audio (2150 Hz, 75 ms, bordas suaves), agendado no relógio de áudio. Ao chegar mais
  perto, o próximo bipe já vem no ritmo novo.
- `cameraStyle.ts`: estilo MapLibre "de câmera": céu com neblina no horizonte, ruas de asfalto na largura real
  com faixas de borda, prédios em 3D, os caminhões do TMP como blocos, e uma versão noturna.
- `view.ts`: um segundo mapa MapLibre, só da câmera, com a câmera posta na traseira por
  `calculateCameraOptionsFromCameraLngLatAltRotation` (altura de 3,6 m na carreta e 1,6 m no caminhão, inclinada
  42°/36° para baixo, campo de visão vertical de 78°). A imagem é **espelhada**, como numa câmera de ré real.
  Também desenha as linhas guia no chão em perspectiva, com marcas de 1, 2 e 3 m, e uma linha tracejada reta.
  Interpola a telemetria entre as duas últimas amostras, fica limitado a 30 fps (`mapFrameDue`) e não desenha
  com a tela fora de vista (os sensores continuam funcionando).
- `shown.ts`: aparece ao engatar a ré; sai 1,2 s depois de tirar a ré, na hora se o caminhão andar para frente a
  mais de 2,5 m/s, ou se a parte elétrica desligar.
- `ReversingCamera.tsx` e `reversing.css`: a tela inteira com a imagem; um selo "Câmera de Ré · Carreta" ou
  "Caminhão"; "Verifique os arredores"; um painel à direita com a vista de cima (carreta e cavalo no ângulo
  real, as faixas de cada sensor acesas), a distância ("1,2 m" ou "PARE") e o botão de silenciar, que vale até a
  próxima ré.
- `settings.ts`: as preferências salvas (`westplay.reversing`): câmera, bipes e linhas guia, todas ligadas por
  padrão.
- `CabinApp.tsx` mostra o `<ReversingCamera />` no overlay do Shell. `liveMap.ts` ganhou `coverLiveMap()`, que
  pausa o mapa do painel enquanto a câmera o cobre.
- Os textos ficaram em `state/strings.ts` (`reversing`), em português, inglês e turco.
- A telemetria em TS (`shared/host.ts`) ganhou os campos novos e `steering`.

## Parte 2: falta fazer (dá para testar fora do jogo)

1. **Página nos Ajustes**: em `apps/settings`, uma página `reversing` ("Câmera de Ré") com três chaves
   (`showCamera`, `beeps`, `guides`, ligadas a `useReversingSettings`) e a nota `showCameraNote`. Isso exige
   acrescentar `'reversing'` ao `SettingsPage` (settings.ts), uma `NavRow` na lista com um ícone de câmera em
   `glyphs.tsx`, e o case em `PageContent`.
2. **Prévia no navegador** (`?reverse`, em `reversing/preview.ts`): sem o jogo, simular uma telemetria de um
   caminhão com carreta dando ré devagar, com o volante indo de um lado para o outro, na direção de uma parede
   falsa (`cameraView().extraObstacles`). Serve para ver a tela, ouvir os bipes e tirar screenshots com o
   Playwright. O `NO_MAP` mostra a grade (`drawGrid`).
3. **Testes vitest**: `geometry.test.ts` (reto com esterço 0; esterço à esquerda leva a traseira para a
   esquerda; com carreta, a traseira vai para o lado oposto e o caminho para no `JACKKNIFE`; `bearingOf`),
   `sensor.test.ts` (parede a 1 m dá cerca de 1 m nos sensores do meio; nada além do alcance dá `null`; ritmo e
   volume de `beepingFor` nos extremos), `shown.test.ts` (entra na ré, segura 1,2 s em neutro, sai ao andar
   para frente).
4. **Conferir o visual** com a prévia: o espelhamento (linhas e painel do lado certo), a espessura das linhas
   guia, se o céu e o horizonte aparecem, e o desempenho (o mapa da câmera a 30 fps).
5. Rodar `npm run lint` e `npm run format:check` na UI, `cargo clippy` e `cargo fmt`.
6. **CHANGELOG.md**: uma entrada sobre a câmera de ré e os sensores, dizendo que a imagem é desenhada a partir do
   mapa.

## Parte 3: no jogo, com o dono (exige o PC com ETS2/ATS)

1. **Validar a telemetria nova** no log do plugin: `reversing: truck laid out as ...`, `reversing: trailer laid
   out as ...` e `the steering of N wheels registered`. Conferir em caminhões 4x2 e 6x4, com e sem carreta, e no
   ATS. Ver se `trailer.0.*` chega (senão, usar os nomes sem índice: `trailer.connected`,
   `trailer.world.placement` e a configuração `trailer`).
2. **Calibrar**: o sinal de `wheel_steering` (as linhas têm que curvar para o lado certo); o balanço traseiro da
   carreta (`rear_overhang`: o "PARE" tem que bater com o para-choque de verdade); e a altura e inclinação da
   câmera (`TRAILER_CAMERA_HEIGHT`, `TRAILER_TILT`, `VERTICAL_FOV`). Talvez seja preciso um ajuste fino por
   veículo nos Ajustes.
3. **Áudio**: confirmar que o bipe da página da cabine sai nas caixas do PC (o WebView da cabine não fica mudo)
   e decidir se ele deve passar pelo `cab_sound.js`, como os alto-falantes do caminhão.
4. **Desempenho**: medir os fps do jogo com a câmera aberta numa cidade com muitos prédios. Se pesar, reduzir o
   alcance (neblina mais perto) ou tirar o antialias.
5. **Imagem real e sensores reais (investigação)**: (a) usar a renderização dos retrovisores do jogo (render
   targets já vistos pelos hooks de `d3d.rs`) ou a câmera 0 do Modo Personagem como fonte da imagem; (b) raycast
   na física do jogo (PhysX, ver `docs/personagem/antigo/ray.rs.txt`, que travou no 1.61) para os sensores
   perceberem tudo (paredes, docas, carretas, IA). (c) Uma alternativa mais simples, pelo mapa: exportar as vagas
   de entrega dos prefabs (spawn points de descarga, com a rotação, em `crates/mapdata/src/prefab.rs`) e
   desenhar a vaga-alvo no chão da câmera.
