# Modo Personagem — relatório de teste (para o dono)

O que já está instalado no PC e ainda não foi testado no jogo. Testar no **single player, com o jogo salvo**,
com o **app WestPlay aberto** (sem o app, a tecla de descer não faz nada: o log diz "the WestPlay app isn't open").
Marque cada item e anote o que achou; a versão anterior de cada mudança está guardada (ver "Voltar atrás").

Instalado em 2026-11-05 (app, página do app e plugin, ETS2 e ATS). Nada foi para o GitHub.

## Antes de começar

- [ ] O app abre e a página **Personagem** mostra: o Modo Personagem, a **Tecla de descer e entrar** (F8), a
      sensibilidade do mouse e a lista **Quem anda** (Homem 1 a 8, Mulher 1 a 24; marcado: Homem 6).
- [ ] O Modo Personagem está ligado.

## 1. Pés no chão

Antes: flutuava (1,7 m), depois afundou até o joelho (2,25 m). Agora: 1,8 m abaixo dos olhos.

- [ ] Em pé, parado: os pés encostam no chão (nem no ar, nem enterrados). Se não, mande uma foto.
- [ ] Andando e correndo: continua no chão.
- [ ] Agachado (Ctrl): os pés continuam no chão (o boneco fica em pé, só a câmera desce: não há animação de
      agachar).
- [ ] A câmera atrás ainda mostra o corpo inteiro (não corta os pés).

## 2. Som do passo (a gravação de asfalto que você mandou)

- [ ] Andando: soa como pisar, não como chutar; sem estalo.
- [ ] Correndo: os passos acompanham (mais rápidos), sem um cortar o outro.
- [ ] Volume: bom ao lado do som do jogo? (Muito alto / muito baixo: anote.)
- [ ] O som para quando você para.

## 3. Correr (Shift)

Agora: 2,6 m/s com a animação de andar 1,6× mais rápida (o jogo não tem animação de correr).

- [ ] Os pés não escorregam no chão.
- [ ] Parece uma corrida leve / passo apressado? Ou ainda "andar acelerado"? (Anote.)

## 4. Pulo (Espaço) — novo

O jogo não tem pulo: o plugin dá um impulso pra cima (uns 45 cm) quando os pés estão no chão.

- [ ] Espaço pula; segurar Espaço não pula de novo (só um pulo por aperto).
- [ ] No ar, Espaço não pula de novo (sem pulo duplo).
- [ ] Ao cair: um som de passo.
- [ ] Pular perto do caminhão: **sem dano** (confira o dano antes e depois).
- [ ] Altura: boa, alta ou baixa demais? (Anote.)
- [ ] O boneco fica com a pose de parado no ar (não há animação de pulo) — esperado.

## 5. Tecla de descer escolhida no app — novo

- [ ] Na página Personagem, clique em **F8** e aperte outra tecla (ex.: F7): o botão e o texto mudam para F7.
- [ ] Teclas de andar (W, A, S, D, Shift, Espaço, Ctrl, V, -, =, 0–9) são recusadas com o aviso "Essa tecla já é
      usada para andar a pé".
- [ ] No jogo: a nova tecla desce; perto da porta do motorista, entra; segurando 2 s, volta de qualquer lugar.
- [ ] F8 não desce mais (com outra tecla escolhida).
- [ ] Volte para F8 no fim, se preferir.

## 6. Escolher o personagem — novo

Os 32 pedestres que andam nos dois jogos (os mesmos no ETS2 e no ATS). Vale na próxima vez que você descer.

- [ ] Escolha outro (ex.: Mulher 3), desça: aparece a pessoa escolhida.
- [ ] Andando: anima normal.
- [ ] Parado: só 5 têm pose de parado — **Homem 6** (os dois jogos) e, só no **ETS2**, Homem 1, Homem 5,
      Mulher 8 e Mulher 17. Os outros **congelam a animação de andar** onde pararam (pode ficar no meio do
      passo). Diga se isso fica aceitável ou se prefere só os que têm pose de parado.
- [ ] Se um personagem não aparecer: o log diz "isn't in this game: the default person" e volta pro Homem 6.
- [ ] Teste pelo menos um homem e uma mulher no ETS2 e no ATS.

## 7. Subir no caminhão em primeira pessoa — novo

Ao apertar a tecla de descer **perto da porta do motorista**, o boneco some e a câmera "sobe" como se fosse você:
vai até a porta, sobe o degrau (com som de passo), passa pela porta e senta no banco (uns 2,5 s). Aí o jogo volta
pra câmera da cabine.

- [ ] Funciona vindo da terceira pessoa e da primeira (V).
- [ ] O movimento é suave, sem tranco; parece subir um degrau.
- [ ] A altura no fim bate com a cabine (a troca pra câmera do jogo não dá um "pulo" grande). Se der, anote o
      caminhão (cabine alta/baixa).
- [ ] Não atravessa coisas estranhas (ex.: o retrovisor) — anote se atravessar.
- [ ] Apertar a tecla de novo durante a subida entra na hora.
- [ ] Segurar a tecla 2 s longe da porta continua entrando direto (sem subida).
- [ ] Teste no ETS2 e no ATS, de preferência com caminhões diferentes.

## 8. Avisos na tela do jogo — novo (cuidado: primeiro teste no jogo salvo)

O plugin chama o aviso do próprio jogo (o do canto da tela, como "Pare o veículo!"), no idioma do jogo.

- [ ] Sem freio de mão, aperte a tecla de descer: aparece **"Puxe o freio de mão para descer"** com o ícone de
      freio de mão.
- [ ] Com o caminhão andando: aparece **"Pare o veículo!"** (o texto do próprio jogo).
- [ ] A pé, longe da porta, aperte a tecla: **"Chegue perto da porta do motorista para entrar"**.
- [ ] Com o app fechado: **"Abra o app WestPlay para descer do caminhão"**.
- [ ] Apertar várias vezes **não empilha** avisos (o novo troca o anterior).
- [ ] O jogo não fecha nem trava. **Se fechar**, volte a DLL `antes-aviso-parede` (abaixo) e me mande o log.
- [ ] Com o Modo Personagem desligado: nenhum aviso (de propósito: a tecla pode ser sua pra outra coisa).
- [ ] Em outro idioma do jogo (ex.: inglês): o aviso sai nesse idioma.

## 9. A câmera não atravessa paredes — REMOVIDA (decisão do dono, 2026-11-05)

Fechou o jogo duas vezes no ETS2 (dentro da física do jogo, no raio que o plugin lançava ao ficar em 3ª pessoa),
mesmo esperando 1 s. O dono decidiu tirar: a câmera em 3ª pessoa atravessa paredes, como os jogos sem colisão de
câmera. O código ficou guardado como pesquisa em `docs/personagem/antigo/ray.rs.txt`, fora do plugin.

- [ ] Em 3ª pessoa (V), andar perto de paredes e do caminhão: o jogo **não fecha** mais.

## 10. Barra de teclas no jogo — novo (app e plugin novos, ponte v30)

Embaixo da tela, no meio, uma barra mostra as teclas do que dá pra fazer agora (imagem de exemplo:
`../barra-de-teclas.png`).

- [ ] Na cabine: sempre **[Alt] Usar a tela** e **[Alt + 4] Celular** (as suas teclas do app).
- [ ] Parado e com freio de mão puxado (Modo Personagem ligado): aparece também **[F8] Descer do caminhão**; solte
      o freio ou ande: some.
- [ ] A pé, perto da porta do motorista: **[F8] Entrar no caminhão**; longe: só o celular.
- [ ] Subindo no caminhão: a barra some.
- [ ] Trocou a tecla de descer (ex.: F7) ou a do celular no app: a barra mostra a nova.
- [ ] No menu / pausado: a barra some.
- [ ] Página Personagem: **Barra de teclas no jogo** desligada → some no jogo; ligada → volta.
- [ ] **Tecla de mostrar e esconder**: escolha uma (ex.: F9); no jogo, ela esconde e mostra a barra. Backspace
      volta pra "Nenhuma". Teclas de andar e a de descer são recusadas.
- [ ] O tamanho e o lugar ficam bons (não cobrem nada do jogo)? Anote se quiser maior/menor/mais pra cima.
- [ ] O jogo continua liso (sem travadas) com a barra.

## 11. Lanterna (L, a pé) — DESLIGADA por enquanto (decisão do dono, 2026-11-05)

> Nenhuma luz do jogo acendeu num mover do plugin (nem `light_source`, nem `flare_blink`, nem o `mover.light_flare`
> do próprio jogo). O L e o "Lanterna" saíram da barra; a pé o L volta a ir pro jogo (os faróis), como na cabine.
> O código ficou (`FLASHLIGHT_ON` em `character.rs`); o que foi testado está abaixo e em `INVESTIGACAO.md` §7.11.

> 2026-11-05, 3º teste: de noite, L liga ("Apagar a lanterna" na barra, o log diz "flashlight on") mas **não
> ilumina nada**. Até as 20:19 o jogo recusava o arquivo (`mover.wp_flashlight`, nome longo demais: agora
> `mover.wp_torch`, e o log mostra que ele aparece). O único mover do jogo com ganchos (`light_flare`) só tem
> brilhos (`flare`), então não se sabe se o jogo acende uma `light_source` num mover. **Teste agora**: a lanterna
> virou uma luz **em volta** (não um cone), forte e curta (8000 lúmen, 12 m), só pra saber:
> - [ ] De noite (`g_set_time 22` no console), a pé, **L**: o chão e o caminhão **em volta de você** ficam
>       claros? → **sim**: a luz funciona, falta apontar o cone (eu volto pro cone, na direção certa).
>       **não**: o jogo não acende luz num mover; preciso de outro caminho (me avise).
>
> **Resultado (6º teste, jogo das 21:07 com o mod das 20:57): não acende.** O jogo não acende uma `light_source`
> presa num mover. A lanterna precisa de outro caminho (ainda não feito).
>
> **2º caminho (21:20, app novo):** o único mover do jogo com ganchos usa **brilhos** (`flare_blink`), e um
> `flare_blink` pode levar uma luz junto (o jogo tem: `flare.model.red.arrow2.blink0`, com `type: spot`, lúmen e
> alcance). Agora a lanterna é um `flare_blink` sempre aceso, com um brilho branco pequeno e a mesma luz forte em
> volta (8000 lúmen, 12 m). **Feche o jogo e espere ~30 s** (o app refaz o mod só com o jogo fechado), abra, de
> noite, a pé, **L**:
> - [ ] Aparece um **brilho branco** na mão? (sim = o gancho funciona no mover)
> - [ ] O **chão em volta** clareia? (sim = a luz funciona; aí eu faço o cone apontado pra frente)
>
> **Resultado (7º teste, 21:36, mod das 21:31): nada**, nem o brilho. Como o mover, o modelo e o gancho estão iguais
> aos do jogo (`mover.light_flare`), o mais provável é que o jeito de o plugin criar o mover (o dos passageiros) não
> crie os ganchos do modelo. **Teste decisivo (21:42)**: a lanterna (L) agora cria o próprio **`mover.light_flare`
> do jogo** (dois brilhos vermelhos piscando):
> - [ ] Aparecem **brilhos vermelhos piscando** na mão? **sim**: o plugin cria ganchos, o erro é nos nossos
>       arquivos. **não**: o jeito de criar não faz ganchos; é preciso achar no código do jogo como ele os cria.

A lanterna é uma luz do próprio jogo (do mesmo tipo dos holofotes dos postos), presa num objeto invisível do
WestPlay que fica na sua mão direita e aponta pra onde você olha. A tecla é **L** (fixa por enquanto; a pé o jogo
não recebe o L, então os faróis do caminhão não mudam). O app já refez o mod com ela (`westplay.scs` de 14:36).

- [ ] **Feche e abra o jogo** (o mod e o plugin novos só entram ao abrir).
- [ ] À noite, a pé, aperte **L**: o log diz **"flashlight on"** e aparece um cone de luz à sua frente.
- [ ] A luz aponta **pra onde você olha** (olhe pro chão, pra parede, pro caminhão). Se apontar pra outro lado
      (pra cima, pra baixo, pros lados), me diga pra onde: é só um ajuste.
- [ ] Ela acompanha quando você anda, corre e pula.
- [ ] **L** de novo apaga ("flashlight off"). A barra de teclas mostra **[L] Lanterna** / **[L] Apagar a
      lanterna**.
- [ ] Subir no caminhão apaga a lanterna sozinha.
- [ ] Força da luz: forte/fraca demais? Alcance (uns 30 m)? Largura do cone? Anote.
- [ ] Se **nada acender**: o log diz "flashlight on" ou "no flashlight (…)"? Mande essa linha. (Pode ser que o jogo
      não acenda luz num objeto desses; aí a gente tenta o outro caminho.)
- [ ] O jogo não fecha nem trava ao apertar L. **Se fechar**, volte os arquivos `antes-lanterna` (abaixo).
- [ ] Teste no ETS2 e no ATS.

## 12. Ajustes do teste de 2026-11-05 (barra, porta, passos) — novo

- [ ] **Barra de teclas** menor (letras ~14 px em 1080p) e colada embaixo da tela. Tamanho bom agora?
- [ ] **Porta**: o ponto de subir/descer foi 0,55 m pra frente (antes ficava 0,3 m atrás da cabeça do motorista,
      agora 0,25 m à frente, no degrau). F8 perto da porta entra? A subida sai do rumo da porta?
- [ ] **Passos**: som novo de andar (concreto) e de correr (outro som, mais curto). Soa bem? Alto/baixo?
- [ ] Se piorar: volte `westplay.dll.antes-passos` nos dois jogos.

## 13. Personagem próprio (o da IA + Mixamo) — novo, primeiro teste (app e plugin novos)

É o teste que decide tudo: o jogo aceita um `mover_model` nosso, com esqueleto nosso? Os arquivos convertidos
estão em `%LOCALAPPDATA%\WestPlay\personagem` (fora do código e das versões publicadas: as animações são do
Mixamo, só pra testar). O app põe esses arquivos no `westplay.scs` ao montar o mod. No ETS2 já foi feito. No ATS,
o mod é refeito quando o app estiver no ATS.

> 1º teste (2026-11-05, 20:13): o jogo fechou ao descer. Não era o personagem: a escolha salva era o homem padrão
> (`"person": 5`), e quem fechou o jogo foi o raio da câmera (item 9, agora desligado). E o jogo tinha recusado o
> arquivo dos nossos movers: o nome `mover.wp_player_walk` é longo demais (cada parte tem no máximo 12 letras).
> Agora são `mover.wp_walk`, `mover.wp_idle` e `mover.wp_run`. A lanterna tinha o mesmo erro (`mover.wp_flashlight`,
> agora `mover.wp_torch`), então ela também não aparecia antes.

- [ ] No app, página **Personagem → Quem anda**: o último da lista é **WestPlay**. Escolha ele (o tique fica
      nele; em `%APPDATA%\app.westplay\settings.json` fica `"person": 32`).
- [ ] Depois de abrir o jogo, o `Documents\Euro Truck Simulator 2\game.log.txt` não tem mais "is in wrong format"
      para `mover.westplay_player.sii` nem para `mover.westplay.sii`.
- [ ] No jogo, desça (F8). Aparece o nosso personagem (boné azul, camiseta azul, calça preta)? Ou o homem padrão
      do jogo? (O homem padrão quer dizer que o jogo não achou o nosso: mande o `plugin.log`.)
- [ ] **Parado**: ele fica de pé com a animação de parado (balança um pouco; a mão direita meio fechada).
- [ ] **Andando** (W): anda com a animação de andar, pés no chão, virado pra frente (não de costas, não de lado).
- [ ] Tamanho certo (1,80 m, do tamanho dos pedestres)? Textura com cor (não preto, não branco)?
- [x] 2º teste (2026-11-05): ele apareceu, com textura, andando e parado. Pedidos do dono depois: descer em 1ª
      pessoa, a tecla da câmera na barra, a câmera nas paredes (item 9) e o som no passo. Feito (só o plugin novo,
      vale ao reiniciar o jogo):
- [x] **Desce em 1ª pessoa**; **V** muda para 3ª (e volta). A barra mostra **V Câmera** a pé. (3º teste: ok.)
- [ ] **Correndo (Shift)**: agora com a animação de **correr** de verdade (não a de andar acelerada).
- [x] **Som dos passos no pé** (3º teste: "melhor agora"): o som sai quando o pé da animação encosta no chão
      (andar: a cada ~0,52 s; correr: a cada ~0,35 s).
- [ ] **Volume dos passos mais baixo** (4ª versão, pedido do 3º teste: "um pouco alto, principalmente correndo"):
      andar a 60% do que era, correr a 40%. Bom agora? Ainda alto ou baixo demais?
- [x] ~~Parar suave com freio e "terminar o passo"~~ (5ª versão): o 6º teste achou pior ("continua andando,
      escorregando parado"). Tirado.
- [ ] **Parar sem piscar** (6ª versão): o que incomodava era a **piscada** ao parar. Causa provável: cada
      animação é um boneco separado, e o plugin só desenhava o da vez; o jogo faz um boneco que volta a ser
      desenhado "aparecer" (como os pedestres), e isso pisca. Agora os três (parado, andando, correndo) são
      desenhados sempre, e os que não estão em uso ficam 50 m embaixo do chão; na troca, só mudam de lugar.
      Parar e voltar a andar várias vezes: ainda pisca? (A troca de pose continua imediata, porque o jogo não
      mistura animações; se o "parado" parecer estátua, dá pra trocar a animação de parado no Mixamo por uma
      com mais movimento, como "Breathing Idle".)
      (Voltar: `westplay.dll.antes-piscar` nos jogos.)
      **7º teste: piscou mais.** Os bonecos embaixo do chão foram tirados.
- [ ] **Parar sem piscar** (7ª versão): a causa provável agora é outra. O plugin decidia "andando ou parado" a
      cada quadro pela distância daquele quadro (acima de 0,3 m/s); parando (e nos quadros em que a física não
      anda), alternava andando/parado/andando por alguns quadros, trocando o boneco a cada vez. Agora só passa
      a parado depois de 0,15 s parado (e a andando depois de 0,08 s andando). Pisca ainda?
      (Voltar: `westplay.dll.antes-sem-piscar` nos jogos.)
      **8º teste: pisca igual.**
- [ ] **Piscada: log das trocas** (8ª versão, 21:52): o plugin escreve no `plugin.log` cada troca de boneco
      (`character: shown standing` / `walking` / `running`). Em 3ª pessoa, ande e pare umas 5 vezes, devagar,
      e me avise: pelo log dá pra ver se cada parada tem uma troca só (a piscada é a própria troca) ou várias.
      **Resultado (21:54): uma troca por parada** (sem vai-e-vem): a piscada é a própria troca de boneco. E parando
      de correr eram duas (correndo → andando 0,15 s → parado).
- [x] **Piscada: sobreposição** (9ª versão, 21:57; 9º teste: "ficou bem legal"): na troca, o boneco anterior continua desenhado mais 2 quadros
      junto com o novo (o novo sai errado no 1º quadro). E parando de correr vai direto pra parado. Pisca ainda?
      (Voltar: `westplay.dll.antes-sobrepor` nos jogos.)
- [ ] **Correr um pouco mais rápido**: 3,2 m/s (era 2,6). Os pés escorregam no chão?
- [ ] **Descer do caminhão** (F8): a câmera sai do banco, vai até a porta olhando pra baixo, desce o degrau e
      chega no chão (~1,8 s, passo no degrau e no chão), e aí você anda. F8 de novo no meio pula pro chão.
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-descida`, `westplay.dll.antes-descida` e a
      pasta `ui.antes-descida`; nos jogos, `westplay.dll.antes-descida`.
- [ ] **Descer de novo depois de subir** (5º teste, 20:44, com a versão das 20:41: "às vezes desce, às vezes não;
      o personagem desce mas a câmera fica na cabine"). Causa: ao subir, a câmera 0 terminava na cabeça do
      motorista, no mesmo lugar da câmera da cabine; ao descer de novo, o plugin achava que a 0 já estava na tela,
      não apertava a tecla 0, e a tela ficava na cabine. Agora, ao voltar pra cabine, a câmera 0 fica 3 m fora da
      porta. Teste: descer, subir, descer, subir várias vezes seguidas. A câmera sempre sai junto?
      (Voltar: `westplay.dll.antes-estacionar` nos jogos.)
- [ ] Se piorar: na pasta `plugins` de cada jogo, volte `westplay.dll.antes-sem-colisao` (antes de tirar a
      colisão e baixar o volume) ou `westplay.dll.antes-camera` (antes da 1ª pessoa e dos passos novos), com o
      jogo fechado.
- [ ] Se der errado: escolha outro em "Quem anda" (o jogo volta ao normal), ou volte os arquivos
      `.antes-personagem` (abaixo).

## 14. A mulher e a escolha por foto — novo (etapa 1 de 3; app, plugin e mod novos, 22:44)

A mulher (`wm_`, 1,68 m, do Tripo + Mixamo) convertida como o homem, com andar, correr e parado (e já no mod, mas
ainda sem uso: parar, celular parada e celular andando, das etapas 2 e 3). Os 32 personagens do jogo saíram.

- [ ] No app, **Personagem → Quem anda**: dois **cartões com foto** (Homem 1, Mulher 1), o escolhido com borda azul
      e tique. A escolha antiga (32 = o homem) virou o Homem 1, o primeiro.
- [ ] Escolha a **Mulher 1**, desça (F8), V pra 3ª pessoa: é ela? Textura certa, 1,68 m, de frente pra onde anda?
- [ ] Andando, correndo e parada com as animações dela; os passos no pé (o ritmo dela foi medido à parte).
- [ ] Volte pro Homem 1: continua igual a antes.
- [ ] Se os arquivos não estiverem no PC, quem desce é o homem do jogo (`m_walk_06`), sem erro.
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-mulher`, `westplay.dll.antes-mulher` e a pasta
      `ui.antes-mulher`; nos jogos, `westplay.dll.antes-mulher`.

## 15. Corpo em primeira pessoa e balanço da câmera — novo (etapa 1b; app, plugin e mod novos, 23:01)

Em 1ª pessoa agora aparece o **corpo sem cabeça** do personagem (o mesmo, com a cabeça e o pescoço tirados no
Blender), virado pra onde você olha e 12 cm atrás dos olhos; a câmera fica na altura dos olhos dele (1,64 m o
homem, 1,56 m a mulher; antes ficava a 1,80 m, no alto da cabeça) e **balança com os passos** (sobe e desce duas
vezes por ciclo, o mais baixo quando o pé encosta; de lado uma vez), mais correndo, nada parado.

- [ ] Desça: depois da descida (no chão), olhe pra baixo: aparecem o peito, os braços e as pernas? Andando, as
      pernas andam? Sem "buraco" do pescoço ou pedaço de cabeça no meio da vista?
- [ ] Olhando reto, nada do corpo atrapalha a vista?
- [ ] O balanço: natural ou demais (enjoa)? Correndo, mais forte. Parado, para suave.
- [ ] A altura da vista parece a de uma pessoa (não alto demais)? Com a mulher, um pouco mais baixa.
- [ ] V: 3ª pessoa continua igual (corpo inteiro, com cabeça); V de novo volta ao corpo sem cabeça.
- [ ] Subir e descer do caminhão: durante o caminho da câmera, sem corpo; no chão, ele aparece.
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-corpo` e `westplay.dll.antes-corpo`; nos
      jogos, `westplay.dll.antes-corpo`.
- [x] 1º teste (23:03): 3ª pessoa "excelente"; em 1ª pessoa o corpo **ficava parado**, só girando. Causa: o corpo sem
      cabeça tinha um esqueleto próprio (mesmos ossos, outro nome), e o jogo só toca uma animação no modelo do
      mesmo esqueleto dela. Agora usa o esqueleto do modelo inteiro.
- [ ] **Feche o jogo e espere ~30 s** (o app refaz o mod só com o jogo fechado). Em 1ª pessoa, o corpo anda,
      corre e fica parado com as animações?
- [ ] **Passos mais baixos de novo**: andar a 45% (era 60%), correr a 30% (era 40%). Bom agora?
      (Voltar: `westplay.dll.antes-esqueleto` nos jogos.)

## 16. Pulo da mulher — novo (plugin, app e mod novos, 23:21)

A animação "Jump" que o dono baixou (2,6 s: agacha, sai do chão, aterrissa agachando, levanta). No Blender o quadril
não sobe acima de em pé (quem sobe é a física do pulo do plugin, ~0,45 m); ao apertar Espaço ela começa já no
impulso (sem o agachamento de antes, que ficaria com a cápsula já no ar); toca uma vez até o fim; se você sair
andando depois de cair, ela é cortada. Só a mulher (o homem não tem animação de pulo ainda: pula como antes).

- [ ] **Feche o jogo e espere ~30 s** (o app refaz o mod só com o jogo fechado).
- [ ] Mulher, 3ª pessoa, parada: Espaço. Ela pula com as pernas encolhendo e cai agachando? Sobe junto com a física
      (sem subir o dobro nem afundar)?
- [ ] Pular andando e correndo: o pulo aparece; depois de cair, sai andando normal?
- [ ] 1ª pessoa: olhando pra baixo, as pernas encolhem no pulo?
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-pulo` e `westplay.dll.antes-pulo`; nos jogos,
      `westplay.dll.antes-pulo`.
- [x] 1º teste (23:25): "primeiro pula, depois ativa a animação". A animação começava no empurrão (ainda no chão)
      com a cápsula já no ar, e o pouso tinha hora fixa.
- [ ] **2ª versão (23:27, só o plugin; reinicie o jogo)**: a animação segue a física. Ao apertar Espaço já começa
      **no ar** (os pés saindo do chão), vai até o alto com as pernas encolhidas e **segura ali** enquanto a física a
      mantém no ar; quando ela **encosta no chão de verdade**, vai pro pouso (agacha e levanta). Pula e cai junto?
      Pulando de um degrau ou caindo de mais alto, ela continua no ar até cair?
      (Voltar: `westplay.dll.antes-pulo2` nos jogos.)
- [x] 2º teste (23:33): andando, ela fazia o impulso de quem está parada e o pouso todo; correndo, Espaço "não
      fazia nada".
- [ ] **3ª versão (23:43; app, plugin e mod novos)**: parada, o pulo parado (como antes). **Andando ou correndo**,
      o **pulo correndo** que o dono baixou ("Jump correndo certo": 1 s, sem impulso, no lugar, quadril travado):
      já sai no ar, segura no alto enquanto a física a mantém no ar e, **ao encostar no chão, volta na hora pra
      andar/correr** (sem agachar). Ficou natural andando e correndo?
- [ ] Correndo, se Espaço ainda não pular: o `plugin.log` agora diz cada Espaço (`character: Space (running …)`) e,
      se não pulou, por quê (`character: no jump: not on the ground yet …`). Me avise que eu leio.
      (Voltar: `.antes-pulo-correndo` em `%LOCALAPPDATA%\WestPlay` e nos jogos.)
- [x] 3º teste (23:50): parada, "o cilindro sobe, desce, e depois a animação"; andando e correndo, "parece que
      estou escalando", não fluido. Causa: a animação ficava congelada no alto enquanto a física a mantinha no ar
      (uma pose parada, com um joelho à frente) e pulava pro pouso quando a física caía.
- [ ] **4ª versão (23:56, só o plugin; reinicie o jogo)**: a animação toca direto, sem congelar, e **a física
      segue a animação**. Andando/correndo: o pulo correndo começa no Espaço, junto com a física (os dois ficam
      ~0,6 s no ar), e volta a andar/correr ao cair. Parada: ao apertar Espaço ela **agacha primeiro** e a física
      sobe quando os pés saem do chão (~0,3 s depois), caindo junto com o pouso da animação. Fluido agora? O atraso
      de 0,3 s parada atrapalha?
      (Voltar: `westplay.dll.antes-pulo3` nos jogos.)

## 17. Agachar (Ctrl), pulo parado sincronizado, pulo correndo — novo (app, plugin e mod novos, 00:08)

O agachar da câmera do jogo não fazia nada (owner: "a função de agachar nem tá funcionando"); agora é do plugin.

- [ ] **Feche o jogo e espere ~30 s** (o mod com a animação de agachar da mulher só entra assim).
- [ ] A pé, a **barra mostra "Ctrl Agachar"** (junto de V Câmera).
- [ ] **Segurando Ctrl**: a vista desce devagar (~0,45 m em 1ª pessoa, metade em 3ª), anda mais devagar (0,9 m/s);
      a mulher desce no agachamento ("Air Squat") e **fica embaixo** enquanto segura; **soltando**, ela levanta e a
      vista sobe. O homem só abaixa a vista (sem animação ainda).
- [ ] **Pulo parado** (mulher): agacha, sai do chão junto com o cilindro e, se a física ficar mais no ar, ela
      **espera com as pernas pra baixo** e pousa quando o cilindro pousa. Sincronizado agora? O log diz o tempo no
      ar de cada pulo (`character: landed after … s in the air`).
- [ ] **Pulo correndo**: antes era recusado (o log mostrou a cápsula subindo e descendo 0,2–1,8 m/s correndo);
      agora em movimento pula se não estiver subindo ou caindo rápido. Pula correndo?
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-agachar` e `westplay.dll.antes-agachar`; nos
      jogos, `westplay.dll.antes-agachar`.

## 47. Música a pé abafada sempre; ao entrar na cabine ela sobe devagar — novo (app e plugin juntos, 00:05)

- **A pé, o som limpo:** o app só abafava a música depois de ter visto a cabine desenhada naquele caminhão. Se o
  app abria (ou era reinstalado) com você já a pé, a música saía limpa, como dentro da cabine. Agora o plugin
  avisa o app que você está a pé, e a música sai sempre abafada, de longe (a ponte do app com o plugin mudou: v31).
- **Entrar na cabine "estourava":** ao voltar para dentro, o som ia de abafado a cheio de uma vez. Agora ele
  sobe devagar em cerca de 3 s (era 1,5 s; dono 2026-11-07: "aumente o tempo"). Para fora continua na hora, como o jogo corta a câmera.

**App e plugin precisam ser os dois novos**: abra o jogo de novo (ele estava fechado na instalação).

- [ ] Caminhão ligado, a pé, celular, passa a música: sai abafada?
- [ ] Sobe na cabine: a música vem baixa e cresce, sem pancada?
- [ ] Trocar de câmera de fora para dentro, dirigindo: a subida suave não incomoda?
- [ ] Se piorar: `westplay.exe.antes-som-a-pe`, `ui.antes-som-a-pe`, `westplay.dll.antes-som-a-pe` (os três juntos).

## 46. Aviso no app: trocar de personagem com o jogo aberto pede reiniciar — novo (só o app, 23:52)

Na página Personagem, embaixo dos cartões:
- **jogo fechado:** "Vale na próxima vez que você descer" (como antes);
- **jogo aberto:** "O jogo está aberto: reinicie o jogo para aplicar a troca de personagem.", já antes de trocar;
- **trocou com o jogo aberto:** o mesmo aviso em vermelho, até o jogo fechar.
Nos 41 idiomas do app.

- [ ] Com o jogo aberto, o aviso aparece; ao trocar, fica vermelho?
- [ ] Fechando o jogo, volta o aviso normal?
- [ ] Se piorar: `westplay.exe.antes-aviso-personagem` e `ui.antes-aviso-personagem`.

## 45. Descendo em 3ª pessoa: a câmera de lado, não na porta — novo (só o plugin, 23:50)

Dono (duas fotos): ao terminar de descer e ir para a 3ª pessoa, a câmera ficava atrás dele, colada na porta, e só
se via o caminhão. Agora ele fica de costas para a porta (olhando para fora) e a câmera começa ao lado dele, ao
longo do caminhão, olhando para a frente do caminhão, um pouco para baixo (como na foto de exemplo).

- [ ] Desce com a 3ª lembrada: vê o personagem de lado, o caminhão à direita (ou à esquerda, no volante inglês)?
- [ ] Ele está de costas para a porta? O mouse gira a câmera normal depois?
- [ ] Se piorar: `westplay.dll.antes-camera-lado`.

## 44. A câmera lembra a última escolha (1ª ou 3ª pessoa) — novo (só o plugin, 23:40)

Descer é sempre em 1ª pessoa (a animação de descer). Quando termina, a câmera fica na última que você escolheu
com o V a pé: trocou para 3ª e voltou ao caminhão → da próxima vez desce em 1ª e, no chão, vai sozinha para a 3ª.
Ficou na 1ª → continua na 1ª. A memória dura enquanto o jogo está aberto (ao reabrir, começa na 1ª).

- [ ] Desce, aperta V (3ª), sobe; desce de novo: termina de descer e muda para a 3ª sozinha?
- [ ] Desce, V até ficar na 1ª, sobe; desce de novo: fica na 1ª?
- [ ] Se piorar: `westplay.dll.antes-lembrar-camera`.

## 43. O celular se guarda sozinho ao correr, descer e subir — novo (só o plugin, 23:30)

Não há animação de correr com o celular, nem de descer ou subir do caminhão com ele. Então, com o celular aberto,
o plugin o guarda (como se você apertasse a tecla do celular):
- ao começar a **correr** (andar com ele continua normal);
- ao **descer** do caminhão;
- ao **subir** no caminhão.
O log do plugin mostra `phone put away: running` (ou `getting out of the truck`, `climbing into the truck`).

- [ ] Celular aberto andando, aperta para correr: o celular fecha e ele corre normal?
- [ ] Celular aberto no caminhão, desce: fecha. Celular aberto a pé, sobe: fecha.
- [ ] Ele não reabre sozinho (um toque só). Se piorar: `westplay.dll.antes-guardar-celular`.

## 42. Cowboy com as pernas abertas; câmera ao correr fora do corpo — novo (plugin e mod, 23:22)

- **Pernas:** as animações vieram do tatuado, de pernas finas, e o cowboy tem calça larga e bota. Do joelho para
  baixo, uma perna encostava e passava pela outra, parado, andando e correndo. Em todas as animações dele, a coxa
  agora abre 4° e a canela e o pé ficam como estavam: a perna de baixo vai uns 2,7 cm para fora de cada lado
  (`tools/personagem/pernas_abertas.py`). Cena antes: `cowboy_animado.antes-pernas.blend`.
- **Câmera ao correr (todos):** correndo, a cabeça vai 15–22 cm à frente dos pés (inclinado). Em 1ª pessoa o corpo
  agora recua essa distância, como no agachado (`RUN_HEAD_AHEAD`).

Com o jogo fechado, o app refaz o mod em até 30 s (as animações do cowboy mudaram).

- [ ] Cowboy parado, andando e correndo: as pernas não se encostam nem passam uma pela outra?
- [ ] Não ficou de perna aberta demais?
- [ ] 1ª pessoa correndo (todos os personagens): sem ver o corpo por dentro?
- [ ] Se piorar: `westplay.dll.antes-correr-camera`.

## 41. Celular a pé: os apps abrem de novo (não ficam pretos) — novo (só o app, 23:13)

Dono: "fora do caminhão, abro o YouTube Music, o Spotify, qualquer app, e fica a tela preta". A economia da tela
fora de vista (item 24) avisa a tela do caminhão para pausar as animações, e o host repassa essa mensagem também
aos celulares. O celular na mão pausava junto, e a animação que abre um app ficava parada, com a tela preta. Agora
só a tela do caminhão obedece.

- [ ] A pé, celular aberto: YouTube Music, Spotify, mapa, configurações abrem normais?
- [ ] No caminhão continua igual; o FPS a pé continua bom.
- [ ] Se piorar: `westplay.exe.antes-celular-preto` e `ui.antes-celular-preto`.

## 40. Personagem 3, o cowboy (`wc`) — novo (app, plugin e mod, 23:05)

Rig do dono (49 ossos), na altura do homem (osso da cabeça a 1,52 m, 1,76 m com o chapéu), com todas as animações
do tatuado (andar, correr, parado, parar, pulo correndo, agachar e andar agachado, celular). O anelar acompanha o
indicador. Os arquivos estão em `%LOCALAPPDATA%\WestPlay\personagem\model\westplay\cowboy`, e o esqueleto foi
conferido nos 16 arquivos. **O mod só se refaz com o jogo fechado**: feche o ETS2, espere uns 30 s com o app
aberto e abra o jogo.

- [ ] O cartão do cowboy aparece em 3º no app, e dá para escolher?
- [ ] No jogo: em pé, andar, correr, parar, pulo correndo, agachar, andar agachado, celular parado e andando.
- [ ] O chapéu some na 1ª pessoa (o corpo sem cabeça) e não aparece cortado olhando para baixo?
- [ ] O celular na palma, sem a mão por cima da tela? Os dedos não ficam duros?
- [ ] Se algo quebrar: `westplay.exe.antes-cowboy`, `westplay.dll.antes-cowboy`, `ui.antes-cowboy`.

## 39. Som da porta ao descer e subir no caminhão — novo (só o plugin, 23:00)

Os dois sons do dono (ElevenLabs, em Downloads), cortados e no mesmo volume, em `crates/plugin/assets/door_open.raw`
(maçaneta e trinco, 0,7 s) e `door_shut.raw` (batida metálica, 1 s; o resto era silêncio):
- **Descer:** a porta abre quando ela levanta do banco; bate 0,35 s depois de pisar no chão (virando para ela).
- **Subir:** abre quando chega na porta, no chão; bate quando senta.

- [ ] Ouve abrir e bater ao descer, nessa ordem e na hora certa?
- [ ] Ouve abrir e bater ao subir?
- [ ] Volume: a batida mais alta que os passos, a maçaneta mais baixa — está bom? (`DOOR_*_VOLUME` em `steps.rs`)
- [ ] Se piorar: `westplay.dll.antes-porta`.

## 38. Celular fora do caminhão: a bolinha do Alt aparece de novo — novo (só o app, 22:50)

Dono: "abro o celular fora do caminhão, seguro o Alt e não aparece a bolinha; no caminhão funciona". Era a
economia da tela fora de vista (item 24): 1 s sem o caminhão desenhado, o app parava de mandar quadros ao jogo — e
o do celular junto, onde a bolinha é desenhada. Agora só a tela do caminhão para; o celular segue mandando.

- [ ] A pé, celular aberto, Alt: a bolinha aparece e mexe, toques funcionam?
- [ ] A tela do celular atualiza a pé (relógio, vídeo).
- [ ] O FPS a pé continua bom com o celular guardado. Se piorar: `westplay.exe.antes-celular-fora`.

## 37. Primeira pessoa agachada: a câmera não fica mais dentro do corpo — novo (só o plugin, 22:35)

Dono: "quando eu ando agachado, o braço tá passando pra frente da câmera… tô dentro do personagem". Medido no
Blender: agachada, a cabeça vai para a frente dos pés (no joelho 31 cm na mulher, 39 no tatuado; andando agachada
30/34 cm à frente e 21/23 cm para a direita), e a câmera ficava onde estavam os pés, nas costas dela. Agora, em
primeira pessoa, o corpo vai para trás (e para a esquerda andando agachada) na mesma medida, devagar com a
descida, e a cabeça fica sob a câmera.

- [ ] Andando agachada em 1ª pessoa: sem braço/ombro cortando a tela, vê os braços e as pernas olhando para baixo?
- [ ] No joelho parada, idem; ao descer e levantar o corpo não "escorrega" estranho.
- [ ] Em pé nada mudou. Se piorar: `westplay.dll.antes-camera-agachado`.

## 36. Agachar: um toque no Ctrl não buga mais — novo (só o plugin, 22:35)

Dono: "só apertar, tá bugando a animação; segurando e andando fica boa". Com um toque, ela descia até o joelho
mesmo assim antes de levantar (1,3 s), e um novo toque levantando era ignorado. Como o levantar é o descer de
trás para frente, agora ela **vira onde está**: soltou descendo → levanta dali; apertou de novo levantando → desce
dali, sempre da mesma pose.

- [ ] Um toque rápido no Ctrl parada: uma abaixadinha e volta, sem pulo de pose?
- [ ] Vários toques seguidos: sobe e desce acompanhando, sem travar?
- [ ] Segurar e andar continua bom.
- [ ] Se piorar: `westplay.dll.antes-agachar-volta`.

## 35. Agachar novo: no joelho, andar agachada, levantar — novo (app, plugin e mod, 22:22)

Com as animações de `Novas animacoes` (Mixamo): "Standing To Crouched" para descer (num joelho) e, de trás para frente,
para levantar (no lugar, as mesmas poses: o "Crouch To Stand" dava um passo para fora do corpo e demorava 2,6 s);
"Crouched Walking" para andar agachada (centrada: vinha 13 cm de lado). Feitos no Blender
(`tools/personagem/agachar_completo.py`), para a mulher e o tatuado:
- do joelho para o andar agachado, com as pernas já dando passo (0,3 s), e do andar agachado de volta ao joelho, do
  pedaço mais perto do passo (0,3 s, como no andar → parado);
- o começo e o fim do agachar entram e saem do quadro 1 do parado (sem tranco).
No plugin: segurando Ctrl ela desce e fica no joelho; andando com Ctrl, anda agachada; parando, volta ao joelho; soltando
o Ctrl parada, levanta; soltando andando, já sai andando normal. A velocidade agachada é 0,7 (era 0,9), para os pés
casarem com a animação (1,24 m/s). Os olhos (1ª pessoa) descem metade da altura no joelho e 22% andando agachada.

- [ ] Ctrl parada: desce até o joelho, sem braços abertos, e fica lá?
- [ ] Ctrl + andar: levanta um pouco e anda agachada, os pés sem escorregar?
- [ ] Parar andando agachada: volta ao joelho sem pulo de pose?
- [ ] Soltar o Ctrl parada: levanta liso e fica parada sem tranco? Andando: sai andando?
- [ ] 1ª pessoa: a câmera na altura certa no joelho e andando agachada?
- [ ] A mulher e o tatuado. No log: `shown walking crouched`, `stopping crouched`, `crouching`.
- [ ] Se piorar: `westplay.exe/.dll.antes-agachar-novo`; cenas `*_animad*.antes-agachar-novo.blend` (Downloads).

## 34. Padrão: sem pulo parado — novo (app, plugin e mod, 22:10)

Decisão do dono: "o padrão é não ter pulo parado, porque não tá ficando bom". Com a mulher ou o tatuado **parados**,
o Espaço não faz nada (log: "no jump standing still"). **Andando ou correndo**, o pulo correndo continua. A animação
do pulo parado saiu do mod (uma pessoa a menos).

- [ ] Parada: Espaço não pula (nem a física, nem a animação).
- [ ] Andando e correndo: Espaço pula como antes.
- [ ] Se quiser o pulo parado de volta: `westplay.dll.antes-sem-pulo-parado` e `westplay.exe.antes-sem-pulo-parado`.

## 33. Fim do pulo sem o "pulinho" — novo (plugin e animação, 22:00)

`TRANSICOES.md` 7.14: o pulo dava a volta no último quadro (abria os braços) e o parado entrava noutra pose. Agora o
pulo nunca passa do fim, termina na pose do parado, e o parado começa do quadro 1.

- [ ] Pulo parado: ao terminar, ela passa para o parado **sem pulinho e sem abrir os braços**?
- [ ] Pulando e saindo andando logo após pousar: continua normal?
- [ ] A mulher e o tatuado.
- [ ] Se piorar: `westplay.dll.antes-fim-pulo`; cenas `*_animad*.antes-emenda.blend` (Downloads).

## 32. Pulo em 3ª pessoa: a câmera não sobe junto — novo (só o plugin, 21:45)

A gravação mostrou que a câmera de 3ª pessoa subia junto com o pulo, então ela não subia na tela (`TRANSICOES.md`
7.13). Agora a câmera fica na altura do chão enquanto ela está no ar.

- [ ] 3ª pessoa, pulo parado: ela **sobe na tela** (a câmera parada) e desce?
- [ ] O encolher das pernas agora parece no ar, não um agachar no chão?
- [ ] Pulo correndo também (a câmera acompanha para a frente, mas não para cima).
- [ ] 1ª pessoa: a vista sobe e desce com o pulo, como antes.
- [ ] No fim do pulo, ainda aparece ela "dobrada" (agachada e de pé ao mesmo tempo)?
- [ ] Se piorar: `westplay.dll.antes-camera-pulo` (no `%LOCALAPPDATA%\WestPlay` e nos jogos).

## 31. Pulo parado: o "Jump" (1) cortado — novo (plugin e animação, 21:30)

O dono achou o "Jumping" cortado "mais ou menos" e baixou o `Novas animacoes\Jump (1).fbx` (um pulo de verdade:
dobra os joelhos e encolhe as pernas no ar). Cortado nos quadros 15–64: começa já no fundo do agachamento, sai do
chão 0,3 s depois do Espaço, fica 0,63 s no ar (a física fica 0,6) e termina de pé. Mulher e tatuado (reaproveitado
da mulher). Cenas de antes: `*_animad*.antes-jump1.blend` (Downloads).

- [ ] Espaço parado: ela agacha rapidinho e sai do chão junto com o corpo?
- [ ] No ar encolhe as pernas e pousa junto com o corpo (sem pulo de novo depois)?
- [ ] O começo (de pé → agachada) não dá um tranco feio?
- [ ] A mulher e o tatuado.

## 30. Pulo parado cortado (começa já no impulso) — novo (plugin e animação, 21:15)

Dono: no pulo parado ela subia, descia os braços e "pulava de novo" levantando as mãos. O "Jumping" foi cortado no
Blender nos quadros 24–55 (`tools/personagem/cortar_animacao.py`): sem a preparação lenta, ele começa já no
agachadinho do impulso, como o pulo correndo. A física sai 0,13 s depois do Espaço (quadro 5), pousa no quadro 19 e
ela termina de pé, de braços baixos (quadro 32). Não há segundo levantar de braços na animação. Cenas de antes:
`*_animad*.antes-corte-pulo.blend` (Downloads).

- [ ] Espaço parado: ela pula na hora, os braços sobem uma vez só, junto com o corpo?
- [ ] Ao pousar, baixa os braços e fica parada (sem levantar de novo)?
- [ ] Se ainda levantar os braços depois de pousar: cortar também o fim (me avisar).
- [ ] A mulher e o tatuado.

## 29. Pulo parado e parar: animação mandada em pedaços — novo (só o plugin, 21:05)

Hipótese (`TRANSICOES.md` 7.12): o jogo não aceita de uma vez um avanço longo da animação. O pulo parado começava do
quadro 1 em vez do 15 (o corpo subia duro, e o pulo vinha depois), e a transição para o parado não chegava ao
pedaço certo (o passinho). Agora todo avanço longo vai em pedaços de 1/30 s.

- [ ] Pulo parado: ao apertar Espaço, ela já agacha e sobe **junto** com o corpo (sem subir dura antes)?
- [ ] Parar andando: sem o passinho para frente?
- [ ] Pulo correndo, agachar e celular continuam como antes?
- [ ] Se piorar: `westplay.dll.antes-passos` (no `%LOCALAPPDATA%\WestPlay` e nos jogos).

## 28. Onde o jogo guarda o tempo da animação (só log) — plugin novo, 19:58

Resultado do 27 (dono): "quando ela para ainda dá um passinho para frente"; o pulo parado ainda fora de sincronia.
Hipótese em `TRANSICOES.md` 7.9: o jogo começa cada animação num ponto próprio, e o plugin acha que é no 0.

- [ ] Abrir o jogo de novo (o plugin novo só entra assim), descer do caminhão e ficar 2 s parado. Só isso.
- [ ] Me avisar: as linhas `probe:` do `plugin.log` dizem onde está o tempo e de onde começou.

## 27. C' de `TRANSICOES.md` 7.8: parar mais cedo — novo (só o plugin, 19:50)

A medição mostrou que a cápsula para em 1 quadro, mas o personagem só contava como parado 0,15 s depois (andando
no lugar nesse tempo). Agora conta depois de **0,05 s** (`STILL_AFTER`): a transição para o parado começa quase
junto com o corpo.

- [ ] Andando, solte o W: ela para sem dar passo no lugar (sem escorregar)?
- [ ] **Não voltou a piscar** ao parar (andar ↔ parado trocando várias vezes)? Se piscar, é este número.
- [ ] Andar em curvas e soltar/apertar o W rápido: nada pisca nem trava.
- [ ] No log (`measure: let go`): "not moving at" perto de 0,05 s (antes 0,15–0,17).
- [ ] Se piorar: `westplay.dll.antes-parada-rapida` (no `%LOCALAPPDATA%\WestPlay` e nos jogos).

## 26. Medição B de `TRANSICOES.md` 7.7 (só log, nada muda no jogo) — plugin novo, 19:39 — FEITO (só a mulher; resultado em `TRANSICOES.md` 7.8)

O plugin passou a escrever no `Documents\WestPlay\logs\plugin.log` (procure por `measure:`):
- a cada vez que um W/S/A/D desce ou sobe: a velocidade da cápsula **quadro a quadro** por 1,5 s, com o resumo
  (`set off`: quando passou de 0,3 m/s e chegou a 90% da velocidade; `let go`: quando caiu abaixo de 0,3 m/s e
  quando o personagem deixou de contar como andando);
- a cada 10 s, a pé: quanto tempo leva posicionar as pessoas por quadro (média e pior) e quantas são.

Roteiro (uns 3 minutos, a pé, com a mulher e depois com o tatuado):
- [ ] Parado 20 s (o custo com o personagem parado).
- [ ] Andar reto 3 s e soltar o W, umas 5 vezes; depois correr (Shift+W) e soltar, umas 5 vezes.
- [ ] Andar e virar (W+A, W+D), e andar para trás (S) um pouco.
- [ ] Me avisar: eu leio o log e passo os números para o `TRANSICOES.md`.

## 25. Andar → parado sem tranco (etapa 2 de `TRANSICOES.md`) — 2ª versão (app, plugin e mod novos, 19:25)

1ª versão (19:15), testada pelo dono: **pior**, "faz duas, três animações já parado, como se escorregasse". O andar
continuava dando passos durante a transição (até 0,26 s esperando o próximo quarto + 0,5 s da transição), com o
corpo já parado. 2ª versão: uma animação só (`settle`, 8 pedaços de 0,3 s). Cada pedaço parte da pose do andar
**congelada** num oitavo do passo e fecha os pés até o primeiro quadro do parado, **sem nenhum passo a mais**.
Soltando o W andando, toca na hora o pedaço mais perto de onde o andar estava (a diferença é de menos de 0,06 s de
movimento) e depois vem o parado do quadro 1. Correndo, continua o "parar" de antes.

- [ ] Andando, solte o W em vários momentos: ela para em ~0,3 s, sem passos no lugar e sem escorregar?
- [ ] Os braços e as pernas não pulam de pose, nem no começo nem quando o parado começa?
- [ ] Solte o W e aperte de novo logo: volta a andar sem travar nem piscar?
- [ ] Os dois (mulher e tatuado), em 3ª e 1ª pessoa; com o celular aberto continua como antes; descer e subir do
      caminhão.
- [ ] No log do plugin: `shown settling` e depois `shown standing`.
- [ ] Se piorar: `westplay.exe.antes-transicao` e `westplay.dll.antes-transicao` (no `%LOCALAPPDATA%\WestPlay` e nos
      jogos) voltam para antes das duas versões.

## 24. Jogo pesado na cabine — novo (app novo e config dos jogos, 18:59)

Medido às 18:45 (ETS2): com a tela do WestPlay à vista (na cabine) o jogo caía de 60 para 38–42 fps, a placa
(RTX 2050) a 100%, no limite de 45 W e a 87 °C (baixando o clock sozinha). Mudado:
- **No jogo** (ETS2 e ATS, `config.cfg`): escala 125% → **100%**; espelhos até **60 m** (eram 120) e sem a luz
  completa (`r_deferred_mirrors 0`). Cópia: `config.cfg.antes-desempenho` na pasta de cada jogo.
- **No WestPlay**: fora da cabine (3ª pessoa, a pé, menu do jogo), depois de 1 s, a tela não é mais enviada ao jogo e
  o mapa e as animações dela ficam parados; voltando à cabine, voltam na hora. Na cabine nada muda (o jogo desenha a
  tela o tempo todo ali).

- [ ] Na cabine, 1ª pessoa, dirigindo: mais fps que antes (antes ~40)? Os espelhos ainda servem (60 m)?
- [ ] O jogo ficou feio demais com a escala 100%? (dá pra voltar só ela, `r_scale_x/y`)
- [ ] Saindo da cabine e voltando: a tela volta na hora, com o mapa no lugar certo (não uma imagem velha)?
- [ ] No log do app (`Documents\WestPlay\logs\app.log`): fora da cabine, "screen: out of view, held" e "0 frames a
      second sent"; de volta, "screen: in view".
- [ ] Se piorar: o app em `%LOCALAPPDATA%\WestPlay` (`westplay.exe.antes-desempenho`, `ui.antes-desempenho`); os
      gráficos: `config.cfg.antes-desempenho` → `config.cfg` (com o jogo fechado).

## 23. O tatuado no lugar do homem — novo (app, plugin e mod novos, 18:35)

O primeiro homem saiu; o **tatuado** (`wt`, personagem 4) é o "Homem 1" no app, com o retrato novo. O rig é o do
Mixamo que o dono fez (41 ossos), na altura do padrão (osso da cabeça a 1,52 m). As animações são reaproveitadas:
andar, correr e parado do primeiro homem (os dedos levemente fechados), e da mulher o parar, os dois pulos, o
agachar e as duas do celular (o celular na mão dele, o corpo sem cabeça na 1ª pessoa). Tudo passou no conferidor.

- [ ] No app, Personagem: o cartão "Homem 1" mostra o tatuado? Escolhido, é ele quem desce do caminhão?
- [ ] Parado, andando, correndo: os pés no chão, sem escorregar, os passos no tempo do som?
- [ ] Parar, pular parado, pular andando/correndo, agachar (Ctrl): como os da mulher, sem atravessar o chão?
- [ ] Celular (Alt+4) parado e andando: na mão dele, sem a mão atravessar a tela?
- [ ] 1ª pessoa: o corpo sem a cabeça (o cabelo e os óculos não aparecem na frente da câmera)?
- [ ] Alguma animação parece "de mulher" demais nele (o parar, o agachar, o celular)? Dá pra trocar por uma do
      Mixamo de homem.
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-tatuado`, `westplay.dll.antes-tatuado` e
      `ui.antes-tatuado`; nos jogos, `westplay.dll.antes-tatuado` (com eles, o primeiro homem volta: os arquivos
      dele continuam na pasta `personagem`).

## 22. Pulo parado novo: "Jumping" — novo (plugin e mod novos, 18:00)

O pulo parado da mulher agora é o "Jumping" (de `Novos pulos parado`): um pulo reto, de braços pra cima, sem o
agachamento fundo depois. Pés fora do chão nos quadros 29 a 41; o Espaço começa no quadro 15 (ela sai do chão
0,43 s depois); com o pé no chão em cada quadro, como o outro.

- [ ] Parado, Espaço: ela dobra os joelhos, sobe junto com o cilindro, desce e pousa junto? Sem tranco no começo?
- [ ] Demora demais entre o Espaço e sair do chão? (dá pra começar mais adiante: o `from` no perfil)
- [ ] Andando ou correndo, o pulo continua o de correr (não mudou).
- [ ] Se piorar: `westplay.dll.antes-jumping` (no `%LOCALAPPDATA%\WestPlay` e nos jogos) e a cena
      `mulher_animada.antes-jumping.blend` (pasta da mulher em Downloads); as animações de antes em
      `%TEMP%\…\scratchpad\antes-jumping\anim`.

## 21. Padrão do personagem: pulos com o pé no chão, perfil único — novo (app, plugin e mod novos, 17:50)

Os dois pulos da mulher foram refeitos com o pé mais baixo no chão em cada quadro (`tools/personagem/pe_no_chao.py`):
no alto do pulo parado ela encolhe as pernas sem subir os pés a 0,85 m ("escalando"), e no pulo correndo a perna
do pouso não entra mais 13 cm no chão. O plugin agora tem um **perfil** por pessoa (os mesmos tempos de antes), e o
app refaz o mod sozinho quando os arquivos do personagem mudam (antes precisava reiniciar o app).
Guia para incluir personagens: `docs/personagem/INCLUIR_PERSONAGEM.md`.

- [ ] Mulher, pulo **parado**: agacha, sai do chão junto com o cilindro, encolhe as pernas lá em cima e pousa
      junto (log: "landed after" ~0,6 s)? Os pés não atravessam o chão ao sair?
- [ ] Pulo **andando** e **correndo**: pousa sem a perna entrar no chão e continua andando/correndo?
- [ ] O resto igual a antes (andar, correr, parar, agachar, celular, homem): nada mudou de tempo.
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-padrao`, `westplay.dll.antes-padrao` e
      `ui.antes-padrao`; nos jogos, `westplay.dll.antes-padrao`. As animações de antes: cena
      `mulher_animada.antes-pe-no-chao.blend` (pasta da mulher em Downloads).

## 20. Celular encaixado na mão — novo (só os modelos com celular, 00:29; o mod se refaz com o jogo fechado)

Na foto do dono (00:2x) o celular atravessava a mão: metade dentro da palma (a pele aparecia por cima da tela,
em 1ª pessoa) e a ponta pra trás, no pulso. Agora ele fica **apoiado na palma**, a tela virada pros olhos dela,
os dedos por baixo (andando, as pontas abraçam a borda). Conferido em Blender nos quadros 25/150/300/450 da
"Texting" e 10/50/80 da "Texting And Walking".

- [ ] 1ª pessoa, celular aberto, olhando pra baixo: a tela inteira à vista, sem a mão por cima dela?
- [ ] 3ª pessoa: o celular na mão, na frente do peito, sem atravessar os dedos?
- [ ] Se piorar: os modelos de antes em `%TEMP%\…\scratchpad\antes-mao` (wm_phone/wm_phbody .pim/.pit) e a
      cena `mulher_celular.antes-mao.blend`, na pasta da mulher em Downloads.

## 19. Pulo: pouso falso no topo corrigido — novo (só o plugin, 00:23; reiniciar o jogo)

No teste das 00:16 o log dizia "pousou" 0,32 s depois de sair do chão: era o **topo** do pulo (a velocidade passa
por zero), e a animação ia direto pro pouso (e, andando/correndo, era cortada) com o cilindro ainda no alto. Agora
o pouso só conta depois de ver o cilindro descendo.

- [ ] Parado: ela agacha, sai do chão junto com o cilindro e pousa junto (no log, "landed after" ~0,6 s)?
- [ ] Andando e correndo: pula, pousa junto e continua andando/correndo, como estava antes?
- [ ] Se piorar: `westplay.dll.antes-pouso` (em `%LOCALAPPDATA%\WestPlay` e nos dois jogos).

## 18. Parar suave e celular na mão (etapas 2 e 3) — novo (app, plugin e mod novos, 00:16)

**Parar** (mulher): ao parar de andar ou correr, toca só o fim da "Female Stop Walking" (do último pé no chão até
ficar parada, ~0,5 s): sem os três passos no lugar, que escorregariam. Andar de novo corta.
**Celular** (mulher): o modelo de celular baixado, diminuído pra 14,7 cm e com a tela trocada (os ícones do iOS
saíram: uma tela escura azulada e o corpo grafite), preso ao osso da mão direita numa versão da mulher "com
celular" (e a sem cabeça, pra 1ª pessoa), usada só nas animações de celular.

- [ ] **Parar**: andando, solte o W: ela acomoda e fica parada, sem piscar e sem passos no lugar?
- [ ] **Celular parada**: Alt+4 com ela parada. Ela **leva o celular ao peito** (~1 s) e fica digitando? O
      celular está **na mão**, do tamanho certo, a tela virada pra ela? Depois de ~18 s ela continua digitando
      (não pega o celular de novo)?
- [ ] **Celular andando**: com o celular aberto, ande: ela anda digitando ("Texting And Walking")? Correndo, volta
      pra corrida normal.
- [ ] Fechando o celular (Alt+4), ela volta ao normal (o celular some da mão).
- [ ] 1ª pessoa: com o celular aberto, olhando pra baixo, aparecem as mãos com o celular?
- [ ] Se piorar: em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-celular` e `westplay.dll.antes-celular`; nos
      jogos, `westplay.dll.antes-celular`.

## O que olhar se der errado

- Log do plugin: `Documents\WestPlay\logs\plugin.log` (linhas que começam com `character:`). Ele diz a altura dos
  pés ("feet … m under the eyes"), o pulo ("jump"), o personagem criado e por que não desceu.
- Mande print ou o trecho do log.

## Voltar atrás

- Só o plugin: na pasta `plugins` de cada jogo, `westplay.dll.antes-aviso-parede` (antes dos avisos e da câmera nas paredes), `westplay.dll.antes-subida` (antes da subida em 1ª pessoa), `westplay.dll.antes-tecla-pessoa` (antes do pulo, da tecla e do
  personagem), `westplay.dll.passo-sintetico` (antes do som gravado), `westplay.dll.terceira-7` (antes de todo o
  polimento de hoje). Renomeie para `westplay.dll` com o jogo fechado.
- **Lanterna** (app e plugin): `%LOCALAPPDATA%\WestPlay\backup-antes-lanterna` + a DLL
  `westplay.dll.antes-lanterna` nos jogos (a ponte é a mesma v30).
- **Barra de teclas** (app e plugin juntos, ponte v30): `%LOCALAPPDATA%\WestPlay\backup-antes-barra` (exe, dll,
  ui) + a DLL `westplay.dll.antes-barra` nos jogos.
- **Personagem próprio** (app e plugin juntos): em `%LOCALAPPDATA%\WestPlay`, `westplay.exe.antes-personagem`,
  `westplay.dll.antes-personagem` e a pasta `ui.antes-personagem`; nos jogos, `westplay.dll.antes-personagem`.
  Pra não usar o personagem, basta escolher outro em "Quem anda" (os arquivos dele ficam no mod sem atrapalhar).
- App e plugin juntos (a ponte mudou de versão: os dois precisam ser da mesma versão):
  `%LOCALAPPDATA%\WestPlay\backup-antes-tecla-pessoa` (westplay.exe, westplay.dll, ui) + a DLL
  `antes-tecla-pessoa` nos jogos.

## Ainda não feito (precisa do jogo aberto para investigar)

- Correr com animação de corrida de verdade (o `run_01.pma` do ETS2 não tem definição que o use).
- Personagem próprio, com editor.
