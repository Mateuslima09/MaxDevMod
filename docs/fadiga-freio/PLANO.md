# Fadiga de freio: plano completo para outra IA executar

Pedido do dono (21/11/2026): os freios esquentam com o uso e, superaquecidos, perdem força até esfriar. Um HUD
próprio, separado da barra de teclas, com **dois discos de freio** (estilo BG Drive), a **temperatura em °C** ao lado
e a **cor mudando** conforme esquenta. Esfria **mais rápido com chuva e à noite**. **Sons** de caminhão "sem freio"
quando a força cai.

Este arquivo é o roteiro inteiro: o que ler, onde mexer, a ordem, como testar e instalar.

## ONDE PAROU (atualizado 21/11/2026)

**Feito:**
- 0.2: as mudanças da 2.4.4 foram commitadas (`19f7c7b`).
- **Fase A pronta no código** (versão **2.4.5 beta, ponte 54**):
  - `crates/shared/src/lib.rs`: `Telemetry::brake_temperature` e `Telemetry::input_brake`; `BRIDGE_VERSION = 54`.
  - `crates/plugin/src/telemetry.rs`: canais `truck.brake.temperature` e `truck.input.brake` (enum `Field`,
    `CHANNELS`, o `match` que grava, o `reset` e o `SNAPSHOT` inicial).
  - `crates/plugin/src/commands.rs`: `brake_log()` (log `brake fade: temp ...` a cada 1 s andando/freando) e
    `fade_probe()` (sonda A2: arquivo `Documentos\WestPlay\fade-probe.txt` com `-0.4` manda esse valor no eixo
    `abackward` quando o volante do celular não está dirigindo). Chamados em `next_axis()`.
  - Versão 2.4.5 em `Cargo.toml`, `app/tauri.conf.json`, `ui/package.json`, `CHANGELOG.md`.
  - `docs/fadiga-freio/TESTES.md` escrito (A1, A2, A3: o que o dono faz e manda).
  - Instalada 21/11 17:07, commit `0a23ba7`.
- **Teste A1 feito pelo dono (21/11):** `truck.brake.temperature` funciona (sobe freando, esfria parado), **mas trava
  em 200 °C** (o `max_brake_temperature` do jogo) e chega lá numa freada forte a 120 km/h. Medidas tiradas do log:
  aquece ~**4 °C/s por (pedal × m/s)**; esfria exponencial ~**0,0095/s** rumo a ~17–20 °C, **igual a qualquer
  velocidade**. Decisão: **modelo próprio** (seção 2.2), calibrado com esses números.
- **Fase B (modelo) feita:** `crates/host/src/brake_fade.rs` (novo): `BrakeFade::tick(level, telemetry, cargo_kg,
  weather, now) -> Fade { truck, trailer, strength }`, `strength_at()`, `Level`. Calor `HEAT = 1,15` (≈ jogo ÷ 3,5,
  para a fadiga vir só em descida longa), frio `COOL = 0,0095` + vento, chuva (`wetness`) até ×2,5, noite ×1,3; ar
  25/15/18 °C; carreta com disco próprio e carga; força cai na hora e volta 2 %/s; mínimo 15 %. 7 testes passando
  (`cargo test -p westplay-host brake_fade`; suíte do host toda verde: 254).
  Ligado em `crates/host/src/lib.rs` (junto do `auto_suspension.tick`, ~linha 960): calcula e **só escreve no log**
  do app a cada 2 s andando (`[brake fade] model: ...`). `last_weather` guardado no laço do `weather_watch`.
  **Ainda não tira força do freio nem passa nada ao plugin.**

- **Teste A2 feito pelo dono (21/11): FUNCIONA.** Com `abackward = -0.4` e o pedal no fundo, o jogo deu
  `truck.input.brake = 0.60` e `truck.effective.brake = 0.60` (sem a sonda: 1.00). O negativo é somado já no
  pedal (`input.brake` também cai), então o pedal real do jogador **não é mais visível** enquanto a sonda manda
  valor: na Fase C, guardar o valor mandado e calcular `pedal_real = input_brake - valor_mandado`. Seguir a
  seção 3.3 **caminho 1**. Arquivo `fade-probe.txt` apagado.

**Próximo passo:**
1. Dono faz **A3** (o modelo no log, ritmo certo?) e **A2** (a sonda do freio negativo, ainda não feita) de
   `docs/fadiga-freio/TESTES.md`. Ajustar `HEAT`/`COOL` com o log do A3 se ele achar rápido/devagar.
2. Enquanto isso dá para fazer a **Fase D (HUD)** sem depender de nada: pôr na ponte (`HostBlock`, seção 2.5)
   `brake_temp_truck`, `brake_temp_trailer`, `brake_strength`, `brake_hud`; o app grava o `Fade` neles a cada tick;
   o plugin desenha (seção 4). Ponte vira 55, instala app + plugin juntos.
3. Fase C pelo caminho 1 (seção 3.3): no `commands.rs`, trocar `fade_probe()` por `-(1 - strength) * pedal_real`
   (com `strength` vindo do app pela ponte, `HostBlock::brake_strength`), só quando `strength < 0,999`; nunca
   abaixo de -0,85; voltando a 1, mandar 0 uma vez.
4. **Ao terminar, tirar a sonda `fade_probe()`** do `commands.rs` e baixar os logs (1 s no plugin, 2 s no app) para
   só quando `strength < 1`.

Status da instalação/commit: Fase B **instalada** 21/11 17:29 (só o app mudou; ponte continua 54, o jogo não precisa
reabrir; backup `%LOCALAPPDATA%MaxDevModackups61121-172721`) e **commitada** localmente. Nada de push.

---

## 0. Antes de tudo (leia, é obrigatório)

### 0.1 Regras do dono

- **Só instalar local.** Nada de `git push`, nada de `gh release`, até o dono dizer "release" ou "enviar".
  Commits locais podem.
- **Nunca mexer em configurações de segurança do Windows** (Smart App Control etc.). Se um exe novo for bloqueado,
  voltar o backup e avisar o dono.
- **Tudo que não foi testado no jogo** vai para `docs/fadiga-freio/TESTES.md` (o dono testa e responde).
- Textos para o jogador em **português do Brasil**. Comentários no código em inglês, no estilo do repositório
  (frases completas, citando o pedido do dono com data: `owner 2026-11-21: "..."`).
- Nunca deixar o caminhão **sem nenhum** freio: sempre um mínimo (ver 3.3).

### 0.2 Estado do repositório quando este plano foi escrito

Pasta: `C:\Users\barce\Documents\ATS-CarPlay\westplay-base` (git, branch local). Havia mudanças **não commitadas**
das versões 2.4.3 e 2.4.4 (tela flutuante, câmera de ré, retrovisores):

```
M CHANGELOG.md  Cargo.lock  Cargo.toml  app/src/setup.rs  app/tauri.conf.json  crates/host/src/webview.rs
M crates/plugin/src/float_screen.rs  crates/plugin/src/rear_camera.rs  docs/tela-flutuante/TESTES.md  ui/package.json
```

**Primeiro passo:** rodar `git status`. Se essas mudanças ainda estiverem lá, commitar elas sozinhas antes de começar
(ex.: `git add -A; git commit -m "MaxDevMod 2.4.4 beta: virtual mirrors, floating screen fixes"`), para o trabalho do
freio ficar num commit separado e fácil de desfazer.

### 0.3 Mapa do código (o que já existe e serve)

| Peça | Arquivo | Para quê aqui |
|---|---|---|
| Ponte app↔plugin (memória compartilhada) | `crates/shared/src/lib.rs` (`BRIDGE_VERSION = 53`, `HostBlock`, `Telemetry`) | passar temperatura, ajustes e "quanto tirar de freio" |
| Canais de telemetria do jogo | `crates/plugin/src/telemetry.rs` (lista `CHANNELS`, enum `Field`) | registrar `truck.brake.temperature` |
| Freio que o jogador aplica | já lido: `truck.effective.brake` → `Telemetry::brake` (0..1) | aquecimento |
| Injeção de entrada no jogo | `crates/plugin/src/commands.rs` (dispositivo *semantical*, eixos `steering`, `aforward`, `abackward`) | tentativa de tirar freio |
| Volante do celular (usa `abackward`) | `crates/host/src/wheel.rs`, `HostBlock::wheel_brake` | **não quebrar**: o volante usa o mesmo eixo |
| Modelo que roda no app a cada tick | `crates/host/src/auto_suspension.rs` (`tick(...)`, constantes, testes) | **modelo a copiar** para `brake_fade.rs` |
| Chuva | `crates/host/src/weather.rs` (`Sky::Rain`, `wetness` 0..100, lido da memória do jogo) | esfriar mais com chuva (melhor que o limpador) |
| Hora do jogo | `Telemetry::game_time` (minutos desde o início; `% 1440` = hora do dia) + `multiplayer_time_offset` | esfriar mais à noite |
| HUD desenhado pelo plugin | `crates/plugin/src/hud.rs` (`HudQuad`, textura GDI pintada só quando muda, fade de entrada/saída) | **modelo a copiar** para `brake_hud.rs` |
| Sons pelo plugin | `crates/plugin/src/character/steps.rs` | tocar chiado/rangido |
| Sons pelo app | `crates/host/src/cab_sound.js` | alternativa |
| Ajustes do app | `crates/host/src/settings.rs` | liga/desliga, posição, tamanho, volume |
| Apps do celular | `ui/src/handset/*.tsx` (ex.: `SuspensionApp.tsx`, `FloatScreenApp.tsx`), `HandsetApp.tsx`, `Home.tsx`, `icons.tsx`, `prefs.ts` | app "Freios" |
| SDK | `tools/scs_sdk_1_15/include/common/scssdk_telemetry_truck_common_channels.h` linha ~317 | `truck.brake.temperature`: float, °C, **um valor para o caminhão todo** (não por roda) |

Regra do `CLAUDE.md` do projeto: antes de procurar no código, rodar `graphify query "<pergunta>"` (ou
`graphify explain "<conceito>"`); depois de mexer no código, `graphify update .`.

---

## 1. Fase A: sondas (baratas, decidem o resto). Uma instalação só.

### A1. Ler a temperatura do jogo

1. `crates/shared/src/lib.rs`, struct `Telemetry`: acrescentar `pub brake_temperature: f32` (°C) perto de `brake`.
2. `crates/plugin/src/telemetry.rs`:
   - enum `Field`: acrescentar `BrakeTemperature`;
   - `CHANNELS`: `("truck.brake.temperature", SCS_VALUE_TYPE_FLOAT, Field::BrakeTemperature),`
   - no `match` que grava cada campo (procurar `Field::Brake =>`), gravar o float em `brake_temperature`.
3. Log a cada 1 s (no plugin, com `log!`): `brake: temp {t:.1} C, pedal {b:.2}, speed {kmh:.0} km/h`.
4. Como a struct `Telemetry` faz parte da ponte, **subir `BRIDGE_VERSION` para 54** (app e plugin juntos).

### A2. Testar tirar freio com valor negativo

Em `crates/plugin/src/commands.rs`, só para a sonda:

- Uma constante de teste `FADE_PROBE: f32 = -0.4` e um interruptor no `HostBlock` (`brake_fade_probe: AtomicU32`,
  1 = ligado), ligado por um botão escondido no app (ou por um arquivo `fade-probe.txt` na pasta do app, o que for
  mais rápido).
- Com o interruptor ligado **e o volante do celular desligado** (`wheel_values()` retornou `None`), mandar
  `abackward = FADE_PROBE` todo frame; desligado, mandar 0 **uma vez** e largar (como o volante faz).
- Log: `fade probe: sent abackward {v:+.2}, effective brake now {b:.2}`.

**O que o dono faz:** liga a sonda, pisa o freio até o fim a 60 km/h e manda o log. Resultado:

- `truck.effective.brake` cai para ~0,6 → **caminho 1 funciona** (seguir 3.3 caminho 1).
- Fica em 1,0 (o jogo corta em 0 ou o mix usa "maior valor") → caminho 2 (ver 3.3).

Atenção: o mix do `controls.sii` do dono é do tipo `... | semantical.abackward?0`. Na linguagem de mix da SCS, `|`
costuma ser "o maior dos dois", então é provável que o negativo **não** funcione. Por isso a sonda vem antes de tudo.

### A3. Instalar e escrever o teste

- Rodar `docs/fadiga-freio/instalar-local.ps1` (ver seção 6).
- Criar `docs/fadiga-freio/TESTES.md` com os itens A1 e A2 (o que fazer no jogo, o que mandar de volta: o arquivo de
  log do plugin).
- Parar aqui e esperar o dono responder. **Não seguir para a Fase B sem os dois resultados.**

---

## 2. Fase B: o modelo de temperatura (`crates/host/src/brake_fade.rs`, novo)

Seguir o formato de `auto_suspension.rs`: struct com estado, `fn tick(&mut self, settings, telemetry, weather, now)
-> Fade`, constantes com doc-comment, testes `#[cfg(test)]` no mesmo arquivo. Registrar o módulo em
`crates/host/src/lib.rs` e chamar o `tick` no mesmo laço que chama a suspensão automática (procurar
`auto_suspension` em `lib.rs`).

### 2.1 Duas temperaturas (os dois discos do HUD)

- **Disco 1, caminhão:** se a sonda A1 mostrou que `truck.brake.temperature` sobe ao frear e desce parado, usar o
  valor do jogo e **só somar** o extra de chuva/noite (2.3). Se ficou parado em um valor, usar o modelo próprio (2.2).
- **Disco 2, carreta:** o jogo não dá. Sempre modelo próprio, com o mesmo freio (`brake`), peso maior com carga.
  Sem carreta engatada (`trailer_connected == false`), o disco 2 aparece apagado/cinza.

### 2.2 Modelo próprio (por disco), a cada tick `dt` em segundos

```
calor   = K_HEAT * brake * speed_ms * massa_fator          // energia no disco
frio    = K_COOL * (T - T_AMB) * (1 + K_WIND * speed_ms) * chuva_fator * noite_fator
T      += (calor - frio) * dt
T       = clamp(T, T_AMB, 700)
```

- `brake` = `Telemetry::brake` (`truck.effective.brake`, só o freio de serviço; freio-motor e retarder **não**
  esquentam o disco: descer a serra no retarder é o jeito certo de poupar o freio).
- `massa_fator`: 1,0 sem carga; com carga, `1 + peso_carga_t / 20` (o peso do frete está nos dados do frete,
  procurar `cargo_mass` / job em `shared/src/lib.rs`; se não houver, usar 1,4 com carreta engatada).
- `T_AMB` = 25 °C (15 °C à noite, 18 °C com chuva).
- Valores iniciais para ajustar no teste: `K_HEAT = 0,012`, `K_COOL = 0,010`, `K_WIND = 0,03`. Objetivo: descer
  uma serra de 6 % por 3 km só no freio leva a ~400 °C; parar 2 min esfria uns 100 °C.

### 2.3 Chuva e noite (pedido do dono)

- **Chuva:** `weather.rs` dá `Sky::Rain` e `wetness` (0..100). `chuva_fator = 1 + 1,5 * wetness/100`
  (sem `wetness`, `1,8` se `now == Rain`). Sem leitura do clima (mapa/versão sem layout): `1,0`.
- **Noite:** hora do dia = `(game_time + multiplayer_time_offset).rem_euclid(1440) / 60`. Entre 20h e 6h:
  `noite_fator = 1,3`.

### 2.4 Força do freio (curva de fadiga)

```
T <= 250 °C          -> força 100 %
250..400 °C          -> cai linear até 40 %
400..550 °C          -> cai linear até 15 %
>= 550 °C            -> 15 % (MÍNIMO, nunca menos)
```

Intensidade (ajuste do jogador): **leve** = limites +100 °C; **realista** = como acima; **dura** = limites −50 °C.
Histerese: a força só volta a subir quando a temperatura cair 20 °C abaixo do ponto onde caiu (evita ficar
piscando no limite).

Usar o disco mais quente dos dois para a força (com carreta, a média ponderada 40 % caminhão, 60 % carreta).

### 2.5 O que o app manda ao plugin (ponte)

Em `HostBlock` (`crates/shared/src/lib.rs`), com doc-comment em cada um:

```rust
/// Brake fade (owner 2026-11-21): the truck's and the trailer's disc temperatures, °C, f32 bits; NaN = none.
pub brake_temp_truck: AtomicU32,
pub brake_temp_trailer: AtomicU32,
/// How much of the player's brake is left, 0..1 as f32 bits (1 = all of it): the plugin takes the rest away.
pub brake_strength: AtomicU32,
/// The HUD: bit 0 on, bits 1-2 corner (0 bottom-left, 1 bottom-right, 2 top-left, 3 top-right),
/// bits 3-4 size (0 small, 1 medium, 2 large), bit 5 hidden by its key.
pub brake_hud: AtomicU32,
/// The fade sounds: 0..100 volume, 0 off.
pub brake_sound: AtomicU32,
```

Subir `BRIDGE_VERSION` (se a Fase A já subiu para 54, agora 55; ou fazer tudo numa subida só se der).

Testes do modelo (no `brake_fade.rs`): esquenta freando, não esquenta parado nem sem pedal, esfria mais rápido com
chuva e à noite, força nunca abaixo de 15 %, histerese, sem carreta o disco 2 é `None`.

---

## 3. Fase C: perder força de verdade (plugin)

### 3.1 Onde

`crates/plugin/src/commands.rs`, na função que monta os valores dos eixos todo frame (a que chama
`wheel_values()`).

### 3.2 Convivência com o volante do celular

- Volante ativo: ele manda o freio. Aplicar a fadiga **no valor dele**: `brake_do_volante * strength`. Simples.
- Volante inativo: é o freio do jogador (teclado/volante físico), que o plugin não vê nem muda direto. Ver 3.3.

### 3.3 Caminhos, conforme a sonda A2

**Caminho 1 (negativo funciona):** `abackward = -(1 - strength) * pedal_do_jogador`, onde `pedal_do_jogador` vem de
`truck.input.brake` (registrar esse canal também na Fase A: é o pedal antes do jogo, enquanto `effective` é depois).
Mandar só quando `strength < 0,999`; voltando a 1, mandar 0 uma vez e largar. Nunca passar de `-0,85`.

**Caminho 2 (negativo não funciona), em ordem de preferência:**

1. **Pesquisa na memória do jogo** do multiplicador de torque do freio do caminhão (como `weather.rs` acha o objeto
   do clima: procurar o valor de `brake_torque` do `physics.sii`/`chassis` na memória, mudar e ver o efeito).
   Mais caro, mas é o único que tira força ao vivo. Documentar o layout achado como `weather.rs` documenta o dele.
2. Se não achar: **fadiga só visual e sonora** (HUD, cores, sons, aviso no celular "freio superaquecido"), sem tirar
   força. Perguntar ao dono antes de decidir por esta.
3. **Não** usar `brake_torque_factor` do `physics.sii`: só muda ao reabrir o jogo, não serve para fadiga.

Em todos: log a cada 2 s quando `strength < 1`: `brake fade: truck {t1:.0} C trailer {t2:.0} C, strength {s:.2}`.

---

## 4. Fase D: o HUD dos discos (`crates/plugin/src/brake_hud.rs`, novo)

- **Copiar a estrutura de `hud.rs`:** `HudQuad` próprio (estado D3D próprio, `ID3DDeviceContextState`), textura
  BGRA premultiplicada pintada com GDI **só quando muda** (temperatura arredondada a 5 °C, cor, tamanho, canto),
  fade de entrada 200 ms / saída 170 ms. Registrar o desenho no mesmo ponto onde `hud.rs` desenha (procurar quem
  chama o `step` do HUD em `hooks.rs`/`lib.rs` do plugin).
- **Separado da barra de teclas:** estado e liga/desliga próprios (`HostBlock::brake_hud`). Esconder a barra não
  esconde os discos e vice-versa.
- **Desenho:** dois discos lado a lado (círculo com furos de ventilação e a pinça por cima), legenda pequena
  "CAVALO" e "CARRETA", temperatura em °C ao lado de cada um ("312 °C"). Fundo de vidro escuro igual à identidade
  do `hud.rs` (grafite `#101010`/`#25282C`, branco, prata `#AEB4BA`).
- **Cor do disco pela temperatura** (interpolar suave):
  - < 100 °C azul `#3A8DFF`; 100–250 verde `#2ECC71`; 250–350 amarelo `#F5C400`; 350–450 laranja `#FF7A00`;
    450+ vermelho `#D71920`;
  - perdendo força (`strength < 1`): **pulsar** o vermelho (via shader/opacidade, sem repintar a textura).
- **Lugar e tamanho escolhidos pelo jogador:** 4 cantos, pequeno/médio/grande (altura ~6 %, 9 %, 12 % da tela),
  margem que não cubra o painel do jogo nem a barra de teclas (padrão: embaixo à esquerda, pequeno).
- **Tecla própria** para esconder: usar o mecanismo de `keybinds.rs` (ver como as outras teclas do mod são
  registradas) e mostrar a tecla nas Configurações.
- Só aparece dirigindo (motor/elétrica ligados, fora de menu/pausa), como o HUD de dicas.

---

## 5. Fase E: sons e ajustes

### 5.1 Sons

- Com `strength < 0,9`: chiado de ar/freio em loop, mais alto quanto menor a força; acima de 450 °C freando: rangido
  de disco superaquecido.
- Tocar pelo plugin no mesmo fluxo de `crates/plugin/src/character/steps.rs` (ver como ele carrega e toca um wav).
- **Os arquivos de som o dono escolhe** (ElevenLabs/Pixabay, como nos minigames). Deixar nomes fixos
  (`brake_hiss.wav`, `brake_squeal.wav`) e funcionar em silêncio se não existirem. Registrar a origem em
  `THIRD_PARTY_NOTICES.md` quando o dono mandar.

### 5.2 Ajustes (app e celular)

- `crates/host/src/settings.rs`: `brake_fade_enabled` (padrão **desligado** até o dono testar), `brake_fade_level`
  (leve/realista/dura), `brake_hud_enabled`, `brake_hud_corner`, `brake_hud_size`, `brake_hud_celsius`,
  `brake_sound_enabled`, `brake_sound_volume`.
- Celular: app novo **"Freios"** em `ui/src/handset/BrakesApp.tsx`, no formato de `SuspensionApp.tsx`: os dois discos
  ao vivo com a temperatura e a força, e os ajustes acima. Registrar em `HandsetApp.tsx`, ícone em `icons.tsx`,
  entrada em `Home.tsx`, preferências em `prefs.ts`. Textos em português.
- Notificação no celular (`alerts.rs`) uma vez quando a força cair abaixo de 60 %: "Freio superaquecido: use o freio
  motor".

---

## 6. Compilar e instalar (sempre local)

Script pronto: `docs/fadiga-freio/instalar-local.ps1`. Ele:

1. para o `westplay.exe`;
2. faz backup do exe, do dll e da pasta `ui` instalados (com data no nome);
3. `npx tauri build` e depois `cargo build --release -p westplay-plugin` (o build do app copia um DLL velho por
   cima, por isso o plugin é compilado **depois**);
4. confere se o DLL novo contém um texto de prova (padrão `brake fade`);
5. copia exe + dll para `%LOCALAPPDATA%\MaxDevMod`, troca a pasta `ui` por `ui\dist`, e o dll para a pasta de
   plugins dos jogos que já têm o `westplay.dll`;
6. abre o app de novo.

```powershell
powershell -ExecutionPolicy Bypass -File docs\fadiga-freio\instalar-local.ps1
```

Antes de instalar: `cargo test -p westplay-host` e `cargo clippy --workspace` (e `npm test` em `ui` se mexer no
celular). Depois de instalar: o jogo **precisa ser reaberto** (ponte mudou).

Se o Windows bloquear o exe novo: rodar o script com `-Restaurar` (volta o último backup) e avisar o dono.

---

## 7. Ordem de trabalho (checklist)

- [ ] 0.2 commitar o que ficou da 2.4.4
- [ ] A1 temperatura no log + A2 sonda do freio negativo (+ `truck.input.brake`) → ponte 54 → instalar → TESTES.md
- [ ] **esperar o dono** com o log
- [ ] B `brake_fade.rs` com testes, ponte 55
- [ ] C perder força pelo caminho que a sonda decidiu
- [ ] D HUD dos discos
- [ ] E sons + ajustes + app Freios + notificação
- [ ] versão nova (`Cargo.toml`, `app/tauri.conf.json`, `ui/package.json`, `CHANGELOG.md` em inglês no estilo das
      outras entradas), instalar, atualizar `TESTES.md`, commit local
- [ ] **não** dar push nem release

## 8. Cuidados

- Mexer na ponte (`BRIDGE_VERSION`) instala app e plugin juntos; nunca instalar com outra sessão mexendo pela metade
  na mesma pasta.
- O volante do celular usa o mesmo eixo `abackward`: testar os dois juntos.
- TruckersMP: a fadiga mexe só no próprio caminhão; conferir que nada quebra no multiplayer (`truckersmp.rs`).
- Freio de estacionamento e freio de emergência por falta de ar **nunca** são afetados.
