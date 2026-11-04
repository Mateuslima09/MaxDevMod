# Modo Personagem — planejamento

Planejamento para colocar no WestPlay um **personagem 3D que desce do caminhão, anda a pé e volta pra cabine**,
no ETS2 e no ATS. Escrito em 2026-11-04, antes de começar. A ideia é seguir fase por fase, testando cada uma no
jogo antes de passar pra próxima.

---

## ▶ Próxima tarefa (pedida em 2026-11-04, pra começar "quando eu chegar em casa")

**Andar a pé no ETS2 e no ATS, só a câmera, ainda sem boneco.** Junta a fase 0 (só o que for preciso), a 1 e a
2, e um pedaço da 4 (o chão). O que o dono pediu:

1. **Área "Personagem" no app do PC** (página nova no companion), com **Ligar/Desligar**. Começa só com isso;
   novas configurações entram nela conforme o modo cresce (§7).
2. **Uma câmera que se comporta como uma pessoa andando:**
   - **altura fixa em relação ao terreno** (olhos de uma pessoa, ~1,70 m acima do chão onde está, subindo e
     descendo com o terreno — não uma altura fixa no mundo);
   - **olhar em 360°** com o mouse, como na vida real (girar pros lados à vontade; pra cima e pra baixo com
     limite, sem dar cambalhota);
   - **WASD** pra andar (frente, trás, lados), na direção pra onde se olha;
   - **velocidade de caminhada que fique natural** (começar em ~1,4 m/s, como uma pessoa andando; ajustar no
     teste até "parecer certo"; correr fica pra depois).
3. **Sair do lado da porta do motorista:** ao descer, a câmera aparece **do lado de fora da porta do motorista**.
   O lado vem de onde está a cabeça do motorista na cabine (telemetria: à esquerda na maioria; à direita nos
   caminhões com volante à direita, como os do Reino Unido), não de um lado fixo.
4. **Depois, se tudo der certo nos testes:** **som de passos** (fica anotado, não entra agora).

Cuidados desta tarefa (do §5 e §9):
- Só deixa descer com o caminhão **parado**; enquanto a pé, o caminhão **não recebe** WASD nem o mouse.
- **Desligado por padrão**; se algo da câmera não for encontrado no jogo, o modo não liga e o log diz por quê.
- O chão: começar pelo **nível do chão onde o caminhão está**, e logo em seguida acompanhar o terreno de verdade
  (profundidade da imagem, fase 4, ou o que a investigação achar melhor). Se o terreno ficar pra uma segunda
  rodada, deixar isso claro no registro.
- Testar no **ETS2 e no ATS**, num caminhão com volante à esquerda e, se der, num à direita.

**Deu certo se:** liga no app → com o caminhão parado, aperta a tecla → a vista aparece do lado da porta do
motorista na altura de uma pessoa → anda com WASD em velocidade de caminhada, olhando em volta com o mouse,
subindo e descendo calçada/rampa sem flutuar nem afundar → aperta a tecla de novo e volta pra cabine. Sem o
caminhão se mexer e com o FPS igual ao normal.

Prompt pra começar:

> Leia `docs/personagem/PLANO.md` e faça a **Próxima tarefa** (o bloco ▶ no topo). Siga a regra de originalidade
> (§2) e "desligado por padrão / não faz nada se não achar" (§5). Comece pela investigação que for necessária
> pra controlar a câmera 0 e me conte o que achou antes de seguir. Instale no meu PC pra eu testar no jogo, mas
> **não envie pro GitHub** até eu pedir. No fim, atualize o "Registro do andamento".

---

## 1. Objetivo

Com o **Modo Personagem** ligado no WestPlay:

- Com o caminhão parado, uma tecla faz o motorista **descer** (animação), e a câmera passa a seguir ele a pé.
- Ele **anda, corre, agacha e senta** em **terceira pessoa** (câmera atrás) ou **primeira pessoa** (câmera nos olhos).
- Perto da porta, a mesma tecla faz ele **subir** (animação) e o jogo volta pra **câmera interna**.
- Se der: **colocar o cinto** antes de andar, interagir com coisas.

Desligado, o WestPlay continua exatamente como é hoje.

## 2. Regra de originalidade

A inspiração é o mod do **ggfxmods** (stories de 2026-11-03). **Não copiamos nada dele**: nem código, nem modelos,
nem animações, nem ícones, nem a interface. Olhamos só **o que ele faz visto de fora** (comportamento), e
escrevemos tudo do zero, do nosso jeito. Modelos e animações vêm de fontes com licença que permita usar
(ver §8).

## 3. O que se observa no mod de referência (só comportamento)

- Personagem **3D com animação de esqueleto**: anda, vira, agacha, senta num banquinho, braço 3D em primeira pessoa.
- **A câmera segue o personagem** (ele fica no centro, embaixo; o mundo passa em volta): é a **câmera 0** (livre)
  presa a ele, como o dono descreveu.
- Ele **pisa no chão certo** (calçada, asfalto, meio-fio): o mod sabe a altura do chão.
- Interface por cima: barra de itens, fome/sede, dicas de tecla (K entrar no caminhão, V câmera, C agachar).
- Funciona no **TruckersMP** (tags de jogadores visíveis).

Palpite de como funciona (não confirmado): controla a câmera 0 pela memória do jogo; desenha o próprio modelo no
DirectX com a mesma perspectiva da câmera do jogo; usa o **mapa de profundidade** da imagem pra achar o chão e
esconder o personagem atrás das coisas.

## 4. O que o WestPlay já tem e vamos reaproveitar

| Precisa | Já existe | Onde |
|---|---|---|
| Entrar no DirectX do jogo | Ganchos de D3D11/DXGI (Present, render targets, texturas) | `crates/plugin/src/d3d.rs`, `hooks.rs` |
| Desenhar por cima da imagem sem estragar o estado do jogo | O celular do jogo é desenhado assim (estado próprio, trocado e devolvido) | `crates/plugin/src/handset.rs` |
| Saber onde está a câmera | Posição da câmera lida da memória, achada pelo código do jogo (resiste a updates) | `crates/plugin/src/camera.rs` |
| Tirar o mouse/teclado do jogo enquanto usamos | Ganchos de DirectInput e do teclado | `crates/plugin/src/input.rs`, `keyboard.rs` |
| Teclas próprias | Atalhos do WestPlay (celular, volume) | `crates/plugin/src/keybinds.rs` |
| Posição/rotação do caminhão e da cabine | Telemetria do SDK | `crates/plugin/src/telemetry.rs` |
| Achar coisas no código do jogo sem endereço fixo | Busca por formato + checagem (o GPS da 1.62 já usa) | `crates/shared/src/navigator.rs`, `crates/plugin/src/navigation.rs` |
| Conversa app ↔ plugin | Memória compartilhada (bridge) | `crates/shared/src/lib.rs`, `crates/plugin/src/bridge.rs` |
| Configurações e tela no app do PC | Companion (páginas, settings) | `ui/src/companion/pages/*`, `crates/host/src/settings.rs` |

## 5. Como encaixa no WestPlay (arquitetura)

- **Plugin** (`crates/plugin`): um módulo novo `character/`, separado do resto:
  - `camera_control` — prende/solta a câmera 0 e põe ela onde o personagem precisa.
  - `controller` — estado do personagem (no caminhão, descendo, a pé, subindo…), posição, direção, velocidade.
  - `ground` — altura do chão sob o personagem (pela profundidade da imagem, §6 fase 4).
  - `render` — desenha o modelo com a perspectiva do jogo, num estado de D3D próprio (como `handset.rs`).
  - `animation` — esqueleto, clipes e transições (parado ↔ andando ↔ correndo…).
  - `input` — WASD/mouse enquanto a pé, sem o caminhão receber.
- **Bridge** (`crates/shared`): um bloco novo com "personagem ligado", teclas escolhidas e o estado atual
  (pro app mostrar e pra os celulares, se um dia quisermos).
- **App do PC (companion)**: uma página nova **Personagem** — ver §7.
- **Arquivos de arte**: modelos/animações numa pasta do WestPlay (ex.: `assets/character/`), nunca dentro do jogo.

Princípio: **tudo desligado por padrão** e **nada é feito se algo não for encontrado ou não bater** (como o GPS):
o jogo segue normal, e o log diz o que faltou.

## 6. Fases (passo a passo, cada uma testável)

Cada fase termina com um **teste no jogo** e um "**deu certo se…**". Só passamos pra próxima quando a atual
estiver ok. As fases 0 a 2 decidem se o projeto é viável; vale parar e reavaliar depois delas.

### Fase 0 — Investigação (sem mudar o WestPlay)
- Achar no jogo **onde fica a câmera 0**: posição, rotação (para onde olha) e como o jogo liga/desliga ela.
  (A câmera de desenvolvedor exige `g_developer`/console no `config.cfg`? Dá pra ligar sem isso?)
- Ver se dá pra **escrever** na posição/rotação dela a cada quadro sem o jogo "brigar".
- Achar as **matrizes de câmera** (vista e projeção) que o jogo usa pra desenhar — precisaremos delas pra desenhar
  o personagem no lugar certo.
- Ver se o **buffer de profundidade** do jogo é acessível no fim do quadro (formato, se é "invertido", resolução).
- Testar no **ETS2 e no ATS**, e anotar se o **TruckersMP** atrapalha.
- **Entrega**: um relatório `docs/personagem/INVESTIGACAO.md` com o que foi achado e como.
- **Deu certo se**: sabemos ler e escrever a câmera 0 e temos as matrizes. Se não der, o projeto para aqui.

### Fase 1 — Área "Personagem" no app (liga/desliga)
- Página nova no companion: **Personagem**, com **Ligar/Desligar** e as teclas (§7).
- O plugin só lê "ligado/desligado" e as teclas pela bridge; ainda não faz nada visível.
- **Deu certo se**: ligar/desligar no app aparece no log do plugin na hora, e sobrevive a reiniciar o app.

### Fase 2 — Câmera presa, andando sem personagem
- Tecla de **descer**: com o caminhão **parado e com freio de mão**, a câmera vai pra fora, do lado da porta, na
  altura de uma pessoa.
- **WASD** move a câmera no plano; **mouse** gira a vista; a altura fica fixa (por enquanto).
- O caminhão **não recebe** essas teclas enquanto a pé.
- Mesma tecla de novo: volta pra **câmera interna**.
- **Deu certo se**: dá pra andar em volta do caminhão a pé e voltar, várias vezes, sem o caminhão andar e sem
  travar o jogo. FPS igual ao normal.

### Fase 3 — Um boneco simples na cena
- Desenhar uma **forma simples** (caixa ou cápsula do tamanho de uma pessoa) na posição do personagem, usando as
  matrizes do jogo — terceira pessoa, câmera atrás dela.
- **Deu certo se**: a forma fica "presa no mundo" (não escorrega quando a câmera gira) e no tamanho certo ao lado
  do caminhão. FPS igual ao normal.

### Fase 4 — Chão e "ficar atrás das coisas"
- Ler a **profundidade** no ponto embaixo do personagem pra achar o **chão** (subir calçada, descer rampa).
- Usar a profundidade pra **esconder** a parte do boneco que estiver atrás de postes, carros, paredes.
- Suavizar a altura (sem tremer) e limitar degraus grandes (não "escala" paredes).
- **Deu certo se**: o boneco anda em calçada, asfalto e terra sem flutuar nem afundar, e some atrás de um poste.

### Fase 5 — Modelo 3D animado
- Trocar a forma pelo **modelo com esqueleto** (formato **glTF**), com animações: parado, andando, correndo.
- Transições suaves conforme a velocidade; luz simples coerente com o dia/noite do jogo.
- **Deu certo se**: o personagem anda e corre de forma natural, de vários ângulos. FPS: no máximo alguns quadros a
  menos que o normal.

### Fase 6 — Descer e subir do caminhão
- Animações de **descer** e **subir** pela porta do motorista, com a câmera acompanhando.
- Posição da porta por caminhão (a telemetria dá a cabine; ajustar por modelo de caminhão, com um padrão).
- Ao subir: volta pra câmera interna e **devolve os controles** ao caminhão.
- **Deu certo se**: descer e subir funcionam em pelo menos 3 caminhões diferentes, sem o personagem atravessar a
  porta de um jeito feio.

### Fase 7 — Primeira/terceira pessoa, agachar, sentar
- Tecla de **câmera**: alterna terceira pessoa ↔ primeira pessoa (na primeira, só braços/mãos, ou nada).
- **Agachar** e **sentar** (banquinho ou no chão), com animações.
- **Deu certo se**: alternar a câmera não dá "pulo", e agachar/sentar/levantar funcionam com WASD.

### Fase 8 — Extras (opcionais, cada um decidido na hora)
- Animação de **colocar o cinto** na cabine.
- Interface simples (dicas de tecla na tela).
- Mais de um personagem pra escolher (roupas, aparência).
- Ver o personagem nos **celulares** (ex.: botão de descer pelo celular do jogo).

## 7. A área "Personagem" no app do PC

Página nova no companion, ao lado das outras (Tela, Controles, Personalizar…):

- **Modo Personagem**: Ligado / Desligado (desligado por padrão).
- **Teclas**: Descer/Subir (ex.: K), Câmera 1ª/3ª pessoa (ex.: V), Agachar (ex.: C), Correr (ex.: Shift).
  Escolhidas como os atalhos de hoje (`ui/src/companion/pages/Controls.tsx`, `KeyButton.tsx`).
- **Personagem**: qual modelo (quando houver mais de um).
- **Avisos**: "só com o caminhão parado"; "pode não funcionar depois de uma atualização do jogo"; e, se a fase 0
  mostrar, "não funciona no TruckersMP".

## 8. Modelos e animações (licença)

- Usar só o que tiver **licença que permita colocar no WestPlay** e distribuir junto.
- Opções: **Mixamo** (Adobe, grátis para uso em jogos), personagens **CC0** (ex.: Quaternius, Kenney), ou um
  modelo feito pra nós.
- Guardar a licença de cada arquivo em `THIRD_PARTY_NOTICES.md`.
- Nada do mod de referência.

## 9. Riscos

- **Atualizações do jogo** podem mudar a câmera ou as matrizes de lugar (como a 1.62 fez com o GPS). Mitigação:
  achar tudo pelo formato do código e conferir, como `navigator.rs`; se não achar, o modo fica desligado sozinho
  e o resto do WestPlay continua.
- **TruckersMP**: pode bloquear a câmera livre ou não gostar de mudarmos a câmera. A testar na fase 0.
- **Desempenho**: desenhar um modelo animado por quadro precisa ser leve (o jogo já cai de FPS com o vídeo do
  YouTube — ver o log `performance` do app).
- **Chão pela profundidade** só "vê" o que está na tela: atrás da câmera ou muito longe, precisa de cuidado.
- **Caminhão andando**: nunca deixar descer com o caminhão em movimento.

## 10. Decisões em aberto (pra decidir antes ou durante)

- Primeira pessoa mostra **braços** ou **nada**?
- Correr: tecla segurada ou alternar?
- Um personagem só no começo, ou já com escolha?
- O que fazer se o jogador sair do caminhão longe da estrada (terreno sem profundidade confiável)?

---

## Como retomar (prompt pra colar numa sessão nova)

> Leia `docs/personagem/PLANO.md` (e `docs/personagem/INVESTIGACAO.md`, se existir). Estamos na **Fase N**.
> Faça só essa fase, siga a regra de originalidade (§2) e o princípio "desligado por padrão / não faz nada se não
> achar" (§5). Instale no meu PC pra eu testar no jogo, mas **não envie pro GitHub** até eu pedir. No fim, me diga
> como testar e o critério "deu certo se…" da fase, e anote no fim deste arquivo o que foi feito e o que ficou
> pendente.

## Registro do andamento

| Fase | Situação | Data | Notas |
|---|---|---|---|
| ▶ Próxima tarefa (andar a pé, só câmera) | investigação em andamento | 2026-11-04 | Ver `INVESTIGACAO.md`: o jogo tem uma câmera andando própria (`walking_camera_u`, nas cenas de configurar o caminhão); a câmera 0 guarda a posição em formato ainda não achado. Som de passos: depois. |
| 0 — Investigação | não começada | | |
| 1 — Área Personagem no app | não começada | | |
| 2 — Câmera presa | não começada | | |
| 3 — Boneco simples | não começada | | |
| 4 — Chão e oclusão | não começada | | |
| 5 — Modelo animado | não começada | | |
| 6 — Descer/subir | não começada | | |
| 7 — Câmeras, agachar, sentar | não começada | | |
| 8 — Extras | não começada | | |
