# Modo Personagem — investigação (fase 0)

Feita em 2026-11-04 no **ETS2 1.61** (o ATS está na 1.62), com o jogo aberto, lendo a memória e o executável.
Scripts usados (só leitura, salvo o teste de escrita abaixo): fora do repositório, na pasta temporária da sessão.

## 1. O que o perfil já tem

- `g_developer "1"` e `g_console "1"` no `config.cfg` do ETS2 e do ATS: a **câmera 0** (livre) já funciona.

## 2. A câmera que o jogo usa pra desenhar

- A posição com que o jogo desenha o mundo fica num `static` do executável (o que `crates/plugin/src/camera.rs`
  já acha pelo código): **ETS2 1.61, `exe+0x3693298`**, três `float` (x, y, z em metros).
- Ela é **escrita a cada quadro** pelo jogo (`exe+0x474cdc`, logo depois de `call 0x473a50`, que só monta uma
  caixa de ±10 000 m em volta dela): escrever nela de fora não segura.

## 3. Onde a câmera 0 guarda a própria posição — ainda não achado

- A posição aparece, exatamente igual, em **7 lugares** de memória gravável (o `static` acima e 6 cópias), todos
  como três `float` seguidos; 5 cópias são áreas de cálculo do quadro (com a caixa de ±10 000 m), 1 é um objeto
  no heap (`0x1b1e42800c0` nessa sessão do jogo).
- **Teste de escrita** (com autorização do dono): subir o `y` desse objeto 1 m. Escrito, e **desfeito pelo jogo
  em menos de 0,1 s**; a câmera não se mexeu. Também é uma cópia.
- Não está em `double`, nem em inteiro de 1/256 m, nem espaçada numa matriz. Conclusão: a câmera 0 guarda a
  posição de outro jeito (provavelmente relativa a uma origem, ou montada a partir de outro estado) e recalcula
  a posição "pronta" a cada quadro.
- A **direção** (pra onde olha) também não apareceu perto dessas posições quando a vista girou.

Próximo passo, se for por aqui: seguir o código que **monta** a posição (a função em volta de `exe+0x474cdc`,
`0x4746d0..0x475853`) até o objeto da câmera 0 e o formato em que ela guarda posição e direção.

## 4. Achado importante: o jogo já tem uma câmera de andar a pé

No executável do ETS2 existe a classe **`walking_camera_u`** ("walking_camera"), com os controles de fábrica,
já com teclas no perfil do dono (`controls.sii`):

| Controle | Tecla |
|---|---|
| `camwalk_for` / `camwalk_back` / `camwalk_left` / `camwalk_righ` | W / S / A / D |
| `camwalk_run` | Shift esquerdo |
| `camwalk_jump` | Espaço |
| `camwalk_crou` | Ctrl esquerdo |
| `camwalk_lr` / `camwalk_ud` (olhar) | mouse/joystick |

E textos de interface: `@@walk_mode@@`, `@@overlay_camera_walk@@`, `@@overlay_camera_crouch@@`,
`@@notification_walking_camera@@` (ao lado de `@@notification_orbit_camera@@`).

**Onde ela é usada**: os textos em volta são das cenas de **configurar o caminhão/reboque/ônibus** (concessionária,
garagem: `truck_configuration`, `bus_garage_scene`, `trailer_config_scene`), que alternam entre câmera de órbita e
câmera andando. Não há sinal de uso no mundo aberto.

(Separado: `use_adaptive_camera_height` e `camera_height_factor` são da câmera da cabine,
`vehicle_cabin_camera_u`, não da câmera andando.)

### Por que isso importa

Se der pra usar a `walking_camera_u` **no mundo**, o jogo já faz por nós: andar com WASD, correr, pular, agachar
e olhar com o mouse — provavelmente presa ao chão (numa garagem o chão é plano; no mundo, a testar). Seria bem
menos trabalho e mais estável que controlar a câmera 0 por fora.

### Como a cena de configuração liga a câmera andando (ETS2 1.61, endereços relativos ao exe)

Na função da cena em `0x11668d6..0x1166b98` (o `rsi` é a cena), ao trocar pra câmera andando:

1. **Cria**: `call 0x16cb1d0` (uma "fábrica", com um registro global indexado por um modo) devolve o objeto da
   câmera andando (`r15`); a cena marca `[r15+0x6a0]`.
2. **Posição de partida**: copia a transformação da câmera em uso (`[rsi+0x1f0]`, campos `+0x40..+0x5c`) pra
   `[rsi+0x460]`.
3. **Inicia**: `call 0x16cc000` com `rcx` = a câmera andando, `rdx` = a posição de partida, `r8` = o **mundo
   físico da cena** (`[rsi+0x568]`). Dentro, ela pega `[r8+0xd0]` e cria um **corpo físico em cápsula** (rotação
   0,7071 = 45°), do tamanho de uma pessoa: é assim que anda no chão e bate nas coisas — **colisão de verdade,
   pela física do jogo**.
4. **Põe pra funcionar**: `call 0x108680` com o lugar da câmera atual (`r14`) e a câmera nova.

O que falta pra usar isso na estrada:
- o **mundo físico do jogo** (o da estrada, onde os caminhões estão), no lugar do da garagem;
- o **lugar da câmera ativa do jogo** na estrada (onde ficam as câmeras 1–8 e a 0) e como trocar pra outra;
- a posição de partida: a porta do motorista (telemetria: posição da cabine e da cabeça);
- ver se os controles `camwalk_*` funcionam fora da cena de configuração (ou se o plugin os repassa).

Riscos: é chamar funções internas do jogo, no fio principal dele; um erro derruba o jogo (testar no modo
single player, com o jogo salvo antes). Cada versão do jogo tem endereços próprios: o plugin tem de achar tudo
pelo formato do código, como no GPS.

## 5. Caminhos possíveis daqui

1. **Usar a câmera andando do próprio jogo no mundo** (o mais promissor): achar como as cenas de configuração
   criam/ligam a `walking_camera_u` e fazer o plugin ligar a mesma câmera com o caminhão parado na estrada,
   posicionada na porta do motorista. Precisa confirmar que ela funciona fora da garagem (chão, colisão).
2. **Controlar a câmera 0** (o plano original): continuar a investigação do §3 até achar o estado dela, e o
   plugin passa a escrever posição e altura (e ler o mouse) a cada quadro.

## Registro

| Data | O que | Resultado |
|---|---|---|
| 2026-11-04 | câmera de desenho achada (`exe+0x3693298`) | ok, reescrita a cada quadro |
| 2026-11-04 | posição da câmera 0 (float, double, int, matriz) | não achada; 6 cópias, todas refeitas a cada quadro |
| 2026-11-04 | teste de escrita na cópia do heap | desfeito pelo jogo em < 0,1 s |
| 2026-11-04 | `walking_camera_u` e controles `camwalk_*` | existem; usados nas cenas de configurar o caminhão |
| 2026-11-04 | como a cena liga a câmera andando | fábrica `0x16cb1d0`, início `0x16cc000` (cápsula física), ativar `0x108680` |

## 6. Segunda rodada (2026-11-04, à noite): o que faltava pro caminho 1

Lendo o executável e a memória do ETS2 1.61.1.9 aberto (só leitura):

- **Classes do jogo pelo nome**: cada objeto tem `get_class` (função virtual 5: `lea rax, [descritor]; ret`); o
  descritor (no `.rdata`) aponta pro nome (`walking_camera`, `debug_camera`, `camera_manager`…), pra classe-mãe
  (`+0x18`) e pro tamanho (`+0x30`). Dá pra reconhecer qualquer objeto sem endereço fixo.
- **Onde as câmeras guardam a posição** (resolve o §3): `core_camera + 0x40`, 32 bytes — x, y, z locais
  (`float`) num **setor de 512 m**, os setores x e z (`int16`), e a rotação (quatérnio w, x, y, z). A câmera 0 é a
  **`debug_camera`** e só guarda a posição aí (sem outra cópia). Antes não aparecia porque x e z são locais ao setor.
- **Gerenciador de câmeras**: o objeto central do jogo (`game_ctrl`, no `static` `exe+0x36af7c8`) tem o
  `camera_manager` (`+0x960`), com a lista das câmeras em `+0x38`/`+0x40` (a 0 é a primeira).
- **Mundo físico da estrada**: `game_ctrl + 0x2da0`, criado pela mesma função que a garagem usa
  (`0x1653a50`, a partir do sistema de física em `+0x2d90`); `[mundo+0xd0]` é o sistema, como na garagem.
- **Câmera andando**: 30 funções virtuais (as de `core_camera`); a 13 (`+0x68`) é a atualização (`this`, segundos),
  a 29 (`+0xe8`) liga os controles dela (troca o contexto de entrada do jogo). As câmeras do caminhão e a 0 têm
  mais funções (35 a 61): por isso **não** dá pra pôr a câmera andando no lugar delas na lista (o jogo chamaria
  funções que ela não tem).
- O jeito escolhido pra versão de teste: o plugin cria a câmera andando como a garagem cria (fábrica, início com o
  mundo da estrada, ponteiro do jogo), liga os controles dela, e a cada quadro chama a atualização dela e põe a
  câmera 0 onde ela está — o jogo desenha pela câmera 0.
- Os mesmos formatos de código estão no **ATS 1.62** (início, mundo, objeto central: uma vez só cada).

Ainda **não testado no jogo**: se a câmera andando funciona fora da garagem (chão, colisão, WASD), se a câmera 0
aceita a rotação que pomos nela (ou recalcula pelo mouse), e se o jogo devolve o contexto de entrada sozinho.

| Data | O que | Resultado |
|---|---|---|
| 2026-11-04 | classes pelo nome (`get_class` → descritor) | ok, 1139 classes |
| 2026-11-04 | posição das câmeras (`+0x40`, setor de 512 m) | achada; câmera 0 = `debug_camera` |
| 2026-11-04 | mundo físico da estrada (`game_ctrl+0x2da0`) | achado, mesmo tipo do da garagem |
| 2026-11-04 | versão de teste do plugin (F8) | testada: altura ok |
| 2026-11-04 | 1º teste no jogo (ETS2) | altura ok; o andar ia sempre pra direção do início (o mouse é da câmera 0) e o caminhão pegava o WASD |
| 2026-11-04 | 2ª versão: rumo da câmera andando (`+0x66c`) copiado da vista da câmera 0 a cada quadro; WASD, Shift, Espaço e Ctrl escondidos do jogo e dados aos controles dela (`+0x3f0`…, estado `+0xf8` = 2 apertado) | funcionou: anda pra onde se olha, caminhão parado |
| 2026-11-04 | 3ª versão: F8 já põe a câmera 0 (tecla 0 mandada pra janela do jogo) e ao sair volta pra cabine (tecla 1); mouse a 50% enquanto anda (gancho do DirectInput), `-` e `=` mudam, guardado em `Documents/WestPlay/personagem.txt` | funcionou |
| 2026-11-04 | Página Personagem no app (ligar, sensibilidade), `g_developer` automático | instalada |
| 2026-11-04 | Voltar só perto da porta do motorista (F8 a até 1,5 m dela, medido no chão); segurar F8 2 s volta de qualquer lugar | instalada; falta testar |
| 2026-11-04 | F8 que voltava na hora ("apertar duas vezes"): a hora do aperto que entrou pela porta não era apagada; e teclas de câmera (1–9, 0) a pé encerravam a caminhada | corrigidos: hora apagada ao sair; teclas de câmera escondidas do jogo enquanto anda; falta testar |

### 1º teste no jogo e o que mudou

- **O mouse é da câmera 0**: ela gira a vista sozinha (refaz a rotação dela a cada quadro); a câmera andando ficou
  com o rumo do início (`+0x66c`, radianos 0..2π; `+0x670` é olhar pra cima/baixo). Agora o plugin põe nela o
  rumo pra onde a câmera 0 olha antes de cada passo, e copia de volta só a posição.
- **Os controles da câmera andando**: objetos em `+0x3f0`, `+0x400`, … `+0x450` (frente, trás, esquerda,
  direita, correr, pular, agachar), cada um com o estado em `+0xf8` (2 = apertado, 0 = solto). Com as teclas
  escondidas do jogo (pra o caminhão não virar a roda, acelerar ou frear), o plugin aperta esses controles ele
  mesmo, pela tecla de verdade.

### Pra quem instala o WestPlay

- O plugin já vai pro jogo sozinho (a Configuração do app instala o `westplay.dll` na pasta `plugins` do jogo).
- A câmera 0 precisa de `uset g_developer "1"` no `config.cfg` do jogo (em Documentos). Com o Modo Personagem
  ligado no app, o app faz isso sozinho, mas só com o jogo fechado (aberto, o jogo reescreve o arquivo ao sair):
  vale da próxima vez que o jogo abrir. O console (`g_console`) não é necessário.
- As teclas de câmera que o F8 aperta são as de fábrica (0 = câmera 0, 1 = cabine); quem mudou essas teclas nos
  controles do jogo precisa apertar a câmera 0 antes.

### O pulo (a investigar)

- Teste do dono (2026-11-04): andar, correr e agachar funcionam; **o pulo (Espaço) não**.
- O plugin aperta o controle `camwalk_jump` como os outros (estado 2 em `+0x440` → `+0xf8`). Mas, no ETS2 1.61, o
  código da câmera andando só cita `+0x440` no construtor e no destrutor (`0x16cb2xx`–`0x16cbaxx`); a atualização
  (`0x16cbd20` e o que ela chama: `0x16cd180` olhar, `0x16cc530` andar, `0x16cda90`) não lê o pulo. Ou seja, a
  câmera andando parece **não ter pulo** (a tecla existe nos controles, mas não é usada nela).
- Próximo passo: procurar quem mais lê esse controle (a cena da garagem); se ninguém, o pulo será nosso: um
  impulso vertical no corpo físico (`+0x638`, a cápsula), pelas funções virtuais da física do jogo que a câmera
  andando já usa (posição, velocidade), só com os pés no chão. Detalhes no plano (▶, etapa A, item 5).

### No ATS, o F8 não faz nada (resolvido: era o freio de mão)

- Teste do dono (2026-11-04, ATS 1.62.1.1, Volvo VNL 2025): o F8 não faz o personagem descer.
- O log do plugin mostra que o F8 chega e que o plugin recusa a descida: `character: not walking: the truck must be
  stopped, on its parking brake` a cada aperto. Ou seja: pra telemetria do ATS, o caminhão não estava parado
  (acima de 0,2 m/s) **ou** não estava com o freio de mão (`truck.brake.parking`). A mensagem não dizia qual.
- Feito: o log agora diz os dois valores (`character: the truck at X m/s, parking brake on/off`).
- Próximo passo: o dono testa de novo no ATS, parado e com o freio de mão puxado, e lê essa linha. Se o freio de
  mão vier "off" mesmo puxado (ou a velocidade não for 0 parado), ver o canal da telemetria no ATS 1.62 e aceitar
  o que ele manda (ex.: freio de mão pelo `truck.brake.parking` de outro jeito, ou velocidade um pouco maior que 0
  parado). Depois disso ainda faltam as peças do ATS (fábrica, início, mundo físico, núcleo: os formatos de
  código já conferidos no `amtrucks.exe`), que só se sabem no jogo.
- **Resolvido (2026-11-04)**: o dono confirmou que o freio de mão não estava puxado; com ele puxado, funciona no
  ATS. Fica o ponto de atenção: avisar na tela (plano, "Também pedido pelo dono", item 5).

### Encostar no caminhão a pé dá dano (a investigar, prioridade)

- Teste do dono (2026-11-04): correndo ou andando contra o caminhão, o jogo mostra "Danos no veículo" (ex.: 15% +4%).
- Causa provável: a cápsula da câmera andando (`walking_camera + 0x638`) está no mundo físico da estrada
  (`game_ctrl + 0x2da0`), junto com o caminhão; o contato dela conta como batida. Na garagem isso não acontece
  porque lá ela fica num mundo físico próprio da cena.
- O que o início (`0x16cc000`) faz com o corpo (a investigar): `0x1653ad0` cria o corpo no mundo; virtual `+0x1c0`
  com um `float` (massa?); `0x1653d40` cria a forma (cápsula, rotação 45°); `0x1657fd0(corpo, x)` com
  `x = 0` ou um global, conforme o tipo do mundo (`[sistema+0x40]`); `0x1658040(corpo, 0x12)` (camada/grupo de
  colisão?); virtuais `+0x200`, `+0x1b0` com 0; `0x1656ea0(corpo, mundo)` o põe no mundo.
- Detalhes do que tentar no plano (▶, "Também pedido pelo dono", item 6).
- **Causa achada e proposta**: ver §7.1 (etapa A).

## 7. Etapa A — investigação do personagem (2026-11-04, rodada automática, só leitura)

Feita sem o dono, lendo o `eurotrucks2.exe` (ETS2 1.61.1.9), o `amtrucks.exe` (ATS 1.62) e os pacotes `.scs`
dos dois jogos; com o ETS2 aberto, **só lendo** a memória dele (nada escrito, nada mudado no WestPlay). Scripts
(Python + capstone, `uv run`) na pasta de rascunho da sessão. Endereços relativos ao exe do ETS2 1.61, salvo
quando diz ATS.

### 7.1 Dano no caminhão ao encostar a pé (prioridade) — causa achada

**A física é PhysX** (classes `physx::Np*` no exe, textos `[physx] @%u: ...`), por baixo de uma camada do jogo
(`[phys] ...`). O que o início da câmera andando (`0x16cc000`) faz com a cápsula:

| Chamada | O que é |
|---|---|
| `0x1653ad0(sistema)` | cria um **corpo dinâmico** (virtual `+0x58` do sistema; o objeto PhysX fica em `corpo+0xf8`) |
| virtual `+0x1c0` com `0,007` | **massa** (guarda em `corpo+0xe0` e passa ao PhysX; `+0x1c8` refaz a inércia). A cápsula pesa quase nada |
| virtual `+0x1d0` | inércia (a partir da massa) |
| `0x1653d40` | cria a **forma** (cápsula 0,6 × 0,9, rotação 45°) |
| `0x1657fd0(corpo, 13)` | **grupo** (6 bits em `corpo+0x90`, bits 0–5; em cada forma, 5 bits em `forma+0xa4`). É 0 ou o global `0x304670c` (= 13), conforme `[sistema+0x40]` (= 1 na estrada) |
| `0x1658040(corpo, 0x12)` | **camada de colisão** = **18** (bits 6–11 de `corpo+0x90`; em cada forma, bits 5–9 de `forma+0xa4`). Vira o `word0` do filtro de simulação do PhysX e `1 << camada` no filtro de consultas |
| virtuais `+0x200`, `+0x1b0` com 0 | sono e modo cinemático (não mexem com batida) |
| `0x1656ea0(corpo, mundo)` | põe o corpo no mundo (lista em `mundo+0x10`/`+0x18`) |

**Quem bate em quem** (o filtro do PhysX, `0x1633830`, posto na cena em `0x163516e`): uma **tabela 32 × 32** de
camadas (`mundo+0x38`, 32 números de 32 bits; `mundo+0x40` = 32). Se a camada de uma forma não está na linha da
camada da outra, o par é ignorado (nem colide). Se colide, as "bandeiras" do par vêm do `word1` das duas formas (a
maioria em OU: **basta uma das formas pedir aviso de contato**). Lida na memória, na estrada:

| Camada | Colide com |
|---|---|
| 1 (mundo fixo: estrada, prédios — os corpos estáticos) | 1, 2, 5–15, 17, 18, 20, 23–31 |
| 4 (trânsito: carros da IA, ouvinte `0xa2a150`) | 4–8, 11–15, 18–31 |
| **6, 7** (**caminhão do jogador**, `game_physics_vehicle`) | 6: 1–5, 8, 11–20, 23–31 · 7: 1, 2, 4, 5, 8, 11–20, 23–31 |
| 8 (reboque, `game_trailer_actor`; ele tem formas na 10 também) | 1–8, 11–15, 17–20, 23–31 |
| 3 | só 6, 8, 20 |
| 9 | 1, 2, 9, 11–18, 23–31 (**não** 4, 6, 7, 8) |
| 10 | 1, 2, 11–15, 18, 23–31 (**não** 4, 6, 7, 8) |
| 16 | só 6, 7, 9 |
| **18 (a cápsula)** | 1, 2, **4–8**, 9–15, 22–31 |

**Como o contato vira dano**: depois de cada passo da física, o mundo percorre os contatos guardados
(`mundo+0x148`, registros de 0xa8 bytes, `0x1635516..`) e chama o **ouvinte de cada forma** (`forma+8`). As formas
do caminhão têm como ouvinte o próprio `game_physics_vehicle` (virtual `+0xb0` = **`0xa02dc0`**); as do reboque, o
`game_trailer_actor` (`+0xb0` = `0x638250`). Os dois chamam antes `0x863950` (comum aos veículos) e:

- `0xa02dc0` (caminhão): pega a camada das duas formas (`1 << ((forma+0xa4 >> 5) & 0x1f)`); **se uma delas for a
  camada 3 ou a 16 (`test esi, 0x10008`, em `0xa02e3e`), sai sem dano**. Senão, mede a **velocidade da batida** (a
  diferença das velocidades, na direção do contato) e **só conta dano acima de 1,38 m/s** (`0x251d850`; ~5 km/h:
  andar rápido ou correr passa disso). Daí em diante usa essa velocidade ao quadrado (com ajustes pra outras
  camadas: 2, 4/5/22).
- `0x638250` (reboque): **sai sem dano se uma das formas for a camada 3** (`(forma+0xa4 & 0x3e0) == 0x60`).
- **A massa não entra na conta** (por isso a cápsula de 7 g dá dano). Diminuir a massa **não resolve**.

Na garagem não dá dano porque lá o caminhão é só um modelo na camada 6 (`0x1165c60`), sem o ouvinte de veículo.

**O que propomos (sem implementar)**, do mais seguro pro menos:

1. **Trocar a camada da cápsula de 18 pra 9 (ou 10)** logo depois do início dela, com a mesma função do jogo
   (`0x1658040(corpo, 9)`, que já atualiza as formas e o PhysX). Com 9/10 ela **continua no chão e batendo no mundo
   fixo** (camada 1), mas **atravessa o caminhão, o reboque e os carros da IA** (6, 7, 8 e 4 fora da linha): **nunca
   dá dano** (nem nos carros da IA, que com a 18 também podiam levar). É uma chamada, sem mexer em código do jogo. O
   preço: passa por dentro do caminhão. Se incomodar, o plugin segura a posição fora da caixa do caminhão.
2. **Pra bater no caminhão sem dano**: além da forma na 9, uma **segunda forma na camada 3** (bate no 6 e no 8, que
   os dois ouvintes ignoram pra dano; não bate no 7). Exige criar mais uma forma (`0x1653d40`) e camada por forma
   (`corpo+0x90` com camada 32 = "cada forma a sua", como o reboque faz). Mais trabalho; só se o 1 não bastar.
3. Não recomendado: limitar a velocidade perto do caminhão abaixo de 1,38 m/s (frágil), ou tirar o par
   caminhão × cápsula pelo filtro do jogo (`0x164adc0`: lista de corpos a ignorar em `corpo+0x68`/`+0x70`; serve,
   mas mexe numa lista do corpo do caminhão).

**Teste necessário** (com o dono, single player, jogo salvo): pôr a camada 9 e conferir que (a) a cápsula não cai
pelo chão, (b) bate em parede/poste, (c) encostar e correr contra o caminhão não muda o dano (antes/depois), nos
dois jogos. No ATS 1.62 as mesmas peças estão lá (mesmo formato de código): o filtro (`0x16930e7`), o "pula as
camadas 3/16" (`0x974f4e`), e a câmera andando com `mov edx, 0x12` antes da camada.

### 7.2 Os personagens nos pacotes do jogo (`def.scs`, `base*.scs`)

Lidos com o leitor do WestPlay (`crates/mapdata`, exemplos `ls`/`cat`). Há **três gerações** de pessoas:

1. **Antiga** (`/model/mover/characters/…`; unidades `mover_model` em `/def/world/mover*.sii`, `pedestrian: true`
   nos que andam pelos `walker_item`): modelos `models/{generic,workers,special,enforcement}/*.pmd` (ex.:
   `workers/m_worker_caucasian_01.pmd`, `m_worker_afam_01.pmd`, `m_worker_old_01.pmd` — **os trabalhadores**;
   `generic/m_white_02.pmd`, `w_blond_03.pmd`…), com 4 LODs. Animações em `animations/man/…`:
   **`walk/walk_01.pma` (andar)**, **`run/run_01.pma` (correr: está no pacote, nenhuma definição usa)**,
   `idle/*.pma` (parado: `cros_hand_01`, `hand_back_01`, `leaning_01`, `idle_drink`, `idle_jog_stretch`…), e
   `woman/walk/walk_01.pma`. **É a única geração com correr.**
2. **"new_generation"** (`/model/mover/characters/new_generation/new/…`): `animations/universal/ngn_walk.pma`
   (andar), `crosshand_idle_anim`, `handback_idle_anim`, `leaning_idle_anim`, `sitting_*`, `wheel_change*`,
   `box_pick_anim`…; usadas pelo grupo `human` (tags `man`, `walk`, `idle`…).
3. **Nova** (`/asset/character/human/<tipo>/*_mov.pmd`; unidades `mover_desc` com `group_tags`, e `mover_anim` com
   o mesmo grupo + `stand`/`idle`/`walk`…): tipos `casual` (o maior), `police_officer`, `farmer`, `border_officer`,
   `business`, `beach`, `fireman`, `civil`, `worker` (frentistas: `gasstation_crew_boy_mov.pmd`) e
   `driver/man/driver.pmd` (o **motorista dos carros da IA**, com animações por carro). Animações em
   `/asset/animation/{man,woman}/`: **`meso_walking_01/02.pma`** (andar), **`meso_standing_idle_01..05.pma`**
   (parado), `meso_leaning_idle_*`, `meso_sitting_idle_*`, `*_mobile_*` (com celular), e versões `reduced_*` (mais
   leves), cada uma com 5 LODs (`_l1`…`_l5`). **Não há correr nem pular.**

ATS 1.62: a mesma estrutura (`asset/animation/man/meso_*`, `reduced_*`, `trafic_driving_ani.pma`; tipos `casual`,
`cowboy`, `driver`, `driver_bus`, `farmer`, `police_officer`…); **não tem** `model/mover/characters/animations/man/run`.

**O que serve pro personagem**: um homem da **nova geração** (corpo inteiro, LODs) com `meso_standing_idle_01`
parado e `meso_walking_01` andando — nos dois jogos. **Correr**: `meso_walking_01` mais rápido (se o mover aceitar
a velocidade da animação; `mover_action` tem `animation_params`), porque `run_01.pma` é do esqueleto antigo e só
existe no ETS2. Os trabalhadores de colete do ETS2 (`m_worker_*`) são da geração antiga (andar `walk_01`, correr
`run_01`): opção só pro ETS2. Pular e agachar: não há animação (o corpo fica com a de parado).

### 7.3 Como o jogo cria e mexe num mover (exe)

- **Criar**: `new(0x210)` + construtor **`0xa18f00`** (a instância do mover, 0x210 bytes), depois
  **`0xa19570(instância, dono, 0, 0, 0)`**: guarda o dono em `instância+0x108`, pede a ele o modelo (virtual `+8`
  "vale?", `+0x28` "qual modelo") e cria as instâncias do modelo (textos `[mover] Missing mover model definition.`,
  `[mover] Unable choose mover desc for given location.`, `[mover] Unable to create model instances.`). Quatro lugares
  do jogo fazem isso: o `mover_hookup` dos prefabs (`0x6f0ec0`, monta `mover.%s`), o `mover_item` do mapa
  (`0x7e1fe0`), `0xf7c600`, e — **o melhor exemplo pra nós** — o **passageiro de ônibus**
  (`player_bus_passenger_u`, **`0xb95230`**): um boneco criado **na hora**, perto do ônibus, a partir de um
  `mover_model` (o dono é o próprio passageiro: modelo em `+0xf8`).
- **Pôr no lugar**: **`0xa1c400(instância, &posição)`**, a posição no formato de sempre (32 bytes: x, y, z no setor
  de 512 m, setores, quatérnio; o passageiro passa `passageiro+0x90`). Chama `0x13ce580`/`0x13ce640` (move o nó da
  cena, em `instância+8`, e avisa a mudança). Só o passageiro a usa: é o jeito de pôr um boneco onde se quer.
- **Animação e movimento**: o mover tem **ações** (`/def/world/mover_action*.sii`; o tratador é **`0xa1e620`**,
  função 60 da vtable `0x22f4b88`): `show`, `move` (seguir um caminho), `transport`, `bind` (prender num osso:
  `rot_pos`, `bone_rot_pos`), tocar animação (`[mover] No animation found to play.`), cor do modelo. A
  `mover_action` tem `animation_params` (`looped`, `layer`, `blend_time`, `delay`), `procedural_animation_params`,
  `sound_params`, `timer_params`. Falta achar **a chamada que troca a animação** de uma instância já criada (provável:
  o "tocar animação" dentro de `0xa1e620`, e o que o passageiro usa pra andar até a porta — `0xb98980`/`0xb99850`
  chamam `0xa14cc0`/`0xa14ba0`, listas de ações de 0x80 bytes).
- **ATS 1.62**: as mesmas peças (passageiro `0x928e06`, criação `0x98bad2`, ações `0x990d99`; endereços das
  referências aos textos).

**O plano A parece possível**: o próprio jogo cria bonecos na hora (os passageiros) e tem uma função pra pô-los
em qualquer lugar (`0xa1c400`). Falta, com o jogo aberto: (1) achar um mover vivo e ler a instância (dono em
`+0x108`, nó em `+8`); (2) achar a troca de animação; (3) o **teste de escrita**: criar um mover nosso como o
passageiro (um dono nosso com as virtuais `+8`/`+0x28`, devolvendo um `mover_model` do jogo) e movê-lo com
`0xa1c400`. Se o dono "nosso" não der, o plano B (desenhar o `.pmd` nós mesmos) continua de reserva.

### 7.4 Sons de passos

- Os bancos FMOD estão em `sound/<grupo>/*.bank` dentro dos `.scs`, cada um com um `.bank.guids` em texto que
  lista os eventos. O de coisas do mundo é **`sound/world/mover.bank`**: trens, aviões, helicóptero, britadeira,
  cortador de grama, guindaste do porto, gato, serra… **nenhum passo**. Em todos os `.guids` do ETS2 e do ATS,
  procurando `step`, `foot`, `walk`, `jump`, `land`, `pedes`, `door`, `gravel`: só o cascalho dos **pneus**
  (`tires/gravel`).
- Conclusão: **o jogo não tem sons de passos**. Vale o que o plano já diz: sons CC0, anotados em
  `THIRD_PARTY_NOTICES.md`. O chão sob os pés: as formas da física têm "substâncias" (`collider_substances`), a ver.

### 7.5 O pulo

- O controle `camwalk_jump` (no jogo é um *token* de 64 bits, `0x58288f9202457173`, não um texto) só aparece em
  **dois lugares**: a lista de controles do jogo (`0x44050`) e o construtor da câmera andando (`0x16cb630`). Os
  outros (`camwalk_for`, `camwalk_run`, `camwalk_crou`) aparecem também nas opções (`0xa0b20`) e na cena da
  garagem (`0x1053c90`, `0x112e240`). **Ninguém lê o pulo: o jogo não tem pulo.** Vai ser nosso.
- Como fazer (proposta): a cápsula (`+0x638`) tem **`+0x138` = ler a velocidade** (vec3) e **`+0x240` = pôr a
  velocidade** (vec3); a própria atualização da câmera andando usa as duas a cada quadro e **mantém a velocidade
  vertical** (parada, põe `(0, vy, 0)`). Então o pulo é: depois da atualização dela, com os pés no chão, ler a
  velocidade e pôr `vy` ≈ 4 m/s; a gravidade traz de volta. (Há também `+0x250`, somar força no modo aceleração do
  PhysX.) Pés no chão: `vy` perto de 0 por alguns quadros (ou um raio pra baixo, a achar).

### 7.6 Aviso na tela

- O texto `@@notification_walking_camera@@` (`0x105e460`) **não é uma notificação de canto**: é o texto de um
  elemento da tela da garagem (põe o texto no elemento, `+0x100`, e chama a virtual `+0xf8`).
- O jogo tem textos próprios pro freio de mão: **`@@aca_parking_brake_on1@@` / `_on2@@`** (da autoescola,
  `0xa05130`), mostrados por **`0xa643f0`**, que escreve `"título|texto"` num painel do HUD da autoescola
  (`[[obj+0x50]+0x340]+0xf18`, e marca `+0xf38 = 1`). Pode servir se esse painel existir fora da autoescola (a ver).
- As notificações gerais do HUD (`notification_widget_hdl`, `@@notifications_keep_hidden@@`,
  `/material/ui/hud/alert_notification_generic.mat`) não foram achadas pelo texto no código (são ligadas por
  tabelas). Próximo passo: com o jogo aberto, achar o objeto `notification_widget_hdl` (pela classe) e quem o chama
  quando aparece um aviso (ex.: o autosave). Se não der, vale o (b) do plano: um aviso do WestPlay por cima.

### 7.7 Complementos (segunda rodada da etapa A, no mesmo dia, só leitura)

Outra rodada automática, feita em paralelo, chegou ao mesmo que §7.1–7.6; aqui só o que ela acrescenta:

- **Material físico** (bits 0–5 de `corpo+0x90`, "grupo" na §7.1): é um índice na **tabela de materiais**
  `exe+0x3674630` (32 ponteiros; nome em `+0x18`). Lida no jogo: 0 static, 1 road, 2 road_snow, 3 dirt, 4 snow,
  5 grass, 6 road_dirt, 7 invis, 8 ice, 9 metal, 10 rubber, 11 rumble_stripe, 12 plastic, 13 glass, 14 wood, 15 soft,
  16 road_smooth, 17 road_coarse, 18 gravel, 19 concrete. A cápsula na estrada fica com 13 (glass); a maioria dos
  corpos dinâmicos do jogo, 9 (metal, o global `exe+0x3046728`). **Isto responde o "chão sob os pés" da §7.4**: o
  material da forma em que a cápsula encosta diz asfalto/terra/grama/cascalho/concreto/metal/madeira → o som do passo.
- **Velocidades da câmera andando**: 4 m/s andando, 8 m/s correndo (`0x16cbf7d`/`0x16cbf87`, conforme o controle de
  correr `+0x430` estar em 2). Correndo (8 m/s) passa muito do limite de dano de 1,38 m/s.
- **Camada 10 em vez de 9**: as duas não batem em veículo; a 10 também não bate em si mesma nem na 9/16/17. Qualquer
  uma serve; testar a 9 primeiro (como proposto) e a 10 se a 9 der algo estranho.
- **Trabalhador de colete da nova geração** (corrige em parte a §7.2): o `mover_desc.n_worker` (ETS2) e
  `n_worker_1..7` / `nk_worker` (ATS) usam `/model/mover/characters/new_generation/new/models/man/worker_vest/
  ngn_worker_vest_lod0.pmd`, com **`ngn_walk.pma`** andando e `crosshand_idle_anim`/`handback_idle_anim` parado —
  **nos dois jogos**. É o "de colete" da foto do dono; candidato tão bom quanto o casual `m_casual_01a_mov.pmd` +
  `meso_walking_01`. Sem correr nem pular, como os outros.
- **ATS 1.62**: o construtor do mover é `amtrucks.exe+0x98b3c0` (mesmo formato `new(0x210)` → construtor →
  início); a camada 0x12 da câmera andando em `amtrucks.exe+0x16fd6ac`.
- **Notificação do canto do HUD (resolve em parte a §7.6)**: a função do jogo é **`exe+0x623da0`** (usa a lista
  `hud_notification_request_t`; 80 chamadas: `@@stop_to_weigh@@`, `@@trailer_decoupling@@`, `@@secret_found@@`,
  `@@level_up_msg@@`…). Chamada como `0x623da0(hud, &texto, r8, ícone, +7 argumentos na pilha)`, com
  `hud = [obj+0x30] + 0xd8` (exemplo completo em `exe+0x5d9524..0x5d9580`, o aviso de desengatar o reboque). O
  **ícone** é um índice na tabela (cópia viva em `exe+0x2d6a370`): 0 alerta, 1 aviso, 2 anúncio, 3 carta, 4 rádio,
  5 cidade descoberta, **6 freio de mão** (`/material/ui/hud/parking_brake.mat`), 7 combustível. Falta: de onde vem
  o `obj` (provável o núcleo do jogo) e o papel de cada argumento da pilha; conferir no ATS. Com isso dá pra fazer o
  aviso "Puxe o freio de mão para descer" **com a cara do jogo** e o ícone de freio de mão dele.

### 7.8 Como pôr uma pessoa do jogo onde quisermos (o teste de escrita)

O passageiro de ônibus mostra o caminho inteiro (ETS2 1.61; o mesmo formato no ATS 1.62, endereços entre
parênteses):

- **O dono** (`player_bus_passenger`, a parte "dono do mover"): 0x110 bytes, feito pelo construtor
  `0xb94fc0` (ATS `0x928a80`), vtable `exe+0x23226b0`. As perguntas que o mover faz ao dono são 11 virtuais
  curtas, todas lendo campos dele: `+0x8` vale (sempre sim), `+0x10` onde fica (copia o nó de `+0x90`, se
  `+0x88` ≠ 0), `+0x18` semente (`+0x8`), `+0x20` condições (`+0x10`), `+0x28` o modelo (`+0xf8`), `+0x50` o
  visual (`+0x100`); `+0x40`/`+0x48` devolvem 0. Com o construtor do jogo, um dono nosso responde igual ao de
  um passageiro.
- **O nó** em `dono+0x90` é um nó de cena do jogo: posição do mundo pronta em `+0`, posição local em `+0x20`
  (o formato de sempre: x, y, z no setor de 512 m, setores, quatérnio), pai em `+0x40`, filhos em
  `+0x48`/`+0x50`, "pronto" em `+0x58`. `0x13ce640` (ATS `0x14017d0`) **põe a posição** (num nó sem pai, a
  local) e avisa os filhos; `0x13ce580` troca o pai.
- **Criar**: `0xb95230(dono, modelo, &visual)` (ATS `0x928cf0`): só faz algo com `dono+0x88` ≠ 0; guarda o
  modelo e o visual, cria a instância do mover (`new(0x210)` → `0xa18f00` → `0xa19570`), guarda em `+0x108` e
  a **pendura no nó do dono** (`0xa1c400`: troca o pai mantendo a posição). Mover o nó do dono move a pessoa.
- **Soltar**: `0xb951c0(dono)` (ATS `0x928c80`): solta a instância e o modelo.
- **O modelo pelo nome**: `0x12a560(&texto)` (ATS `0x12d6f0`; recebe o endereço de um `char*`), o mesmo que os
  prefabs usam com `"mover.%s"`: `"mover.work_idle"` (trabalhador de colete parado, nos dois jogos) ou
  `"mover.n_ca_man_wlk"` (homem comum andando, `ngn_walk`).

**Versão de teste** (`crates/plugin/src/character/person.rs`): a pé, **F9** cria um dono pelo construtor do
jogo, põe o nó 2,5 m à frente da vista e 1,7 m abaixo dos olhos, virado pra você, e chama a criação do
passageiro com `mover.work_idle` (se não houver, `mover.n_ca_man_wlk`); a cada quadro o nó acompanha você
(`0x13ce640`). F9 de novo, ou subir no caminhão, solta a pessoa (a soltura do passageiro e o destrutor do dono,
sem liberar a memória, que é do plugin). Tudo achado pelo formato e conferido; nada achado, o F9 não faz nada.
A testar: se a pessoa aparece, se a animação anda sozinha (o código do passageiro não chama a atualização da
instância: deve ser o sistema de movers do jogo), a altura dos pés e pra que lado ela olha.

### 7.9 Aviso no canto da tela: como chamar (2026-11-05, só leitura do exe, ETS2 1.61 e ATS 1.62)

Completa a §7.6/§7.7. Lidas as 80 chamadas da função de aviso no ETS2 (83 no ATS) e a própria função; todas
montam os argumentos do mesmo jeito.

- **A função**: ETS2 `0x623da0`, ATS `0x622870`, **o começo igual byte a byte nos dois** (72 bytes:
  `44 89 4C 24 20 4C 89 44 24 18 48 89 54 24 10 55 57 41 56 48 83 EC 60 80 79 38 00 4D 8B F0 4C 8B C2 49 63 E9 48
  8B F9 0F 84 ?? ?? ?? ?? 80 79 3A 00 75 0A 80 79 3B 00 74 04 32 C0 EB 02 B0 01 84 C0 0F 85`): dá pra achar pelo
  formato, como o resto do plugin.
- **O objeto (1º argumento, `rcx`)**: `[[core] + 0x31b0] + 0x30`, mais `0xd8` — `core` é o mesmo objeto central
  que o plugin já acha (`CORE_SITE`: ETS2 `exe+0x36af7c8`, ATS `exe+0x3719ca8`). 18 das chamadas fazem exatamente
  `mov rax, [core]; mov rax, [rax+31b0h]; mov rcx, [rax+30h]; add rcx, 0d8h`, nos dois jogos. Nele: `+0x38` ligado
  (se 0, a função não faz nada), `+0x3a`/`+0x3b` escondido, `+0x18`/`+0x20` a lista de avisos (0x70 bytes cada),
  `+0x40` o próximo número, `+8` o widget do HUD.
- **Os argumentos** (como `@@stop_vehicle_short@@`, "Pare o veículo!", o aviso curto de "pare pra dormir"):
  1. `rcx` = o objeto acima;
  2. `rdx` = **endereço de um `const char*`** com o texto (UTF-8). A função **copia** o texto (`0xf2f60`, string
     a partir de `char*`): o nosso pode ser temporário. Pode ser uma chave do jogo `@@…@@` (o jogo traduz) — a
     confirmar no jogo se um texto sem `@@` aparece como está (o esperado);
  3. `r8` = endereço de um `const char*` (vai pra `+0x48` do aviso); o jogo sempre passa o global
     `exe+0x2733198`, que aponta pra `""`. Passar um `""` nosso;
  4. `r9` = o **ícone**: índice na tabela (ETS2 `exe+0x2d6a370`, ATS `exe+0x2dd1ad0`, **igual nos dois**): 0
     alerta, 1 aviso, 2 anúncio, 3 carta, 4 rádio, 5 cidade, **6 freio de mão** (`parking_brake.mat`), 7
     combustível, 8/9 passageiros; **11 = material próprio**, dado no 5º;
  5. pilha `+0x20` = o material do ícone 11 (senão o mesmo `""` do 3º);
  6. (2026-11-05, corrigido: o aviso do WestPlay não sumia) o **2º deles (`+0x30`) = 1 deixa o aviso na tela até o
   jogo tirar** (`0x624200`: com 1, sem tempo; com outro, o tempo de sempre do jogo). "Pare o veículo!" usa 1 porque o
   jogo tira quando o caminhão para. O WestPlay passa **0**.
   6. `+0x28`, `+0x30`, `+0x38` = três números que ordenam a lista (prioridade): "Pare o veículo!" usa
     **2, 1, 2**; o achado de segredo, 3, 0, 2; o "proibido caminhão", 1, 0, 0;
  7. `+0x40` = um número de 16 bits (vai pra `+0x58`): "Pare o veículo!" usa **7**, a maioria −1;
  8. `+0x48` = um objeto com vtable (chamado quando o aviso sai) ou **0**;
  9. `+0x50` = endereço de `{u8 tem_número; u32 número}`: com `tem_número` = 1, um aviso com o mesmo número é
     **trocado** em vez de somado (o "Pare o veículo!" usa o número 4). Pra nós: um número nosso (ex.:
     `0x5750_0001`), assim apertar F8 várias vezes não empilha avisos.
- **Chamar**: no fio principal do jogo, no fim do quadro (onde o plugin já chama as funções do jogo), com o
  jogo fora do menu. Com `+0x38` = 0 (avisos desligados), não chamar.

**Textos**: o jogo tem `@@stop_vehicle_short@@` ("Pare o veículo!", nos dois jogos e em todos os idiomas) pro
caminhão andando. Não tem "puxe o freio de mão para descer" (só os da autoescola, longos: "Agora, acione o
freio de estacionamento para que o caminhão não se mova acidentalmente…"). Proposta: os textos do WestPlay no
idioma do jogo (o app já sabe o idioma: a página Personagem usa o mesmo), passados ao plugin pela ponte; e a chave
do jogo onde existir.

**Avisos propostos** (PLANO "Também pedido pelo dono" 5): sem freio de mão → ícone 6, "Puxe o freio de mão para
descer"; caminhão andando → ícone 1, `@@stop_vehicle_short@@`; longe da porta ao voltar → ícone 2, "Chegue perto da
porta do motorista para entrar"; app fechado / modo desligado / TruckersMP → ícone 1, cada um com o seu texto.

**Risco**: argumento errado aqui pode fechar o jogo. Por isso: a função achada pelo formato acima (nada achado →
só o log, como hoje), o objeto conferido (`+0x38` lido e a lista `+0x18`/`+0x20` coerente: contagem pequena,
ponteiro válido) antes de chamar, e o primeiro teste no jogo salvo.

### 7.10 O raio da física (câmera fora das paredes) (2026-11-05, só leitura do exe, ETS2 1.61 e ATS 1.62)

- **PhysX 3.4** (caminhos `PhysX_3.4\Source\...` no exe). `PxScene::raycast` = **ETS2 `0x1b06a10`**, ATS
  `0x1b39bb0` (começo igual; um 2º com o mesmo começo, ETS2 `0x1b0e6e0`, é de outra classe). Argumentos (conferidos
  na função): `(cena, &origem, &direção, distância (xmm3), &buffer, &bandeiras (u16), &filtro, callback, cache)`.
- Na tabela virtual da cena (`NpScene`, ETS2 `0x2502350`, posta pelo construtor em `0x1adfef3`) fica na posição
  **`0x2f0`**; igual no ATS (`0x2557618` − `0x2f0`). Achar a cena: o objeto cuja tabela tem o raio nessa posição,
  procurado entre os ponteiros do mundo da física e do sistema.
- **Filtro de consulta das formas do jogo**: `word0 = 1 << camada` (o "pôr camada" da forma, ETS2 `0x1641980`,
  chama `setQueryFilterData` (`+0xa8` da forma PhysX) com isso). O plugin pede as camadas 1, 2, 4, 6, 7, 8 (mundo,
  trânsito, caminhão, reboque) e não a 9 (a cápsula) nem a 3 (a 2ª forma).
- **Coordenadas**: as do PhysX são as do jogo menos uma **origem** (o jogo soma `[obj+0x58]+0x110` ao ler a
  posição de uma forma, ETS2 `0x163b510`). O plugin tenta essas origens e confere: **um raio pra baixo, dos olhos,
  tem que bater no chão a ~1,8 m** (onde estão os pés); se não bater, sem raio (a câmera como antes).
- Uso: do ponto 1,25 m acima dos pés até a câmera; batendo, a câmera fica 0,25 m antes (no mínimo 0,4 m do
  ponto), e volta a 3 m/s. `character/ray.rs`.

### 7.11 Lanterna: uma luz do jogo num mover nosso (2026-11-05, só leitura, ETS2 1.61 e ATS 1.62)

- As luzes de verdade do jogo (as que iluminam, não só o brilho) são unidades **`light_source`** em
  `/unit/hookup/lights/` (353 no ETS2; a pasta `spotlight/` tem holofotes): `setup: lumen_hue_saturation`,
  `diffuse_color`/`specular_color` (lúmen, matiz, saturação), `range`, `inner_angle`/`outer_angle` (cone). Ex.:
  `light.spot.y.fadelong.150m.deg50`: 8000 lúmen, 150 m, 1° e 50°.
- Uma luz entra num modelo por um **locator** com `hookup` (o nome da unidade), na `.pmg` (registro de 44 bytes: nome,
  posição, escala, rotação, deslocamento do nome do hookup na tabela de textos).
- **O jogo já tem um mover com ganchos**: `mover.light_flare` (`/asset/animation/nescript/light_flare.pmd`, os dois
  jogos): 2 pedacinhos de 1 mm, cada um com um locator ligado a um `flare` (brilho). Ou seja, um `mover_model` cria os
  hookups do seu modelo.
- Feito (`crates/setup/src/flashlight.rs`): no `westplay.scs`, `/model/westplay/flashlight.pmd/.pmg` escritos por nós
  (um pedacinho de 1 mm, transparente, e um locator `light` com hookup `wp.flashlight`), a luz
  `/unit/hookup/westplay_flashlight.sii` (2500 lúmen, branca quente, 30 m, 12° e 40°) e
  `/def/world/mover.westplay.sii` (`mover.wp_torch`). O app refaz o mod quando ela falta
  (`game_mod::layouts_current`).
- No plugin: **L** a pé cria o mover (como a pessoa: `person::show_thing`), põe na mão direita (0,22 m à direita,
  0,35 abaixo e 0,15 à frente dos olhos) virado pra vista (rumo e inclinação), a cada quadro; desenhado junto com a
  pessoa no gancho de desenho (`DRAWN_TOO`). O L não vai pro jogo a pé (os faróis).
- **A confirmar no jogo**: (1) se a luz de um mover acende (o brilho do `light_flare` sim; a luz, esperado); (2) se a
  luz aponta pro −z do locator (como a câmera) ou pra outro eixo; (3) se precisa ser noite (as luzes do mapa acendem
  por hora; esta não tem `lamps_on_hour`).
- **Resultado (2026-11-05, ETS2 1.61, de noite)**: até as 20:19 o jogo recusava `mover.westplay.sii` (o nome
  `mover.wp_flashlight` tem 13 letras numa parte; o máximo é 12: agora `mover.wp_torch`). Depois disso o mover
  aparece (log), mas **uma `light_source` nele não ilumina**, nem o cone nem uma luz forte em volta (`type: point`,
  8000 lúmen, 12 m).
- **Luzes que andam, lidas no jogo**: (a) `flare_vehicle` (`/unit/hookup/vehicle/flare/`), as luzes dos veículos:
  levam luz de verdade (`vehicle_int_light.sii`: `type: spot`, lúmen, `range`), mas acendem pelo estado das luzes do
  veículo (`light_type`: `low_beam`, `electrics`…), ou seja, presas a um veículo; (b) `flare` e `flare_blink`
  (`/unit/hookup/light_blinking.sii`), os brilhos dos modelos, que o mover do jogo usa (`flare.model.red.blink0`,
  `flare.model.plane.blink`); um `flare_blink` **também pode levar luz** (`flare.model.red.arrow2.blink0`:
  `setup`, `type: spot`, `range` 3, ângulos, `model_light_source`).
- **2º teste montado** (21:20): a lanterna como `flare_blink` sempre aceso (`blink_pattern: "X"`), brilho
  `/model/flare/lamp_flare_small.pmd` e a luz em volta. Se o brilho aparecer e a luz não, o próximo caminho é (a): um
  acessório do caminhão não segue o jogador, então seria preciso achar no código como o jogo cria uma luz de veículo.
- **Resultado do 2º (21:36)**: nada, nem o brilho. Comparados: o nosso `flashlight.pmg` e o `light_flare.pmg` do
  jogo têm o mesmo cabeçalho e o mesmo registro de locator (posição 0, escala 1, sem giro, o hookup no 1º texto), e
  `mover.light_flare` é definido como o nosso (só `mover_name` e `model_desc`, em `mover.base_share.sii`).
  Hipótese: o caminho dos passageiros (`person::show_thing`: dono, `spawn`) cria o modelo, mas não os seus ganchos.
  **3º teste (21:42)**: o plugin cria o próprio `mover.light_flare` do jogo na mão; se os brilhos vermelhos dele
  também não aparecem, a hipótese está confirmada.

### Registro (etapa A)

| Data | O que | Resultado |
|---|---|---|
| 2026-11-05 | lanterna e barra de teclas | **Lanterna** (§7.11): uma `light_source` nossa num mover nosso, L a pé; instalada (o app refez o mod). **Barra de teclas** (`hints.rs`): desenhada pelo plugin (GDI → textura, como o celular: `handset::Quad`), na cabine o celular e a tela sempre e descer parado com freio de mão; a pé entrar perto da porta e a lanterna; página Personagem: ligar/desligar e tecla de esconder (ponte v30). Falta o teste do dono (`TESTES.md` 10–11) |
| 2026-11-05 | avisos na tela, câmera fora das paredes, subir em 1ª pessoa (sem o jogo aberto) | **Avisos** (`character/notice.rs`, §7.9): função achada pelo começo (67 bytes, um só nos dois jogos), objeto `[[core]+0x31b0]+0x30+0xd8` conferido (12 chamadas do jogo o pegam assim, nos dois); ligado (`+0x38`) e a lista coerente antes de chamar; número próprio `0x57500001` (troca em vez de empilhar); textos em 11 idiomas pelo `g_lang` do `config.cfg` do jogo (senão inglês); caminhão andando = `@@stop_vehicle_short@@`. Avisos: sem freio de mão (ícone 6), andando, longe da porta (ícone 2), app fechado, multiplayer; modo desligado: nada. **Câmera**: §7.10. **Subir**: a tecla na porta passa pra 1ª pessoa e a câmera vai da posição atual à porta, sobe o degrau, passa a porta e senta (≈ 2,5 s, `character/climb.rs`). Instalados; falta o teste do dono (`TESTES.md` 7–9) |
| 2026-11-05 | aviso na tela: como chamar | função `0x623da0` (ATS `0x622870`, começo igual), objeto `[[core]+0x31b0]+0x30+0xd8`, argumentos e ícones (iguais nos dois jogos; 6 = freio de mão), texto por `const char*` copiado; §7.9. Falta implementar e testar |
| 2026-11-05 | pulo, tecla de descer, escolher o personagem (sem o jogo aberto) | **Pulo** (§7.5): Espaço, a cada aperto, com a cápsula parada na vertical (|vy| < 0,15 m/s) por 3 quadros: `vy = 3,0 m/s` pela virtual `+0x240` (uns 45 cm); sem passos no ar; ao cair (2 quadros parada), um som de passo; log "jump". **Tecla**: `character.key` nas configurações (código virtual, F8 = 0x77 por padrão), na ponte `HostBlock::character_key` (ponte **v29**); o plugin usa no lugar do F8 (descer, entrar, segurar 2 s). **Personagem**: `CHARACTER_PEOPLE` (shared) = os 32 `mover_model` andando com `walk_01.pma`, iguais no ETS2 1.61 e no ATS 1.62 (`m_walk_01..08`, `w_walk_01..24`, lidos de `/def/world/mover*.sii`); `character.person` (lugar na lista, padrão 5 = `m_walk_06`) na ponte `character_person` (lugar + 1). Pose de parado do mesmo modelo só onde o jogo tem (`person.rs` `STANDING`: `m_walk_06` → `m_idle01`/`idle_event01`; só ETS2: `m_walk_01` → `idl_m_b_h_bc`, `m_walk_05` → `idl_m_w_h_cr`, `w_walk_17` → `idl_w_bln_1`, `w_walk_08` → `idl_w_br_1`); sem ela, parado, a de andar fica com a animação parada (atualizada com 0 s). Escolhido ausente no jogo → o padrão. Página Personagem: botão de tecla (recusa as de andar e a da tela) e a lista "Quem anda" (textos nos 41 idiomas). Testado no navegador (preview) e nos testes; app + plugin instalados; falta o teste do dono: `TESTES.md` |
| 2026-11-05 | passo gravado (no lugar do sintetizado) | Dono: o som feito pelo plugin "parece chutar, não pisar"; ele deu uma gravação de passos no asfalto (`passos_asfalto_sem_ruido.wav`, 67 s, 24 kHz, estéreo, já sem ruído, um passo a cada ~0,6 s). Recortados 12 passos regulares (intervalo 0,55–0,7 s antes e depois, pico médio), cada um 0,32 s a partir de 20 ms antes do calcanhar, em mono, entrada de 15 ms e saída de 80 ms suaves, mesma altura (RMS dos primeiros 100 ms), pico ≈ 0,32: `crates/plugin/assets/steps_asphalt.raw` (16 bits, 24 kHz, 184 KB, embutido na DLL). Tocados pelo mesmo fluxo que mistura (`steps.rs`), sorteados sem repetir o anterior. **Antes de publicar**: confirmar com o dono de onde vem a gravação e a licença (anotar em `THIRD_PARTY_NOTICES.md`). Instalada; antiga em `plugins/westplay.dll.passo-sintetico` |
| 2026-11-05 | polir: som do passo, pés no chão, corrida | **Som**: o tom grave (seno de 70–100 Hz) saiu — era ele o "chutar lata"; agora só ruído com forma de sapato no chão (baque abaixo de ~200 Hz + toque abafado de ~250 Hz a ~2 kHz, calcanhar e ponta, começo e fim suaves), 8 variações sorteadas; tocado por um fluxo de som próprio do plugin (`waveOut`, 3 buffers de 20 ms, aberto ao andar e fechado após 3 s quieto) que **mistura** os passos (nenhum corta o outro, e o Windows não abre/fecha o som a cada passo: o estalo do `PlaySound`). Amostra: `../passos-teste.wav` (teste `write_a_walk`). **Pés**: lido no código (ETS2 `0x16cc000`/`0x16cce70`, ATS `0x16fd4f0`/`0x16fe3e0`): olhos = meio da cápsula + 0,75 + `+0x660` − agachar (`+0x664`, 0..1); a cápsula tem 0,6 e 0,9 (forma `+0x18`/`+0x1c`) do meio até a base → pés 2,25 m abaixo dos olhos (somando os dois tamanhos inteiros). **Teste do dono (ATS): afundado até os joelhos**: o 0,9 é o comprimento da parte reta da cápsula (metade: 0,45), então **pés 1,8 m abaixo dos olhos** (0,75 + 0,6 + 0,45); corrigido e reinstalado. O plugin lê esses valores a cada quadro (`feet_under`; agachado, os pés ficam no chão) e o log diz a altura. A câmera de 3ª pessoa passou a girar em volta de um ponto 1,25 m acima dos pés (o mesmo enquadramento de antes). **Correr**: 2,6 m/s (era 3,6) com a animação de andar 1,6× mais rápida (os pés acompanham o chão), passo a cada 0,75 m. Instalada (ETS2 e ATS; antiga em `plugins/westplay.dll.terceira-7`); falta o teste do dono |
| 2026-11-05 | andar com animação e passos | o mesmo homem `m_vanilla_ice_01` tem `mover_model` parado (`m_idle01` ATS / `idle_event01` ETS2, `cros_hand_01`) e **andando** (`mover.m_walk_06`, `walk_01.pma`, nos dois jogos). Feito: dois bonecos (parado e andando), os dois postos e atualizados a cada quadro, desenhado o que combina (andando acima de 0,3 m/s). Velocidade a pé limitada a 1,6 m/s (3,6 com Shift) pela velocidade da cápsula (virtuais `+0x138` ler, `+0x240` pôr; conferidas no update da câmera andando nos dois jogos), pra os pés não escorregarem (a câmera andando é 4/8 m/s). Passos: som feito pelo plugin (baque grave + arrasto de ruído filtrado, calcanhar e ponta, 4 variações, 22 kHz, `PlaySound` da memória), um a cada 0,75 m (1,1 m correndo). Sem som de terceiros |
| 2026-11-05 | 4º teste (ETS2): girar a câmera "tremia" | causa: o plugin punha a câmera 0 atrás do boneco no fim do quadro, e o jogo a girava pelo mouse no quadro seguinte, antes de desenhar: um quadro de atraso. Achado (lido no jogo): a câmera 0 guarda o rumo em `core_camera+0x10` e a inclinação em `+0x14` (radianos 0..2π; olhando pra baixo, 2π − 0,34) e refaz o quatérnio deles a cada quadro. Feito: a pé, o mouse é do plugin (`input` dá o movimento ao `character`, zero ao jogo); a cada quadro o plugin soma o mouse no rumo e na inclinação (0,004 rad por contagem a 100%, × sensibilidade; inclinação de −1,2 a +0,35 rad), escreve os dois em `+0x10`/`+0x14`, o quatérnio e a posição juntos. Câmera na altura da cabeça, como as referências do dono: 3,8 m atrás, 0,15 m acima dos olhos, começando 10° pra baixo |
| 2026-11-05 | 3º teste (ETS2): **o boneco aparece** | o homem `m_vanilla_ice_01` de braços cruzados nos pés do jogador, de costas pra câmera (o modelo olha pra −z, como a câmera), a animação tocando. Pedido do dono: a câmera girar 360° sem virar o boneco, e mais longe. Feito: o boneco vira só pelo andar (para onde o jogador se moveu no quadro, acima de 0,3 m/s, até 10 rad/s, pelo caminho mais curto); câmera 4,5 m atrás e 0,6 m acima dos olhos |
| 2026-11-05 | 2º teste (ATS), modelo fixo | o modelo é um `mover_model` (`m_idle01`), a instância nasce e fica pendurada no dono, mas **nada aparece** e o nó da pessoa nunca é recalculado ("em dia" 0): **ninguém atualiza nem desenha** a instância. Achado: um mover não se cuida sozinho; quem o tem chama a cada quadro **`0xa1b200(instância, segundos)`** (ATS `0x98d5e0`; o laço dos passageiros em `0x5ae470` usa o tempo do quadro em µs × 1e-6), que liga o "mostrado" (`+0x158` bit 8, em `0xa1b4c3`) quando o modelo fica pronto e toca a animação, e no desenho **`0xa1c210(instância, contexto)`** (ATS `0x98e5f0`; só desenha com `+0x158 & 0xc == 8`), chamada pelos passageiros dentro de **`0x88a040`** (ATS `0x879190`), a virtual que junta as coisas do mundo do jogador pra desenhar (contexto em `rdx`; passadas em `[contexto+0x18] & 0xf`). Feito: o plugin chama o "atualizar" a cada quadro e um gancho (MinHook) em `0x88a040` desenha a pessoa depois do jogo, só no mesmo fio do quadro do plugin (senão, avisa no log e não desenha) |
| 2026-11-05 | 1º teste da 3ª pessoa (ATS) | a câmera atrás funciona; a pessoa **não aparece**: no lugar, o "?" verde do jogo (objeto sem modelo, visto com `g_developer`) **na porta do motorista**, onde ela foi criada, e não nos pés (não acompanha o nó do dono). Log sem erro (`shown (mover.work_idle)`). Feito: modelo fixo `mover_model` (`m_idle01` no ATS / `idle_event01` no ETS2: `m_vanilla_ice_01` de braços cruzados) antes dos grupos, e no log o tipo do modelo e, a cada 2 s, o nó do dono, o nó da pessoa (`instância+8`), se está em dia e se está pendurado no dono |
| 2026-11-05 | terceira pessoa (no lugar do F9) | a pessoa do jogo nos pés do jogador (`on_feet`: 1,7 m abaixo dos olhos, virada pro rumo da vista) e a câmera 0 atrás (`behind`: 3 m atrás pela direção da vista, 0,4 m acima); V troca 1ª/3ª. Instalada; falta o teste do dono |
| 2026-11-05 | pôr uma pessoa do jogo (teste de escrita) | dono do passageiro de ônibus (`0xb94fc0`), criar `0xb95230`, mover o nó `0x13ce640`, soltar `0xb951c0`, modelo por nome `0x12a560`; mesmos formatos no ATS. Versão de teste com F9 instalada; falta o teste do dono |
| 2026-11-04 | versão de teste: cápsula na camada 9; F9 a pé põe/tira uma 2ª forma igual na camada 3 (bate no caminhão sem dano). Peças achadas pelo formato no início da câmera andando, no `set_layer` e no destrutor (mundo `+0x678`, sistema `+0xd0`, corpo `+0x638`, forma `+0x640`, campo `+0x90`, virtual da forma `+0xd8`, criar forma `0x1653d40`, soltar forma: virtual `+0xa8` do sistema); conferidas no ETS2 1.61 e no ATS 1.62 | instalada; falta o teste do dono |
| 2026-11-04 | teste do dono (ETS2): camada 9 + forma na camada 3 | **sem dano**, bate no caminhão ("ficou excelente"); agora as duas vêm com o F8 (descer) e a 2ª forma sai com o F8 (subir); o F9 saiu |
| 2026-11-04 | 2ª rodada: materiais físicos, notificação do HUD, colete nova geração | materiais `exe+0x3674630` (chão pros passos); notificação `exe+0x623da0` (ícone 6 = freio de mão); `ngn_worker_vest` + `ngn_walk` nos dois jogos (§7.7) |
| 2026-11-04 | dano no caminhão: física, camadas, filtro, ouvinte do caminhão | causa achada: a cápsula (camada 18) bate no caminhão (6, 7) e no reboque (8); dano acima de 1,38 m/s; camadas 3/16 isentas; massa não conta. Proposta: camada 9 |
| 2026-11-04 | personagens nos pacotes (ETS2, ATS) | 3 gerações; nova: `meso_walking_01`/`meso_standing_idle_*`, sem correr; antiga (só ETS2): `walk_01`, `run_01` |
| 2026-11-04 | criar/mexer num mover (exe) | criar: `0xa18f00` + `0xa19570` (como o passageiro de ônibus, `0xb95230`); pôr no lugar: `0xa1c400`; ações: `0xa1e620`. Falta: trocar a animação |
| 2026-11-04 | sons de passos (FMOD) | não há no jogo |
| 2026-11-04 | pulo | o jogo não lê `camwalk_jump`; proposta: velocidade da cápsula (`+0x138`/`+0x240`) |
| 2026-11-04 | aviso na tela | `@@notification_walking_camera@@` é texto da garagem; textos de freio de mão da autoescola (`0xa643f0`); a notificação geral: a achar no jogo |
