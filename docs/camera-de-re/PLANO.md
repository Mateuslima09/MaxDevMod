# Câmera de ré e sensor de estacionamento: plano

A Parte 1 foi feita numa sessão na nuvem, no branch `ccr-e299790b-kv15r8` (commit `5191139`, em `origin`). Em
2026-11-13 ela foi trazida para a pasta local, **sem commit**, junto com as outras mudanças locais (TikTok, nota
fiscal, abastecimento), e a **Parte 2 foi feita** em cima dela. Tudo está instalado no PC e **ainda não foi testado no
jogo**. Falta a Parte 3, com o dono; o roteiro está em [TESTES.md](TESTES.md).

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

## Parte 1: feita (na nuvem), trazida para a pasta local em 2026-11-13

### Plugin (`crates/plugin`)
- `reversing.rs` (novo): lê da configuração do caminhão e da carreta `wheel.position`, `wheel.steerable`,
  `wheels.count` e `hook.position`, e calcula o eixo dianteiro (o que esterça), o eixo traseiro (o meio dos que
  não esterçam), a quinta roda/pino rei, a traseira (último eixo mais um balanço estimado, `rear_overhang`) e a
  meia largura. Tem testes unitários, que não rodam no Linux (o crate é só Windows).
- `telemetry.rs`: registra `truck.wheel.steering` por roda (índices 0 a 15) e lê a configuração da carreta, e
  publica tudo na telemetria. **Na junção com a pasta local** (2026-11-13), a carreta passou a vir dos canais que
  a nota fiscal/engate manual já registrava (`trailer.connected`, `trailer.world.placement`, guardados em
  `TRAILER`, e a configuração `trailer` ou `trailer.0`), em vez de canais `trailer.0.*` próprios: um canal só, para
  as duas coisas. A telemetria publica a carreta como engatada só com `connected` e `placed` (posição já dita).

### Shared (`crates/shared/src/lib.rs`)
- `Telemetry` ganhou `wheel_steering` (radianos, esquerda positivo), `front_axle_z`, `rear_axle_z`, `hook_z`,
  `rear_z`, `half_width` e `trailer_connected`, `trailer_x/y/z`, `trailer_heading`, `trailer_hook_z`,
  `trailer_axle_z`, `trailer_rear_z`, `trailer_half_width`. Com isso, a `BRIDGE_VERSION` subiu (31 para 32 no branch; **37 para 38**
  na pasta local, que já estava na 37): plugin e app precisam ser instalados juntos.

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

## Parte 2: feita em 2026-11-13 (na pasta local, sem commit)

1. **Página nos Ajustes**: `apps/settings` tem a página `reversing` ("Câmera de Ré"), com uma linha laranja na
   lista (grupo do Som) e o ícone `ReversingGlyph` (uma câmera) em `glyphs.tsx`. Ela tem três chaves ligadas a
   `useReversingSettings` (`showCamera` com a nota `showCameraNote`, `beeps` e `guides`). Conferido na prévia:
   as chaves guardam em `westplay.reversing`.
2. **Prévia no navegador**: `reversing/preview.ts` é ligada pelo `CabinApp` só fora do app. Abre com
   `cabin.html?reverse`: um 6x4 com carreta de 13,6 m dá ré a 1,4 m/s, mais devagar perto da parede, com o volante
   indo de um lado para o outro (0,28 rad, a cada 10 s), na direção de uma parede falsa a 14 m com um poste perto do
   canto direito. Para a 0,3 m, espera 2,5 s e recomeça. Variantes: `?reverse=truck` (sem carreta),
   `?reverse=night` e `?reverse=still` (parado a 1,2 m da parede, volante à esquerda), que podem ser combinadas
   (`?reverse=still,truck`). O passo da simulação é o mesmo das linhas guia: `geometry.ts` ganhou `backUp`
   (um passo da ré), que agora é usado também por `reversePath`.
3. **Testes vitest**: `geometry.test.ts` (reto; esquerda leva a traseira para a esquerda, -x; carreta para o lado
   oposto e o caminho para logo após o `JACKKNIFE`; a traseira e a câmera; `bearingOf`), `sensor.test.ts`
   (parede a 1 m; nada além do alcance, nada à frente; poste só nos sensores da direita; girando com a traseira;
   `beepingFor` nos extremos e sempre mais rápido e mais alto ao aproximar; `zoneOf`) e `shown.test.ts` (entra na
   ré; segura 1,2 s em neutro; ré, neutro e ré sem sair; sai ao andar para frente; sai sem a parte elétrica ou sem
   o jogo). São 20 testes novos, e a UI inteira passa: 496 testes. Os testes Rust do `reversing.rs`, que não rodavam
   no Linux, rodaram e passaram no Windows.
4. **Visual conferido na prévia**:
   - **Defeito da Parte 1 corrigido**: a imagem ficava preta. O `calculateCameraOptionsFromCameraLngLatAltRotation`
     do MapLibre 5.24, sem o argumento `roll`, devolve opções que fazem o `jumpTo` montar matrizes com NaN (o erro
     "Cannot read properties of null (reading '0')" a cada quadro). Agora o `roll` vai como 0 nos dois lugares
     (`view.ts`, `placeCamera`).
   - O espelhamento está certo: com o volante à esquerda, só o caminhão curva para a esquerda da tela e a carreta
     para a direita.
   - As linhas, as marcas e o painel dos sensores aparecem com a espessura e as cores certas. A distância desce até
     "PARE".
   - O selo "Carreta/Caminhão" só aparece depois do primeiro quadro (antes, mostrava "Caminhão" por um instante
     com carreta).
   - **Céu**: com as inclinações de 42°/36° e 78° de campo, a borda de cima fica a ~3° do horizonte. Quase não
     aparece céu, e a parede de 3,6 m enche o alto da imagem. Fica para calibrar na Parte 3.
   - **Desempenho**: não deu para medir na prévia (o painel do navegador estava em segundo plano e não roda
     `requestAnimationFrame`). Fica para a Parte 3, no jogo.
5. **Lint e formato**: `oxlint` sem avisos nos arquivos da câmera (foi corrigido um `set-state-in-effect` no
   `ReversingCamera.tsx`). O `prettier` está limpo nos arquivos da câmera e dos Ajustes. `cargo clippy` e
   `cargo fmt` estão limpos no `reversing.rs`, no `telemetry.rs` e no `host/src/lib.rs`. Os avisos que restam no
   repositório são de outras partes (Modo Personagem, TikTok, mapdata) e não foram mexidos.
6. **CHANGELOG.md**: entrada em "Unreleased".

### Depois do primeiro teste do dono (2026-11-13)
- **A câmera só abria acelerando**: ela olhava `gear` (`truck.engine.gear`). Num câmbio automático parado, essa
  marcha fica em 0 com a ré escolhida, e a ré só engata andando. Agora a ré vale pela marcha do painel
  (`displayedGear`) **ou** pela do motor: `inReverse` em `shown.ts`, e o mesmo teste no host para mandar a
  telemetria a 30/s. Tem teste em `shown.test.ts`. Só o app mudou (a ponte continua a 38, o plugin é o mesmo).
- **Imagem vazia no Eldorado**: é a limitação do mapa protegido (sem ruas nem prédios para desenhar). A grade do
  `NO_MAP` (branco a 22%, 1 px) quase não aparece na tela do painel.

### Sensores pela física do jogo (feito em 2026-11-13, a testar no jogo)
A pedido do dono ("o caminhão bate: usar essa lógica"), o item 5(b) da Parte 3 foi começado: `crates/plugin/src/ray.rs`.
- **A causa provável do fechamento de 2026-11-05**: não era o jeito de chamar. Conferido no exe com o `dumpbin`: o
  `PxScene::raycast` do jogo lê as bandeiras por ponteiro (`movzx ecx, word ptr [rax]`), como o código antigo
  fazia. O suspeito é **qual cena e quando**: o código antigo procurava "uma cena" entre ponteiros e lançava no fim
  do quadro, por conta própria.
- **Agora**: foi achada a função de raio do próprio jogo (ETS2 `0x1635990`, ATS `0x16967e0`, pelo formato do
  código): `(mundo, callback, lugar, direção, distância, camadas, filtro)`. Ela pega a cena em `mundo+0x178`,
  converte as coordenadas (origem em `mundo+0x110..0x11E`) e chama o slot `0x2f0` da cena com as bandeiras `0x407`.
  Ela fica **interceptada** (MinHook, `hook_now`), e os raios do WestPlay saem **só de dentro de uma chamada do
  próprio jogo**: logo depois do raio dele, no mesmo mundo e na mesma thread. Antes, confiro que a cena tem o raycast
  achado. Num mundo novo, um raio para baixo, de cima do caminhão, tem que achar o chão.
- **Trava**: `rays.txt` na pasta do jogo no WestPlay diz "testing" antes do primeiro raio e "ok" depois de 600 lotes.
  Se estiver em "testing" na próxima abertura (o jogo fechou), muda para "off" e os raios ficam desligados.
- **Sensores**: com a ré engatada, 12 raios (os 4 sensores da página, 3 raios cada), a 0,6 m do chão, 5 cm atrás da
  traseira (da carreta, se engatada). Camadas 1, 2 (mundo fixo e cenário), 4 (trânsito), 8 e 10 (carretas). Batida
  com a normal virada para cima (> 0,7) é chão e não conta. Vão na telemetria (`ray_sensors`, `rays`; **ponte
  39**), e a página usa o mais perto entre o mapa e a física (`withRays`, sensor.ts).
- Fora do TruckersMP. Testes: layout do callback, leitura dos bytes da chamada do ETS2, camadas, setores, raios.
- **Testes no jogo (2026-11-13)**: 1º, o jogo não fechou, mas o resultado era lido 4 bytes fora do lugar (o MSVC põe os
  campos de `PxLocationHit` depois do preenchimento de `PxQueryHit`: flags em 24, position 28, normal 40, distance 52).
  Corrigido. Também: a leitura fica mantida parado em ré (o jogo não lança raios parado), e fechar normalmente deixa a trava
  em "ok". 2º, funcionou, e a trava foi para "ok". Com a carreta o aviso vinha tarde: o fim da carreta era só estimado
  (`rear_overhang`). Agora ele é **medido**: 15 raios de 8 m atrás para a frente, só contra carretas (camadas 8 e 10),
  em 5 alturas e 3 posições. O ponto mais atrás de cada lote, e a mediana de 15 lotes se concordarem (até 0,3 m), uma vez
  por carreta engatada. Corrige `trailer_rear_z` na telemetria (sensores, câmera e linhas).
- **Falta, depois do primeiro teste**: o sensor de ponto cego (raios para os lados da cabine e da carreta, o
  trânsito na camada 4), e ver se o jogo lança raios a toda hora (o log conta). Se não lançar, achar outro momento
  seguro.

## Parte 3: no jogo, com o dono (exige o PC com ETS2/ATS)

1. **Validar a telemetria nova** no `plugin.log` (pasta `Documentos\WestPlay\logs`): `reversing: truck laid out as
   ...`, `trailer: its coupling pin at ...` seguido de `reversing: trailer laid out as ...`, e `telemetry: the
   steering of N wheels registered`. Conferir em caminhões 4x2 e 6x4, com e sem carreta, no ETS2 e no ATS. A
   carreta já usa os canais sem índice (veja a Parte 1).
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

O roteiro para o dono, item por item, com o que olhar no log, está em [TESTES.md](TESTES.md).
