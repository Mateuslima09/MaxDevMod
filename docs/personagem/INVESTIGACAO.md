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
