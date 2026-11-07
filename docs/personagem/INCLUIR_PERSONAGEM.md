# Incluir um personagem novo no WestPlay (o padrão)

Este é o passo a passo para colocar um personagem novo (ou animações novas) no Modo Personagem. Ele segue o que foi
feito e testado com o homem (`wp`, depois trocado pelo tatuado, `wt`) e a mulher (`wm`) em 2026-11-05/06. Os scripts ficam em `tools/personagem/`.
Quando um passo tem número, use exatamente esse número: foi medido.

O caminho completo é: modelo 3D → Mixamo (esqueleto + animações) → Blender (juntar, padronizar e conferir) →
SCS (exportar e converter) → pasta do WestPlay → código (uma entrada por lugar) → teste no jogo.

---

## O padrão

| O quê | Padrão |
|---|---|
| **Altura do homem** | **1,80 m** do chão ao alto da cabeça: osso `mixamorig:Head` a **1,52 m**, olhos a **1,64 m** |
| **Altura da mulher** | **1,68 m**: osso `mixamorig:Head` a **1,46 m**, olhos a **1,56 m** |
| Como se mede | pelo osso da cabeça do Mixamo (fica onde o pescoço encontra a cabeça). Assim chapéu, boné e cabelo volumoso não entram na conta. O `juntar_personagem.py` escala sozinho. |
| Malha | uma só, de 6 a 12 mil triângulos, virada pra frente (−Y no Blender), pés no chão (z = 0), em T ou A |
| Textura | uma, 2048×2048, cor sem luz embutida; no jogo vai como TGA (o conversor recusa PNG) |
| Esqueleto (Mixamo) | **"2 Chain Fingers" (41 ossos)**: o polegar e o indicador servem para segurar o celular. A mulher e o tatuado usam este |
| Animações | **30 fps**, sem redução de quadros, **"In Place"** quando o Mixamo oferecer |
| Prefixo | 2 ou 3 letras `a-z0-9`, único (`wp`, `wm`, …). Cada parte do nome de uma unidade do jogo tem no máximo 12 caracteres: `mover.<prefixo>f_txwalk` |

### As animações (nome no Blender → nome no jogo)

| Blender (`juntar_personagem.py`) | Jogo (`anim/<nome>.pma`) | Mixamo (o que a mulher usa) | Obrigatória |
|---|---|---|---|
| `andar` | `walk` | Walking (In Place) | **sim** (é dela que sai o modelo: baixe *With Skin*) |
| `parado` | `idle` | Idle / Standing Idle | **sim** |
| `correr` | `run` | Running (In Place) | sim |
| `parar` | `stop` | Female Stop Walking (o script tira o deslocamento) | não |
| ~~`pular`~~ | ~~`jump`~~ | **fora do padrão** (2026-11-06): parado, o Espaço não faz nada. Nenhum pulo parado do Mixamo ficou bom ("Jump", "Jumping", cortados e emendados de vários jeitos: `TRANSICOES.md` 7.10–7.14) | — |
| `pular_correndo` | `jumprun` | Jump, a versão correndo (In Place) | não |
| `agachar` | `crouch` | Standing To Crouched (desce num joelho; de trás para frente, sobe: `agachar_completo.py`) | não |
| `andar_agachado`, `agachado_andar`, `agachado_parar` | `cwalk`, `cstart`, `csettle` | Crouched Walking (In Place), mais as transições feitas pelo `agachar_completo.py` e pelo `transicao_parar.py` | não |
| `celular` | `text` | Texting (pega o celular e digita) | não |
| `celular_andando` | `txwalk` | Texting And Walking (In Place) | não |

Se uma animação faltar, o personagem faz sem ela: para em vez de tocar o "parar", continua andando com o celular
aberto, e assim por diante. Só o pulo e o agachar precisam dos tempos no perfil (passo 8).

### Os arquivos de cada personagem (prefixo `xx`, pasta `<pasta>`)

No jogo, dentro de `model/westplay/<pasta>/`:

- `xx_player.pmd/.pmg` + `xx_player.tobj/.dds`: o personagem (o esqueleto dele é o de todos os outros arquivos);
- `xx_body`: o mesmo corpo sem a cabeça, para a 1ª pessoa;
- `xx_phone` / `xx_phbody`: com o celular na mão, o normal e o sem cabeça (só se tiver as animações de celular);
- `anim/<nome>.pma`: as animações.

---

## Passo a passo

Os comandos usam `B` para o Blender 3.6 portátil (`%LOCALAPPDATA%\Programs\Blender 3.6\blender.exe`; fique no
3.x, porque o SCS Blender Tools 2.4.1 não funciona no 4.x) e `T` para `westplay-base/tools/personagem`.

### 1. O modelo

Feito por IA (Tripo) ou à mão. Antes do Mixamo:

- reduzir para 6–12 mil triângulos, com mais detalhe no rosto e nas mãos (a redução por região, com grupos de
  vértices, deu certo no homem);
- refazer a textura do modelo alto para o baixo (*bake*): no homem, cor e normais com *cage extrusion* 0,004 e
  *ray distance* 0,012. Manchas escuras perto das sobrancelhas eram o *bake* pegando o lado de dentro;
- virar de frente (−Y). O personagem 2 veio de costas e precisou girar 180°;
- deixar na altura do padrão. Não precisa acertar no centímetro: o passo 3 corrige;
- exportar FBX com a textura embutida (*Path Mode: Copy*, *Embed Textures*) e enviar ao Mixamo.

### 2. Mixamo

1. Upload do FBX. No *Auto-Rigger*, marque queixo, pulsos, cotovelos, joelhos e virilha; o **Skeleton LOD** é
   **"2 Chain Fingers"**.
2. Baixe as animações da tabela, numa pasta só do personagem, cada uma com **FBX Binary, 30 fps, Keyframe
   Reduction: none** e **In Place** quando houver. A primeira (andar) é *With Skin*; as outras podem ser
   *Without Skin*.

### 3. Juntar e padronizar (Blender)

```
B -b --factory-startup --python T/juntar_personagem.py -- "<pasta dos FBX>" <homem|mulher> <nome> "<saída>_animada.blend" "andar=Walking.fbx;parado=Idle.fbx;correr=Running.fbx;parar=Female Stop Walking.fbx;pular=Jump.fbx;pular_correndo=Jump Running.fbx;agachar=Air Squat Bent Arms.fbx;celular=Texting.fbx;celular_andando=Texting And Walking.fbx"
```

O script faz sozinho:

- **a altura**: escala o modelo, o esqueleto e os movimentos de todas as animações juntos até o osso da cabeça
  ficar na altura do padrão. Ele mostra a altura antes e depois;
- **o parar no lugar**: tira o quanto o quadril anda para frente;
- **os pulos com o pé no chão** (`pe_no_chao.py`): em cada quadro, sobe ou desce o quadril para o pé mais baixo
  ficar no chão. Assim o pé nunca atravessa o chão nem flutua, e só a física do jogo levanta o corpo. Antes o
  quadril era só travado na altura de quem está em pé: no alto do pulo os pés ficavam a 0,85 m do chão ("parece
  que está escalando"), e no pulo correndo a perna do pouso entrava 13 cm no chão;
- e mostra (`AIR …`) **em que quadros o pé sai do chão e volta**: são esses os `takeoff` e `land` do passo 8.

### 3b. Reaproveitar animações de outro personagem (opcional)

Os esqueletos do Mixamo usam os mesmos nomes de ossos, então uma animação pode passar de um personagem para
outro. Basta o personagem novo ter o rig. Do Mixamo, então, só precisa sair o modelo com esqueleto (Download,
FBX Binary, T-pose, *With Skin*), que entra como `modelo=`:

```
B -b --factory-startup --python T/juntar_personagem.py -- "<pasta>" homem <nome> "<saída>_animada.blend" "modelo=<nome>.fbx"
B -b "<saída>_animada.blend" --python T/reaproveitar_animacoes.py -- "<outro>_animada.blend" "andar,correr,parado"
```

O script copia as rotações osso por osso e escala o quadril pela altura do quadril de cada um. Depois põe o pé
mais baixo no chão em todos os quadros, porque as pernas não têm o mesmo comprimento (sem isso, as animações da
mulher no homem afundavam os pés até 7 cm). Ossos que o outro não tem ficam de fora: os dedos, num esqueleto
"No Fingers". Os tempos do perfil (passo 8) são os do personagem de onde a animação veio. Para não deixar os dedos
esticados nessas animações: `B -b "<saída>_animada.blend" --python T/dedos_relaxados.py -- agachar 1 andar,correr,parado`
(os dedos ficam na pose do quadro 1 do agachar, levemente fechados, do começo ao fim). A corrida não tem o pé no
chão quadro a quadro, porque tem um instante com os dois pés no ar: só desce o ciclo inteiro até o ponto mais
baixo tocar o chão. Testado em 2026-11-06
com o agachar, o pulo, o pulo correndo, o parar e o celular da mulher no homem: tudo passa no conferidor (só
o bico da bota, no pulo correndo, desce 3,6 cm).

### 4. Conferir

```
B -b "<saída>_animada.blend" --python T/conferir_animacoes.py -- <homem|mulher>
```

Ele mostra, para cada animação: a duração, a **emenda** (se o fim encaixa no começo nas que repetem), quanto o
quadril **sai do lugar**, o **pé mais baixo** (se atravessa o chão ou flutua) e a malha. No fim escreve
`RESULTADO ok` ou a lista dos **PROBLEMA**s, cada um em português, dizendo o que fazer. O homem e a mulher passam
sem nenhum (2026-11-06).

### 5. Celular na mão (só se tiver `celular`)

```
B -b "<saída>_animada.blend" --python T/celular_na_mao.py -- "<celular.fbx>" "<saída>_celular.blend" "<pasta>/xx_phone_tex.png"
```

O celular sai com 14,7 cm e duas cores (tela escura, corpo grafite: sem ícones de nenhuma marca). Ele fica
deitado na palma, com a tela virada para os olhos, preso ao osso da mão direita. O script gera três imagens
(`_olho`, `_lado`, `_frente`) para conferir. Se a palma aparecer por cima da tela, aumente o `up` (o padrão é
0,031 m; com 0,024 a palma atravessa a tela). Ele precisa dos ossos do polegar e do indicador
("2 Chain Fingers").

### 6. Exportar para a SCS e converter

A pasta do projeto SCS é `personagem_scs\base` (ao lado do repositório; mude com `WESTPLAY_SCS_BASE`). A textura
vai em `base\model\westplay\<pasta>\xx_player.tga`:

```
B -b --factory-startup --python T/textura_tga.py -- <cor.png> "<base>/model/westplay/<pasta>/xx_player.tga"
B -b "<saída>_animada.blend" --python T/exportar_scs.py -- <nome> <pasta> xx_player "walk:andar,idle:parado,run:correr,stop:parar,jump:pular,jumprun:pular_correndo,crouch:agachar,text:celular,txwalk:celular_andando"
B -b "<saída>_animada.blend" --python T/exportar_scs.py -- <nome> <pasta> xx_body "" xx_player headless
B -b "<saída>_celular.blend" --python T/exportar_scs.py -- <nome> <pasta> xx_phone "" xx_player
B -b "<saída>_celular.blend" --python T/exportar_scs.py -- <nome> <pasta> xx_phbody "" xx_player headless
```

(A textura do celular vira TGA do mesmo jeito: `xx_phone_tex.tga` + `xx_phone_tex.tobj`.)

Depois converta (PowerShell). `linked_wp` é uma junção, criada uma vez, de `SCS Conversion Tools\rsrc\linked_wp`
para `personagem_scs\base`:

```
cd "$env:LOCALAPPDATA\Programs\SCS Conversion Tools\bin\win_x64\tools"; .\resconvert.exe -update -root linked_wp
```

Os arquivos do jogo saem em `rsrc\linked_wp\@cache`, junto com uma pasta `automat\`, que também vai. Para
**conferir o esqueleto**, os 8 bytes no deslocamento 24 de cada `.pmg` e no 12 de cada `.pma` têm de ser iguais
(`xxd -s 24 -l 8 -p x.pmg`). Se um for diferente, o modelo aparece congelado no jogo, sem nenhum erro.

### 7. Na pasta do WestPlay

Copie todo o `@cache` para `%LOCALAPPDATA%\WestPlay\personagem\`. **Com o jogo fechado**, o app refaz o mod
sozinho em até 30 s (desde 2026-11-06 ele também percebe quando só os arquivos do personagem mudam). O retrato
do cartão no app sai assim:

```
B -b "<saída>_animada.blend" --python T/retrato.py -- ui/public/characters/xx.png parado 60 <altura>
```

### 8. No código (uma entrada em cada lugar, na mesma ordem)

1. `crates/shared/src/lib.rs`: `CHARACTER_PEOPLE` ganha `"xx_walk"`. A ordem é a dos cartões no app.
2. `crates/setup/src/own_character.rs`: `PEOPLE` ganha um `Person { dir: "model/westplay/<pasta>", model:
   "xx_player.pmd", body: "xx_body.pmd", phone: "xx_phone.pmd", phone_body: "xx_phbody.pmd", prefix: "xx" }`.
3. `crates/plugin/src/character/person.rs`: `PEOPLE` ganha um `Profile`, com tudo que o plugin sabe da pessoa:
   - `eyes`: o osso da cabeça mais 45% até o topo dele (1,64 o homem, 1,56 a mulher, se estiverem no padrão);
   - `walk` / `run`: o ciclo (quadros − 1)/30 e os dois momentos em que cada pé toca o chão (o som do passo);
   - `jumprun` (o `jump`, parado, fica `None`: fora do padrão): `frame(n)` do começo, da saída do chão (`takeoff`) e do pouso (`land`), do `AIR` do
     passo 3. O tempo no ar da física é ~0,6 s; se a animação ficar muito mais tempo no ar, o plugin segura o
     último quadro antes do pouso até a física pousar;
   - `crouch`: o quadro em que chega embaixo e o quadro em que já está de pé;
   - `stop`: de onde começa (o último pé no chão) e o comprimento;
   - `phone`: para onde a animação parada volta (depois de pegar o celular) e o comprimento.
   O que ficar `None`, o personagem faz sem.
4. `ui/src/companion/useSettings.ts`: `CHARACTER_PEOPLE.people` ganha `{ picture: '/characters/xx.png', kind:
   'man' | 'woman' }`. O nome no cartão sai sozinho ("Homem 2", "Mulher 2").

Os testes (`cargo test`) conferem se as listas estão na mesma ordem e se os nomes cabem no jogo.

### 9. Testar no jogo

Instale o app e o plugin (veja `docs/personagem/TESTES.md`) e confira, nesta ordem: em pé, andar, correr, parar,
pular parado, pular andando, pular correndo, agachar, celular parado e andando, 1ª pessoa olhando para baixo, e
subir e descer do caminhão. O log do plugin (`Documents\WestPlay\logs\plugin.log`) mostra `shown <animação>` a
cada troca e `landed after X s` a cada pulo (perto de 0,6 s).

---

## Armadilhas (cada uma custou uma exportação)

- O operador de exportação do SCS Tools não roda sem janela. O `exportar_scs.py` chama a exportação por dentro
  e coloca o esqueleto em repouso antes, senão a malha sai na pose.
- Nomes de ossos: no máximo 12 caracteres `[a-z0-9_]`. O script renomeia os ossos e também os caminhos das
  animações.
- A textura começa com `//` e aponta para um `.tobj`; o conversor recusa PNG (use TGA).
- O personagem fica de frente para −Z no jogo girando o esqueleto 180° dentro da raiz SCS. Girar a raiz não
  adianta.
- Numa pasta de animação própria, o exportador escreve `Skeleton: "xx_player.pis"`. O certo é
  `"../xx_player.pis"`, e o `exportar_scs.py` já corrige.
- Unidades do jogo (`mover.xx_walk`): cada parte com 1 a 12 caracteres. Se uma passar, o jogo descarta o `.sii`
  inteiro ("is in wrong format" no `game.log.txt`).
- O corpo sem cabeça e o com celular **usam o esqueleto do personagem** (`.pim` apontando para `xx_player.pis`,
  sem `.pis` próprio). Com esqueleto próprio, mesmo com os mesmos ossos, ficam congelados.
- O app só refaz o mod com o jogo fechado.

## Licença

As animações do Mixamo podem ser usadas em jogos, mas não podem ser redistribuídas como arquivos soltos, e o
WestPlay é aberto. Por isso os arquivos convertidos ficam **fora do repositório e das releases**
(`%LOCALAPPDATA%\WestPlay\personagem`), e o mod só tem o personagem quando os arquivos estão no PC (veja
`PERSONAGEM_PROPRIO.md`, "Licença").

## Pendências conhecidas (2026-11-06)

1. **O tatuado (`wt`, 2026-11-06) entrou no lugar do primeiro homem (`wp`)**. Ele já tem tudo: o rig do Mixamo
   (41 ossos) e as animações reaproveitadas, o andar, correr e parado do primeiro homem (com os dedos levemente
   fechados, `dedos_relaxados.py`) e o resto da mulher. Os arquivos do primeiro homem continuam em
   `%LOCALAPPDATA%\WestPlay\personagem\model\westplay\character` (o mod não os usa mais) e em
   `Downloads\personagem_otimizado`. As animações do tatuado que vieram da mulher são de mulher: o "Female Stop
   Walking", o pular, o agachar e o celular. Se alguma parecer feminina demais, troque por uma masculina do
   Mixamo (`juntar_personagem.py` ou `reaproveitar_animacoes.py`).
2. **O cowboy (`wc`, personagem 3) entrou em 2026-11-06**, pasta `model/westplay/cowboy`, terceiro cartão. Rig
   do dono (`pesonagem 3/cowboy_mixamo.fbx`), com o esqueleto **"3 Chain Fingers" (49 ossos: polegar, indicador
   e anelar)**. Todas as 13 animações vieram do tatuado (passo 3b), e o anelar repete o indicador
   (`anelar_como_indicador.py`, depois do `reaproveitar`; sem isso ele ficaria esticado). O perfil é o do tatuado,
   e o celular usa `up` 0,042 (com 0,031 a palma aparecia na tela). Cenas: `pesonagem 3/mixamo/cowboy_animado.blend`
   e `cowboy_celular.blend`. **Falta o personagem 2** (o caminhoneiro, prefixo sugerido `wk`): só o rig do Mixamo.
3. **Licença do Mixamo**: para uma release com personagem, as animações precisam ser nossas ou CC0 (veja
   "Licença"). Hoje o personagem só existe no PC de quem tem os arquivos.
4. **Backup no repo privado `Mateuslima09/WestPlay-personagem`** (2026-11-07): a própria pasta `personagem_scs` é
   esse repo (cenas, `base\`, `convertido\`, `scripts\`), e os originais de Downloads (`Pesonagem novos`,
   `personagem_otimizado`: os FBX do Mixamo e as cenas `_animada`/`_celular`, a fonte de tudo) estão em
   `personagem_scs\fonte\`. Num PC novo, clone ele em `Documents\ATS-CarPlay\personagem_scs`. Depois de mexer num
   personagem, faça commit e push lá também (os `.blend1` ficam de fora).
5. Detalhes pequenos: do fim do "parar" para o "parado" o quadril sobe 2 cm (uma troca seca, quase invisível), e
   o andar do Mixamo repete o primeiro quadro no fim: pode dar um tranquinho a cada volta (não visto no jogo).
6. A lanterna está guardada (`FLASHLIGHT_ON = false`): luz num mover criado pelo plugin não acendeu
   (`INVESTIGACAO.md` §7.11).
