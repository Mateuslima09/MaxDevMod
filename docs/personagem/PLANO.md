# Modo Personagem — planejamento

Planejamento para colocar no WestPlay um **personagem 3D que desce do caminhão, anda a pé e volta pra cabine**,
no ETS2 e no ATS. Escrito em 2026-11-04, antes de começar. A ideia é seguir fase por fase, testando cada uma no
jogo antes de passar pra próxima.

---

## ▶▶ Onde estamos (2026-11-05) e a próxima conversa: polir o personagem

### O que já funciona (testado pelo dono no ETS2 e no ATS)

- **F8** com o caminhão parado e o freio de mão puxado: desce a pé, na câmera andando do próprio jogo
  (`walking_camera`), com chão e colisão pela física dele. F8 na porta do motorista: volta. Só single player.
- **Sem dano no caminhão**: a cápsula vai pra camada de colisão 9 (não bate em veículo) e ganha uma 2ª forma na
  camada 3 (bate no caminhão e no reboque sem dano). Automático no F8.
- **Shift e Ctrl** a pé não trocam a marcha (os dois lados escondidos do jogo). **V** troca 1ª ↔ 3ª pessoa.
- **3ª pessoa**: o personagem do próprio jogo (o homem `m_vanilla_ice_01`, fixo) nos seus pés, desenhado pelo
  jogo (luz, sombra) — o plugin cria a pessoa como o jogo cria os passageiros de ônibus, chama o "atualizar" dela a
  cada quadro e a desenha por um gancho na função de desenho do mundo (`INVESTIGACAO.md` §7.8 e registro).
- **Câmera** gira 360° em volta, lisa (o mouse a pé é do plugin; no menu/mapa volta pro jogo), em volta do peito,
  4,8 m atrás. O boneco só vira pra onde você anda (WASD).
- **Andar**: troca pra versão andando do mesmo homem (`mover.m_walk_06`, braços soltos); velocidade de pessoa
  (1,6 m/s; 3,6 com Shift); som de passo feito pelo plugin.
- Código: `crates/plugin/src/character.rs`, `character/person.rs`, `character/steps.rs`; `input.rs` (mouse),
  `hooks.rs` (`hook_now`). Instalado no PC do dono; nada no GitHub (só quando ele pedir).

### O que falta (teste do dono, 2026-11-05) — fazer nesta ordem

**Roteiro de teste para o dono: `TESTES.md`.**

**Feito em 2026-11-05 (instalado, falta o teste do dono; registro em `INVESTIGACAO.md`, etapa A):** (1) som do passo
**gravado** (12 passos recortados da gravação de asfalto que o dono deu), tocado por um fluxo próprio que mistura
os passos (sem cortar, sem estalo) — antes de publicar, confirmar a origem e a licença da gravação; (2) pés 1,8 m
abaixo dos olhos (2,25 afundou até os joelhos), lidos da câmera andando e da cápsula a cada quadro (agachado também); (3) correr a 2,6 m/s com a
animação de andar 1,6× mais rápida (o `run_01.pma` do ETS2 continua sem definição que o use: o caminho (a) abaixo
fica pra depois).
Depois, no mesmo dia (sem o jogo aberto): **pulo** com Espaço, **tecla de descer escolhida no app** e **escolher o
personagem** (32 pedestres dos dois jogos) na página Personagem; ponte v29 (app e plugin juntos).

1. **O som do passo está ruim**: "parece chutar uma lata", "estrala", "um golpe". Causas prováveis: o som é
   sintetizado (`steps.rs`: um seno grave de 70–100 Hz com ataque de 3 ms + ruído) — soa metálico/estalado; e o
   `PlaySound` assíncrono **corta** o som anterior no meio quando o próximo começa (estalo). Caminho: trocar por
   **gravações de passos de verdade com licença livre (CC0)**, várias por tipo de chão, anotadas em
   `THIRD_PARTY_NOTICES.md` (§8; baixar só com o "sim" do dono), e tocar com mistura (XAudio2 ou waveOut, sem
   cortar um som pelo outro), no volume do jogo. Depois: o passo pelo chão (o material físico da forma em que a
   cápsula encosta: tabela `exe+0x3674630`, `INVESTIGACAO.md` §7.7).
2. **O personagem flutua** (não encosta no chão). Hoje os pés são 1,7 m abaixo dos olhos da câmera andando
   (`FEET_UNDER_EYES`, um chute). Achar a altura certa: a câmera andando guarda as alturas da cápsula/olhos (os
   floats em `+0x660`/`+0x664` que o início dela usa) — calcular os pés pela base da cápsula, ou medir o chão
   (um raio pra baixo pela física do jogo). Conferir também agachado (Ctrl).
3. **Correr está feio**: é a animação de andar acelerada. Não existe um "correndo" do mesmo homem: no ETS2 só há
   `/model/mover/characters/animations/man/run/run_01.pma` (esqueleto antigo "man", o mesmo do `m_vanilla_ice_01`,
   nenhuma definição usa) e no ATS não há corrida de pessoa. Caminhos: (a) achar como o jogo troca a animação de
   uma instância (a ação `play_anim`, virtual 2 da instância, `0xa1e620`; o modelo animado em `instância+0xa8` e as
   funções `0x15df…`) e tocar o `run_01.pma` no ETS2; (b) se não der, ao correr, a animação de andar com o passo
   mais rápido mas a velocidade a pé menor (ex.: 2,6 m/s), pra não parecer acelerada; no ATS, idem.
4. Depois (já anotado acima, "Também pedido pelo dono" 7–10): câmera não entrar em paredes, escolher o
   personagem na página Personagem, personagem próprio com editor; aviso na tela do jogo (função de notificação
   `exe+0x623da0`, ícone 6 = freio de mão); tecla de descer escolhida no app.

### Prompt pra colar na conversa nova

> Leia `docs/personagem/PLANO.md` (o bloco ▶▶ "Onde estamos") e `docs/personagem/INVESTIGACAO.md` (§7, em
> especial §7.7, §7.8 e o registro da etapa A). O personagem 3D já anda em 3ª pessoa. Agora quero polir, nesta
> ordem: (1) o som do passo (está parecendo chutar lata e estala), (2) o personagem flutuando, sem encostar no
> chão, (3) a corrida, que hoje é a animação de andar acelerada. Rode `graphify query` antes de procurar no código
> (CLAUDE.md). Instale no meu PC pra eu testar (o jogo salvo, single player), mas não envie pro GitHub até eu
> pedir. Fale comigo em português, simples.

## ▶ Próxima tarefa: o personagem 3D, com o modelo do próprio jogo (planejado em 2026-11-04)

Já funciona (ver o registro no fim e `INVESTIGACAO.md`): descer com F8, andar a pé com a câmera andando do próprio
jogo (`walking_camera`, chão e colisão pela física dele), olhar com o mouse (câmera 0), voltar pela porta do
motorista, página Personagem no app. **Falta o corpo.** O dono pediu: primeiro em **primeira pessoa**, depois a
**terceira pessoa** usando a mesma lógica; e usar **o personagem do próprio jogo** (os pedestres e trabalhadores
que o ETS2 e o ATS já têm, como o de colete e capacete das empresas).

### Por que o modelo do jogo (e não um nosso)

É o mesmo raciocínio do caminho 1 (a câmera andando): deixar **o jogo** fazer o trabalho pesado.

- **O jogo desenha o boneco** como desenha os pedestres: com a luz do dia/noite, sombra, chuva e escondido atrás
  das coisas (poste, carro, parede). Com um modelo nosso (fases 3–5 antigas) teríamos de fazer tudo isso no
  DirectX, por cima da imagem — muito mais trabalho e nunca fica igual.
- **Cara de jogo**: o personagem parece do ETS2/ATS, não um corpo estranho.
- **Licença**: nada é copiado nem distribuído. O plugin só pede ao jogo do jogador pra mostrar um modelo que o
  próprio jogo já carrega (como um mod faz). Não entra arquivo da SCS no WestPlay. (§8 continua valendo pra
  qualquer coisa que um dia viermos a desenhar nós mesmos.)
- **ETS2 e ATS** têm pedestres próprios: cada jogo usa os seus.

O que o executável mostra (2026-11-04, ETS2 1.61): o sistema de **movers** — `mover_desc`, `mover_model`,
`mover_anim` (`/def/world/mover_anim.sii`), `mover_action`, `mover_instance`, `walker_item` (pedestres que andam
por caminhos do mapa), `pedestrian_desc`/`pedestrian_hookup` e `animated_model_data`. São os bonecos animados do
mapa (o trabalhador da foto do dono é um deles). O plano é **criar um desses pelo plugin e mandar nele**: onde
está, pra onde olha e qual animação toca.

### Etapa A — Investigação (só leitura, como a fase 0)

1. Nos arquivos do jogo (`def.scs`, `base.scs`; o WestPlay já lê esses pacotes em `crates/mapdata`): listar os
   `mover_desc`/`mover_model` de pessoas e as animações de cada um (`mover_anim`: parado, andando, correndo…; o
   modelo `.pmd`, o esqueleto e as animações `.pma`). Anotar quais servem (corpo inteiro, de frente pra câmera).
2. No executável: como o jogo **cria** um mover a partir de um `walker_item`/prefab ao carregar um setor do mapa,
   e como ele **mexe** nele a cada quadro (posição, rumo, animação). Achar pelo formato do código e pelas
   classes (`get_class`), como foi com a câmera andando.
3. No jogo aberto: achar um mover vivo perto do caminhão (pela classe), ler a posição dele (o formato de
   `+0x40` com setor de 512 m, provavelmente) e a animação que toca. Ver se dá pra **mudar a posição** dele e
   se o jogo aceita (o teste de escrita, com autorização do dono, salvo e em single player).
4. **Sons de passos**: ver se o jogo tem sons de passos nos pacotes de som dele (os bancos FMOD em `base.scs`;
   o executável não cita nenhum). Se não tiver, usar sons com licença livre (CC0), anotados em
   `THIRD_PARTY_NOTICES.md` (§8). Ver também se dá pra saber **o chão** sob o personagem (asfalto, terra,
   grama, metal) pela física do jogo, pra o passo soar diferente em cada um.
5. **O pulo não funciona** (teste do dono, 2026-11-04: Espaço não pula). Primeira olhada: o código da câmera
   andando só cita o controle de pular (`camwalk_jump`, `+0x440`) ao ser criada e destruída; a atualização dela
   (frente, trás, lados, correr, agachar) não o lê. Investigar: se outro código do jogo lê esse controle (a cena da
   garagem?) e faz o pulo; se não, **fazer o pulo nós**: um impulso pra cima no corpo físico dela (a cápsula em
   `+0x638`, pelas funções da física do jogo), só com os pés no chão, e a animação/som de pulo e de queda (etapas
   B e C).
6. **Deu certo se**: sabemos criar (ou pegar emprestado) um mover, pôr onde quisermos e trocar a animação dele. Se
   não der, vale o Plano B abaixo.

Plano B (se o jogo não deixar criar/mexer num mover): desenhar nós mesmos o **modelo do jogo** (ler o `.pmd`/
`.pma` do pacote do jogo do jogador e desenhar no DirectX com as matrizes da câmera, como as fases 3–5 antigas).
Mais trabalho; só se o plano A não der.

### Etapa B — Primeira pessoa

A câmera continua como hoje (olhos do personagem, na câmera andando). O corpo aparece **embaixo e em volta**:

1. Ao descer (F8), o plugin cria o boneco **nos pés da câmera andando** (a posição dela menos a altura dos olhos).
2. A cada quadro: o boneco vai pra posição da câmera andando e **gira pro rumo dela** (o `+0x66c` que já usamos).
3. **Animação pela velocidade**: parado / andando / correndo (Shift), pela distância andada no quadro; agachado
   e pulo pelos controles dela (`camwalk_crou`, `camwalk_jump`, que o plugin já aperta).
4. **Cabeça**: em primeira pessoa a cabeça do boneco fica dentro da câmera. Opções, nesta ordem: o modelo ter uma
   versão/"look" sem cabeça; a câmera um pouco à frente da cabeça (~15 cm); ou o jogo cortar o que está colado
   na câmera. Decidir na investigação.
5. Olhando pra baixo, você vê o corpo e os pés; olhando pros lados, os ombros. (Braços à frente, segurando
   coisas, ficam pra depois.)
6. **Como uma pessoa andando** (pedido do dono, 2026-11-04): a vista **balança de leve** no ritmo dos passos
   (sobe e desce, um pouco pros lados; mais forte correndo, nada parado), com opção de desligar na página
   Personagem.
7. **Som dos passos**, no ritmo da animação (cada pé que toca o chão, um passo; mais rápido correndo; um som ao
   cair de um pulo), e, se a investigação achar, diferente em cada chão. Tocado pelo app do PC (como a música do
   WestPlay), no volume do jogo; o plugin avisa cada passo pela ponte.
8. Ao voltar pro caminhão (F8 na porta), o boneco some.
9. **Deu certo se**: olhando pra baixo você vê o corpo andando, no ritmo dos seus passos, sem a cabeça na frente
   da vista; a vista balança como a de quem anda; cada passo se ouve; e o FPS não cai.

### Etapa C — Terceira pessoa (a mesma lógica, a câmera atrás)

1. O boneco é **o mesmo da etapa B** (posição, rumo e animação iguais); só a câmera muda.
2. A câmera 0 fica **atrás e acima** do boneco (ex.: 3 m atrás, 0,5 m acima da cabeça), girando em volta dele com
   o mouse (a rotação da câmera 0, como hoje). Andar continua indo pra onde a câmera olha; o **boneco vira** pro
   lado em que anda (suave, sem girar de uma vez).
3. **Câmera sem atravessar parede**: se houver uma parede/caminhão entre o boneco e a câmera, a câmera chega mais
   perto (pela física do jogo, como a cápsula da câmera andando, ou pela profundidade da imagem).
4. A **animação andando/correndo** e o **som dos passos** são os mesmos da etapa B (o boneco anda como uma pessoa,
   no ritmo dos passos que se ouvem); o balanço da vista fica só na primeira pessoa (na terceira, a câmera
   segue suave).
5. **Tecla pra trocar** 1ª ↔ 3ª pessoa (ex.: V; escolhida na página Personagem), e a escolha fica salva.
6. **Deu certo se**: em terceira pessoa você vê o personagem inteiro andando e correndo, ouvindo os passos, em volta do caminhão, a
   câmera não entra nas paredes, e trocar de câmera não dá "pulo".

### Também pedido pelo dono (2026-11-04)

1. **Escolher a tecla de descer/entrar no app** (página Personagem): hoje é F8 fixo. Pôr um botão de tecla como os
   atalhos da página Controles (`KeyButton.tsx`), salvo nas configurações (`character.key`), passado ao plugin pela
   ponte (um campo novo no `HostBlock`, a ponte sobe de versão) e mostrado no texto da página no lugar de "F8".
   Recusar uma tecla que o jogo ou o WestPlay já usam pra andar (W, A, S, D, Shift, Espaço, Ctrl, 0–9). Junto, de
   preferência: as teclas de sensibilidade (`-` e `=`) e, na etapa C, a de trocar 1ª/3ª pessoa. **Feito
   (2026-11-05)** só a de descer; falta o teste (`TESTES.md`).
2. **F8 que precisava ser apertado duas vezes** ("a câmera volta pra posição zero e só depois sai"): a causa não
   era a câmera virada. Ao entrar pela porta, o plugin guardava a hora daquele aperto de F8 e não a apagava; na
   descida seguinte, com o F8 ainda apertado no primeiro quadro, achava que ele estava segurado havia mais de 2 s
   e devolvia na hora pra cabine (o log mostrava "stopped walking (F8 held)" 16 ms depois de começar). **Corrigido**
   (a hora é apagada ao sair); falta o teste do dono.
3. **Trocar de câmera (2, 3, 4…) a pé te devolvia pro caminhão**: o jogo saía da câmera 0 e o plugin encerrava a
   caminhada. Agora, enquanto você anda, as teclas de câmera (1 a 9 e 0) não chegam ao jogo. **Corrigido**; falta
   o teste. Na etapa C, uma tecla (ex.: V, ou a de câmera do jogo) passa a trocar 1ª ↔ 3ª pessoa.

4. **No ATS o F8 "não fazia nada"**: era o freio de mão, que não estava puxado (o plugin recusa a descida sem
   ele, e só dizia isso no log). **Resolvido**: no ATS funciona. O log agora diz a velocidade e o freio de mão.
5. **Aviso na tela quando não dá pra descer** (ponto de atenção do dono): F8 sem o freio de mão puxado → um aviso
   "Puxe o freio de mão para descer do caminhão"; com o caminhão andando → "Pare o caminhão para descer"; e os
   outros motivos que hoje só vão pro log (modo desligado no app, TruckersMP, longe da porta ao voltar: "Chegue
   perto da porta do motorista para entrar"). Como mostrar, nesta ordem: (a) **o aviso do próprio jogo** (as
   notificações do canto da tela, como "@@notification_walking_camera@@", achando a função que o jogo usa pra
   mostrá-las — fica com a cara do jogo, no idioma dele se usarmos textos dele); (b) se não der, **um aviso do
   WestPlay desenhado por cima do jogo**, como o celular do jogo é desenhado (`handset.rs`), no idioma do app.
   Some sozinho depois de alguns segundos. **Como chamar o aviso do jogo: achado** (2026-11-05,
   `INVESTIGACAO.md` §7.9, ETS2 e ATS); falta implementar e testar.

6. **Encostar no caminhão a pé dá dano nele** (teste do dono, 2026-11-04: correndo ou andando contra o caminhão,
   "Danos no veículo 15% +4%"). **Prioridade** (custa dinheiro ao jogador, no save dele). Por quê: a câmera andando
   anda com um **corpo físico em cápsula** (`+0x638`), criado no **mundo físico da estrada**, o mesmo do caminhão;
   o jogo trata a batida da cápsula como a de outro veículo e calcula dano. Investigar, nesta ordem:
   (a) o que o início da câmera faz com a cápsula (`0x16cc000`: a função virtual `+0x1c0` com um número, talvez
   a massa; `0x1657fd0` e `0x1658040(.., 0x12)`, talvez o grupo/camada de colisão) e pôr nela uma **camada de
   colisão** que bate no caminhão sem causar dano, ou **massa quase zero** (o impulso fica abaixo do que conta como
   batida); (b) achar o código do jogo que transforma contato em dano e ver como ele ignora corpos que não são
   veículos; (c) em último caso, a cápsula não colide com o caminhão do jogador (passa por ele, mas não estraga).
   Testar só em single player, com o jogo salvo (e conferir o dano antes e depois). Até lá: **evitar encostar no
   caminhão** andando a pé. **Causa achada (2026-11-04, `INVESTIGACAO.md` §7.1)**: camada 18 da cápsula bate no
   caminhão; a massa não conta (é a velocidade, acima de 1,38 m/s). Proposta: pôr a cápsula na camada 9
   (`0x1658040(corpo, 9)` depois do início); falta o teste.

7. **Terceira pessoa primeiro** (2026-11-05: "na primeira pessoa não dá pra ver"): a etapa C vem antes da B.
   **Versão de teste instalada**: ao descer, o personagem do jogo fica nos seus pés, virado pra onde você olha, e a
   câmera 0 fica 3 m atrás e 0,4 m acima dos olhos, girando em volta dele com o mouse; **V** troca pra primeira
   pessoa (o boneco some) e volta (o V é o bloqueio do diferencial no jogo; a pé ele fica escondido do jogo).
   Falta: o teste do dono (pra que lado o boneco olha, a altura dos pés, a distância da câmera), a câmera não
   entrar em paredes, o boneco virar suave, e a animação andando/parada.
8. **Um personagem padrão, sempre o mesmo** (2026-11-05): não "aleatório". Hoje é o grupo `mover.work_idle`
   (trabalhador de colete) com semente fixa (0): o jogo escolhe sempre o mesmo modelo do grupo; a confirmar no jogo
   que é sempre o mesmo (o grupo também olha o país do lugar). Se variar, usar um `mover_desc` fixo.
9. **Escolher o personagem** na página Personagem do app (depois do 7 e do 8): a lista das pessoas dos pacotes do
   jogo do jogador (`INVESTIGACAO.md` §7.2/§7.7: colete, casuais, policial, fazendeiro…), salva nas configurações e
   passada ao plugin pela ponte. Os `mover_desc` têm `looks`/`variants` (cores e roupas de um mesmo modelo): dá pra
   oferecer "trocar a roupa" entre as do próprio jogo.
10. **Personagem próprio, feito do zero, com editor** (roupa, modelo…), pedido pra mais tarde. É outro caminho: o jogo
    só mostra os modelos dele (`.pmd` nos pacotes), então um personagem nosso seria (a) um mod com o nosso modelo
    num pacote `.scs` (ferramentas da SCS pra converter o modelo e as animações), ou (b) o Plano B, desenhado pelo
    WestPlay por cima do jogo. Os dois precisam de um modelo com licença livre (§8) e animações no esqueleto
    certo. Fica pra depois do 9. **Plano detalhado (2026-11-05): `PERSONAGEM_PROPRIO.md`** (com a
    animação de entrar no caminhão e o texto pra pedir ajuda a outras IAs).

### Depois (fica anotado, não entra agora)

- Animação de **descer e subir** do caminhão pela porta (fase 6 antiga).
- **Escolher o personagem** na página Personagem (os pedestres que o jogo tiver).
- Mostrar o boneco aos outros no TruckersMP: não (o modo é só single player).

### Cuidados (como até aqui)

- Desligado por padrão; nada acontece se algo não for achado (o log diz o quê).
- Cada teste de escrita na memória do jogo: só com o dono, em single player, com o jogo salvo.
- Achar tudo pelo formato do código (resiste a atualizações); ETS2 1.61 e ATS 1.62.
- Instalar no PC do dono pra testar; nada vai pro GitHub até ele pedir.

Prompt pra começar:

> Leia `docs/personagem/PLANO.md` (o bloco ▶) e `docs/personagem/INVESTIGACAO.md`. Faça a **etapa A**
> (investigação dos movers do jogo) e me conte o que achou antes da etapa B. Instale no meu PC pra testar, mas não
> envie pro GitHub até eu pedir. No fim, atualize o "Registro do andamento".

### Tarefa anterior (feita): andar a pé, só a câmera

Pedida em 2026-11-04 e feita no mesmo dia: área Personagem no app, câmera andando do jogo, WASD, mouse, sair pela porta do motorista e voltar por ela. Detalhes no registro abaixo e em `INVESTIGACAO.md`.

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
| ▶ Próxima tarefa (andar a pé, só câmera) | versão de teste instalada | 2026-11-04 | Ver `INVESTIGACAO.md`: o jogo tem uma câmera andando própria (`walking_camera_u`, nas cenas de configurar o caminhão); a câmera 0 guarda a posição em `+0x40` (setor de 512 m, §6). Som de passos: depois. Caminho 1: `crates/plugin/src/character.rs`, tecla **F8**, só single player, caminhão parado com freio de mão e câmera 0 na tela; ver `INVESTIGACAO.md` §6. 1º teste: altura ok; 2ª versão (andar pra onde se olha, caminhão sem WASD) ok; 3ª (F8 direto, sensibilidade do mouse) ok; opções na página Personagem do app (fase 1). Voltar pra cabine só perto da porta do motorista (segurar F8 2 s volta de qualquer lugar): instalado, falta testar. App (Ligar/Desligar) ainda não. |
| 0 — Investigação | feita | 2026-11-04 | `INVESTIGACAO.md` (câmera andando, câmera 0, mundo físico, classes). |
| ▶ Etapa A (personagem 3D) | parte sem o jogo feita (só leitura) | 2026-11-04 | `INVESTIGACAO.md` §7. **Dano**: a cápsula está na camada de colisão 18, que bate no caminhão (6, 7) e no reboque (8); o caminhão conta dano acima de 1,38 m/s e ignora as camadas 3 e 16; a massa não conta. Proposta: camada 9 (atravessa o caminhão, nunca dá dano) — a testar. **Movers**: o jogo cria bonecos na hora (passageiros de ônibus, `0xb95230`) e os põe no lugar com `0xa1c400` → plano A parece possível; falta a troca de animação e o teste de escrita. **Modelos**: nova geração (`meso_walking_01`, `meso_standing_idle_*`, sem correr) nos dois jogos; correr só no ETS2 (geração antiga, `run_01`). **Passos**: o jogo não tem (vai CC0). **Pulo**: o jogo não lê `camwalk_jump`; será nosso, pela velocidade da cápsula. **Aviso**: ainda não achado o das notificações; pistas anotadas. Nada instalado, nada no GitHub. Falta (com o dono e o jogo aberto): o resto da etapa A (item 3: mover vivo, animação, teste de escrita; o teste da camada 9). |
| ▶ Etapa A — 2ª rodada (complementos) | feita (só leitura) | 2026-11-04 | `INVESTIGACAO.md` §7.7: o material físico de cada forma (`exe+0x3674630`: asfalto, terra, grama, cascalho, concreto, metal…) dá o **chão sob os pés** pro som dos passos; a **notificação do canto do HUD** é `exe+0x623da0` (ícone 6 = freio de mão), falta o objeto e os argumentos; o trabalhador de **colete** da nova geração (`ngn_worker_vest` + `ngn_walk`) existe nos dois jogos. Velocidade da câmera andando: 4 m/s / 8 m/s correndo. Nada mudado no WestPlay. |
| ▶ Etapa A — dano no caminhão | resolvido (teste do dono ok) | 2026-11-04 | Testado pelo dono: com a cápsula na camada **9** e uma 2ª forma na camada **3** encosta e bate no caminhão **sem dano** ("ficou excelente"). Agora é automático: o F8 de descer põe as duas camadas, o F8 de subir solta a 2ª forma (o F9 do teste saiu). Log: `character: the capsule on layer 9`, `bumping on`. Versões antigas em `plugins/westplay.dll.antes-camada` e `.com-f9`. |
| ▶ Shift e Ctrl a pé trocavam a marcha | corrigido, falta o teste | 2026-11-04 | Teste do dono (ETS2 e ATS): a pé, Shift/Ctrl esquerdos (no perfil dele: `gearup`/`geardown`) engatavam marcha e ré. Causa: o plugin escondia o Shift/Ctrl pelo nome genérico, mas as mensagens e consultas do jogo vêm com o lado (Shift **esquerdo**). Agora os dois lados ficam escondidos do jogo enquanto anda (e o direito também no DirectInput); a câmera andando continua recebendo correr/agachar. Antiga em `plugins/westplay.dll.antes-shift`. |
| ▶ Terceira pessoa com o personagem do jogo | versão de teste instalada | 2026-11-05 | No lugar do teste do F9. Ao descer: o trabalhador de colete do jogo (`mover.work_idle`) nos seus pés, virado pra onde você olha, e a câmera 3 m atrás e 0,4 m acima dos olhos (o mouse gira em volta). **V** troca 1ª ↔ 3ª pessoa (na 1ª o boneco some). Ver "Também pedido pelo dono" 7–10 (padrão, escolher, personagem próprio). Antiga em `plugins/westplay.dll.com-f9-pessoa`. |
| ▶ 3ª pessoa — 1º teste | câmera ok; boneco não aparece | 2026-11-05 | No ATS a câmera atrás funcionou; no lugar do boneco, o "?" do jogo (sem modelo), parado na porta onde nasceu. Nova versão: personagem fixo `m_vanilla_ice_01` de braços cruzados (`mover.m_idle01` no ATS, `mover.idle_event01` no ETS2) e diagnóstico no log (`character: the person ...`). Antiga em `plugins/westplay.dll.terceira-1`. |
| ▶ 3ª pessoa — 2º teste | boneco criado mas nunca desenhado | 2026-11-05 | O modelo fixo (`m_idle01`) é criado, mas o jogo não atualiza nem desenha um mover sozinho: quem o tem é que faz, a cada quadro. Nova versão: o plugin chama o "atualizar" do jogo a cada quadro e entra (gancho) na função que junta o que o jogo desenha, pra desenhar o boneco junto (`INVESTIGACAO.md`, registro). Antiga em `plugins/westplay.dll.terceira-2`. |
| ▶ 3ª pessoa — o boneco aparece | versão de teste instalada | 2026-11-05 | Teste do dono (ETS2): o personagem do jogo aparece nos pés, com a câmera atrás. Ajustes pedidos e feitos: a câmera gira 360° em volta sem virar o boneco; o boneco só vira pra onde você anda (WASD), suave; câmera mais longe (4,5 m atrás, 0,6 m acima dos olhos). Falta: animação andando/parado, câmera não entrar em paredes. Antiga em `plugins/westplay.dll.terceira-3`. |
| ▶ 3ª pessoa — câmera lisa e mais baixa | feito (teste do dono ok) | 2026-11-05 | Teste do dono: "ficou lisinha", girar a câmera ok. Mouse a pé é do plugin (sem atraso). Depois: os pés cortados embaixo; a câmera passou a girar em volta do peito (0,45 m abaixo dos olhos), 4,8 m atrás e 0,5 m acima dele (mais ou menos a altura da cabeça), inclinação pra cima até 0,3 rad. Falta o teste desse último ajuste. Antiga em `plugins/westplay.dll.terceira-5`. |
| ▶ Polir: passo, pés, corrida | versão de teste instalada | 2026-11-05 | Som do passo novo (sem o tom de "lata", mistura sem cortar nem estalar), pés no chão (1,8 m abaixo dos olhos, lidos do jogo; 2,25 afundava), correr a 2,6 m/s com a animação de andar mais rápida. Falta o teste do dono. Antiga em `plugins/westplay.dll.terceira-7`. |
| ▶ Mouse no menu a pé | corrigido, falta o teste | 2026-11-05 | Teste do dono: depois da câmera lisa, o mouse não mexia no menu do jogo. Causa: a pé o plugin ficava com todo o mouse, também no menu. Agora só fica com ele com o jogo rodando; pausado (menu, mapa), o mouse volta pro jogo. Antiga em `plugins/westplay.dll.terceira-6`. |
| ▶ Andar com animação e passos | versão de teste instalada | 2026-11-05 | Andando, o personagem troca pra versão dele andando (braços soltos, `mover.m_walk_06`, nos dois jogos); parado, volta aos braços cruzados. Velocidade a pé de pessoa (1,6 m/s; 3,6 com Shift, ainda com a animação de andar). Som de passo a cada passada, feito pelo próprio plugin (sem arquivo de terceiros). Falta o teste; depois: correr de verdade, passo diferente por chão (o material da física), volume pelo jogo. Antiga em `plugins/westplay.dll.antes-andar`. |
| 1 — Área Personagem no app | feita | 2026-11-04 | Página **Personagem** no app do PC: Modo Personagem Ligado/Desligado (desligado por padrão; desligado, o F8 não faz nada) e Sensibilidade do mouse (10–200%, começa em 50%; `-` e `=` no jogo também mudam e o app guarda). Ponte v28: `character_enabled`/`character_sensitivity`. Ligado, o app põe `uset g_developer "1"` no `config.cfg` dos jogos com o jogo fechado (`crates/setup/src/developer.rs`). Textos nos 43 idiomas (sem revisão de nativos). |
| 2 — Câmera presa | feita (pela câmera andando do jogo) | 2026-11-04 | Caminho 1: ver a Próxima tarefa anterior e `INVESTIGACAO.md`. |
| 3 — Boneco simples | trocada | 2026-11-04 | Pelo plano do ▶: o modelo do próprio jogo (movers), etapa A. |
| 4 — Chão e oclusão | em parte | 2026-11-04 | Chão e colisão: pela física da câmera andando. Esconder atrás das coisas: o jogo fará, com o modelo dele (▶). |
| 5 — Modelo animado | trocada | 2026-11-04 | Etapas B (1ª pessoa) e C (3ª pessoa) do ▶. |
| 6 — Descer/subir | não começada | | |
| 7 — Câmeras, agachar, sentar | não começada | | |
| 8 — Extras | não começada | | |
