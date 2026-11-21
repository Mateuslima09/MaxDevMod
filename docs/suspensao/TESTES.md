# App Suspensão, botão "Desligar só o motor" e tela com a ignição: relatório de teste (para o dono)

Instalado em 2026-11-18 às 21:41, **ainda não testado no jogo**. App e plugin mudaram juntos (a ponte agora é a
**versão 44**): abra o jogo depois do app. Nada foi para o GitHub.

## 1. App Suspensão (celular, segunda página de apps)

- [ ] O ícone **Suspensão** aparece na segunda página, depois do TikTok.
- [ ] Dentro do caminhão, a cena mostra o caminhão (e a carreta, se engatada) e os três medidores marcam algo.
      Se aparecer "Sem leitura do jogo ainda", o jogo está com o plugin antigo: reinicie o jogo.
- [ ] **Subir** e **Baixar** (segurando) movem a suspensão no jogo, e a cena e os medidores acompanham. As bolsas de ar
      ficam verdes subindo e laranja descendo.
- [ ] As abas **Caminhão / Dianteira / Traseira / Carreta** mudam o que os botões movem. Carreta só com carreta engatada.
- [ ] **Máxima** e **Mínima** seguram o botão por 5 s (uma barra corre embaixo). **Nivelar** volta ao normal e zera os
      medidores depois de uns 2,5 s.
- [ ] Se a cena e os medidores **não se mexerem** ao subir e baixar (a suspensão sobe no jogo, mas o número fica parado),
      anote: o jogo talvez não conte a altura da suspensão a ar na leitura das molas, e a cena terá de seguir os botões.

## 2. Voltar ao normal sozinha

- [ ] No app, **Desligado**. Com o jogo aberto aparece "Salvo: vale depois que o jogo for fechado e aberto de novo".
- [ ] Feche o jogo: o app grava `uset g_suspension_auto_reset "0"` no `config.cfg` (Documentos\Euro Truck Simulator 2).
- [ ] Abra o jogo, suba a suspensão e saia andando: ela **fica** na altura escolhida.

## 3. Botão "Desligar só o motor" (Comandos › Ignição)

- [ ] Com o motor ligado: o motor para e a energia (painel e tela) continua.
- [ ] Se o motor não parar ou a energia cair junto: avise (talvez precise de mais tempo entre os dois comandos).

## 4. Tela e música só com a ignição

- [ ] Cortar a ignição (E): a tela apaga e a **música para**.
- [ ] Ligar de novo: o painel mostra a animação de ligar e volta.
- [ ] A opção "Manter a tela ligada com o caminhão desligado" não aparece mais nos Ajustes da tela.

## 5. Correções de 18/11 22:07

- [ ] App Suspensão: o caminhão novo aparece grande na cena (sem carreta) e com a carreta quando engatada.
- [ ] Os quatro botões Caminhão / Dianteira / Traseira / Carreta aparecem como botões, o escolhido em azul.
- [ ] **Desligar só o motor**: o painel **não** reinicia (a energia cai menos de 1,5 s e o painel ignora).
- [ ] Cortar a ignição de verdade (E): o painel apaga depois de 1,5 s.

## 6. Frente mais baixa embutida (18/11 22:45)

O mod de teste saiu da pasta de mods: a frente baixa agora vem do próprio MaxDevMod (`westplay_suspensao.scs`, em
DocumentosWestPlayets2, carregado pelo plugin sem o Gerenciador de Mods).

- [ ] No app Suspensão, **Frente mais baixa que o jogo** › **Máxima** (com o jogo fechado ou aberto).
- [ ] No app.log: `setup: the low suspension archive made (Lowest: N chassis)` (N na casa das centenas).
- [ ] Abra o jogo: no plugin.log, `mod mount: .../westplay_suspensao.scs: device ...`.
- [ ] Baixar tudo: a frente desce como no teste do mod. **Baixa** desce menos; **Normal** volta ao do jogo.
- [ ] Outro caminhão do jogo (Scania, Volvo...): a frente também desce mais.
- [ ] Se algum caminhão ficar estranho (modelo trocado, pulando), anote qual.

## 7. Amortecedor (18/11 22:49)

- [ ] App Suspensão › **Amortecedor**: Macio / Normal / Firme / Muito firme (fatores 0,6 / 1 / 1,5 / 2,2 sobre os do
      jogo: dianteira 1,0, traseira 0,8, carreta 0,5). Vale na próxima vez que o jogo abrir.
- [ ] No app.log: `low suspension archive made (..., dampers Firm: ...)`.
- [ ] Firme: o caminhão balança menos nas curvas e sente mais os buracos. Macio: o contrário.

## 8. Suspensão firme de verdade, ícone novo e Comandos (19/11)

Teste do dono (19/11): "mesmo no super rígido o caminhão ficava muito mole; acelerando, a frente subia".

O que se viu: o arquivo estava certo (amortecedores 2,2×) e o jogo carregou ele. Mas:
- o jogo **não tem** um valor de dureza da mola: ele calcula cada mola a partir do curso do eixo (`residual_travel`).
  A **Frente mais baixa › Máxima** triplica o curso da frente (0,1 → 0,3 m), o que muito provavelmente deixa a mola
  da frente umas 3× mais mole. É isso que faz a frente subir ao acelerar;
- o amortecedor só freia o balanço, e o que se sente na cabine é mais a **suspensão da própria cabine**, que o ajuste
  não mexia.

Corrigido (só o app mudou; o arquivo é refeito sozinho e vale **na próxima vez que o jogo abrir**): Firme / Muito firme
agora também deixam mais firmes a barra estabilizadora da frente, o amortecedor da cabine e a barra da cabine, e
diminuem o quanto a cabine pode inclinar para frente/trás e para os lados (Muito firme: 2,25° → cerca de 1°).
Macio faz o contrário.

- [ ] Feche o jogo, abra de novo. No app.log: `low suspension archive made`.
- [ ] Com **Muito firme** e a frente em **Normal**: acelerando, a cabine e a frente quase não sobem; curvas balançam menos.
- [ ] Com **Muito firme** e a frente em **Máxima**: compare. Se a frente ainda subir muito, é a mola mais mole da frente
      baixa (o preço de descer mais); **Baixa** é um meio-termo.
- [ ] O ícone da Suspensão agora é um amortecedor com a **mola vermelha** (fundo grafite).
- [ ] Comandos não tem mais o grupo **Suspensão a ar** (está tudo no app Suspensão).
- [ ] No **celular web** (página do celular do jogo, pelo QR "só o celular"): o app Suspensão aparece na segunda
      página e os botões mexem a suspensão no jogo. Se não aparecer, recarregue a página.

## 9. Piscadas pretas depois de "Desligar só o motor" (19/11, só o plugin: **reinicie o jogo**)

Teste do dono (19/11): a tela inteira dava piscadas pretas rápidas depois de **Desligar só o motor**, e continuava
piscando depois de descer do caminhão.

Causa provável: a imagem em alta resolução da tela do painel é posta no lugar da do jogo pelo **endereço** das vistas
dessa tela, guardado sem segurar a vista. Desligando e ligando, o jogo refaz as texturas da tela, e o endereço de uma
vista solta passa a ser de outra, de um efeito de tela inteira: a imagem do painel ia parar ali e a tela piscava. No
log, a primeira troca (`screen hires: 3D pass SRV substituted`) saiu no mesmo instante do `ignitionoff`.

Corrigido: antes de trocar, o plugin confere se a vista ainda é da textura da tela. DLL antiga guardada como
`westplay.dll.antes-piscada-motor` (pastas plugins dos dois jogos e MaxDevMod).

- [ ] Reinicie o jogo. **Desligar só o motor**: sem piscadas, na cabine e depois de descer?
- [ ] A tela do painel continua nítida (alta resolução) antes e depois de desligar só o motor?
- [ ] No plugin.log, se aparecer `screen hires: a GPS view's address now another texture's: not substituted`, era isso
      mesmo (o plugin pegou e não trocou).

## 10. Bolinha do mouse, barra de início e personagens (19/11, só o app: não precisa reiniciar o jogo)

- [ ] Celular › Ajustes › **Bolinha do mouse**: Pequena / Normal / Grande / Muito grande (0,7× / 1× / 1,35× / 1,7×).
      Muda na hora a bolinha da tela do caminhão e a do celular no jogo.
- [ ] A **barra de baixo** do celular (voltar ao início) está maior (150×7 px, antes 124×5) e mais fácil de acertar.
- [ ] Celular › **Personagem**: aparecem **4** (Homem 1, Mulher 1, Homem 2, Homem 3: o de chapéu). Antes só 3.
- [ ] O **gordinho** não aparece no celular nem no app do computador. Se ele estava escolhido, desce o Homem 1.
      Os arquivos dele continuam no mod: para voltar, é só tirar o 4 de `PEOPLE_OFF` (crates/host/src/settings.rs) e o
      `off: true` das duas listas.

## 11. Aviso de caminhão desligado no app Suspensão (19/11, só as telas: nada a reiniciar)

- [ ] Com o **motor desligado**, o app Suspensão mostra um aviso vermelho: "O caminhão está desligado: a suspensão a ar
      só sobe e desce com o motor ligado". O selo do topo diz **Desligado** em vermelho e os botões ficam apagados.
- [ ] Apertando Subir, Baixar, Máxima, Nivelar ou Mínima com o motor desligado, o aviso **treme**.
- [ ] Ligando o motor, o aviso some na hora e o selo volta para **Pronta**.
- [ ] Igual no celular web (recarregue a página uma vez).

## 12. Suspensão automática (19/11, só o app: nada a reiniciar)

Pedido do dono: "a suspensão se adaptando sozinha, igual na vida real". Primeiro passo, por velocidade, como o ECAS
dos caminhões de verdade. O app do computador aperta os mesmos comandos do app Suspensão: **um tempo (5 s) acima de
80 km/h, baixa um pouco** (segura Baixar frente e traseira por 1,5 s, cerca de um terço do curso); **5 s abaixo de
60 km/h, volta ao normal** (o "nivelar" do jogo). Só com o motor ligado e o jogo sem pausa. Mexer na suspensão pelo
app pausa a automática por 1 minuto. Começa **desligada**.

- [ ] App Suspensão › **Suspensão automática** › Ligado. Melhor com **Voltar ao normal sozinha** desligado (senão o
      jogo pode desfazer).
- [ ] Na estrada, passe de 80 km/h: uns 5 s depois a suspensão **desce um pouco** (os medidores e o desenho também,
      se a leitura das molas acompanhar a altura). No app.log: `suspension by itself: lowered at 8x km/h`.
- [ ] O jogo deixa a suspensão mexer **com o caminhão andando**? Se não deixar, anote: aí a automática teria de agir
      só parada ou bem devagar.
- [ ] Reduza para menos de 60 km/h: uns 5 s depois ela **volta ao normal** (`back to normal at ...`).
- [ ] Mexa você no app (Subir/Baixar) andando rápido: a automática não briga com você por 1 minuto.
- [ ] Desligada: nada acontece sozinho.
- [ ] Para as próximas fases (estrada ruim e carga): os medidores do app mexem quando a suspensão sobe e desce? (a
      pergunta da seção 1).

## 13. Automática ajustada, estrada ruim, carga e "Sem leitura" (19/11; app e plugin novos: abra o jogo depois do app)

Teste do dono (19/11): "funcionou, mas abaixou demais: depois dos 80 km/h ficou pulando, batendo no chão e dando
dano". E: "a mensagem Sem leitura do jogo aparece e some, e quando aparece para de ler".

- **80 km/h**: agora baixa só **0,4 s** de comando (antes 1,5 s), cerca de um terço do que baixava.
- **"Sem leitura"**: o app tomava qualquer leitura negativa por "sem leitura", mas a mola esticada (suspensão subida)
  lê negativo de verdade. Agora só o -1 do plugin é "sem leitura".
- **Estrada ruim** (novo): andando acima de 15 km/h, se as molas pularem muito por 2 s (mais de 1,2 cm em média em
  volta do meio delas), a suspensão **sobe um pouco** (0,6 s). Lisa de novo por 10 s, volta ao normal.
- **Carga** (novo): parado, engatando uma carreta, se a traseira afundar mais de 3 cm do que estava antes, a
  automática **sobe a traseira** em toques de 0,3 s até voltar (no máximo 8 toques).
- A cada 5 s andando, o app.log diz `suspension by itself: XX km/h, road N cm, Normal/Low/High`: com isso eu acerto
  os números da estrada ruim pelo seu log.

- [ ] Acima de 80 km/h: baixa só um pouco, **sem bater no chão nem dar dano**?
- [ ] "Sem leitura do jogo" não pisca mais com o jogo rodando? Os medidores mexem subindo e descendo?
- [ ] Estrada de terra ou esburacada: sobe sozinha (`raised on a rough road`)? Em asfalto bom, **não** sobe à toa?
- [ ] Engatar carreta pesada parado: a traseira sobe sozinha (`the rear X cm down under the load: raised`, depois
      `the load levelled`)?

## 14. Terra que fica em cima, freio longo (19/11; app e plugin novos, ponte 45: abra o jogo depois do app)

Do log da estrada de terra (19/11): a variação na terra deu **1,2 a 2,7 cm** e no asfalto **0,5 a 0,9 cm**. Ela subia
e **0,6 s depois voltava** (vai-e-vem): eu tinha deixado "abaixo de 60 km/h" também mandar voltar. Corrigido do jeito
do dono:

- **Terra:** variação acima de 1,2 cm por 2 s → sobe (0,8 s de comando) e **fica em cima**. Só volta depois de
  **20 s seguidos** com a estrada estável (abaixo de 0,9 cm). Enquanto estiver ruim, **nunca** baixa. Parado, nada
  conta (nem para subir, nem para descer). Depois de subir, ela ignora a medida por 2,5 s (a própria subida mexe a mola).
- **Freio longo** (novo): pedal acima de 30% e acima de 20 km/h **por 2 s** → sobe um pouquinho **de onde estiver**
  (0,4 s de comando) e segura enquanto frear; soltou o freio por 1 s → desce **o mesmo tanto** (volta para onde estava,
  não para o normal). A leitura do pedal é nova (`truck.effective.brake` do SDK), por isso a ponte foi para 45.

- [ ] Terra: sobe e **fica**? No app.log, um `raised on a rough road` e nada de `back to normal` enquanto for terra.
- [ ] Voltando para o asfalto: uns 20 s depois, `back to normal (smooth again)`.
- [ ] Freando forte numa descida ou reta, baixo: `raised a little, braking long`; soltando: `the brake let go: down
      as much again`. Para de bater no chão?

## 15. Tela maior borrada no Mercedes Actros 2014 (FG) e em todo caminhão (20/11, só o plugin: **reinicie o jogo**)

Causa: a tela maior do mod (`FG Mercedes Benz Actros 2014.scs`, caminhão `fg_mb2014`) usa a tela padrão do jogo
(`ui/gps.sii`, layout `generic`) desenhada num alvo de só **256x256**, esticado no tablet grande. O mod é protegido, o
WestPlay não consegue ler nem aumentar o `gps_drawable_size` dele. A troca em alta resolução ao vivo (Ajustes ›
Resolução) devia resolver, mas **nunca trocava**: ela desistia sempre que o alvo atual tinha "cara de tela" (RGBA8, até
2048), e o passe 3D da cabine do ETS2 1.62 desenha num alvo 1920x1080 assim. Nenhum log tinha
`screen hires: 3D pass SRV substituted`. Agora ela só desiste quando o próprio alvo da tela está sendo desenhado.
Vale para todos os caminhões (antes nenhum tinha a troca funcionando). DLL antiga guardada como
`westplay.dll.antes-tela-maior` (pastas plugins dos dois jogos e MaxDevMod).

- [ ] Reinicie o jogo. Mercedes Actros 2014 com a **tela maior**: Ajustes do WestPlay em Alta / Ultra: o texto fica
      nítido? Trocando Padrão › Alta › Ultra, muda na hora?
- [ ] No plugin.log aparece `screen hires: 3D pass SRV substituted` logo depois de `screen hires: generic 256x256 -> ...`?
- [x] 1º teste (11:39): a troca funcionou (`3D pass SRV substituted`), mas a imagem e os toques ficaram de cabeça para
      baixo. Desvirado às 11:41 (DLL de antes guardada como `westplay.dll.de-cabeca-pra-baixo`).
- [x] 2º teste (11:45): em pé, sem serrilhado, mas mole/cinzento (o jogo misturava a versão de 320 px de largura).
      Agora a imagem para na versão de 640 px (DLL de antes: `westplay.dll.antes-nitidez`).
- [ ] Reinicie o jogo: em **Alta** e **Ultra**, o texto ficou nítido? Voltou algum serrilhado (letras tremendo ao
      mexer a cabeça)? Compare as duas e diga qual ficou melhor.
- [ ] Em Padrão do jogo (sem alta resolução) volta a ficar como antes (borrado nesse caminhão, normal nos outros).
- [ ] Volvo FH, DAF, Scania do jogo: a tela continua certa (lugar, tamanho, toque/mouse) e sem piscadas pretas.
- [ ] Desligar só o motor e descer do caminhão: nenhuma piscada preta na tela inteira (item 9 continua valendo).

## 15. Terra com o caminhão baixo, e voltar para a altura que estava (21/11; só o app)

Teste do dono (21/11): "com o caminhão mais baixo que o normal ela não identifica a terra direito; e quando volta,
volta para a suspensão padrão do jogo, não para a muito baixa que eu tinha".

- **Medida:** cada eixo (frente e traseira) é medido sozinho e vale o que pula mais (antes era a média dos dois, que
  escondia). Além da variação média (acima de 1,2 cm = ruim), conta também o **maior sobe-e-desce em 2 s** (acima de
  6 cm = ruim; lisa abaixo de 3,5 cm): baixo, a mola encosta no batente e só varia para um lado, mas o pico aparece.
  No app.log: `road X cm (widest Y cm)` a cada 5 s andando, para acertar os números.
- **Volta:** nada mais de "nivelar" do jogo. Cada movimento é desfeito pelo contrário, do mesmo tamanho: subiu 0,8 s
  na terra, desce 0,8 s; baixou 0,4 s na estrada, sobe 0,4 s. Volta para **a altura que você escolheu**.
  (Se o jogo subir e descer em velocidades diferentes, pode sobrar um pouquinho de diferença: me diga.)

- [ ] Baixe bem o caminhão, entre na terra: ela sobe (`raised on a rough road`)?
- [ ] Saindo da terra (20 s lisos): `down as much again`, e o caminhão volta para **a altura baixa** que estava?
- [ ] Na estrada a 80+ e depois abaixo de 60: volta para a sua altura, não para a do jogo?
