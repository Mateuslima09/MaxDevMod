# Câmera de ré pela imagem do retrovisor: etapa 1 (achar a imagem)

Ideia do dono (2026-11-13): o retrovisor do jogo já é uma "câmera" que olha para trás. Usar essa imagem, que é a
real do jogo (docas, cercas, carretas paradas, trânsito), no lugar do desenho feito a partir do mapa, que fica vazio
em mapas protegidos como o Eldorado.

O jogo desenha a vista de cada espelho numa imagem própria na placa de vídeo e depois a mostra no espelho. Os
caminhões com câmera no lugar do retrovisor (MirrorCam) mostram essa mesma imagem numa tela da cabine. O plugin já
intercepta esses desenhos (é assim que põe a tela do WestPlay no painel).

## Etapas

1. **Achar a imagem do retrovisor** (esta, uma versão de teste). Ainda não se sabe qual das imagens do jogo é a do
   espelho (tamanho, formato). O plugin guarda, uma vez, todas as imagens que o jogo desenha num quadro, na ordem, e
   o dono diz qual é o espelho.
2. **Mostrar essa imagem na tela do painel** na ré, com as linhas guia por cima. Tudo na placa de vídeo, sem passar
   pelo app.
3. **Pôr a câmera no lugar certo** (a traseira da carreta ou do caminhão): trocar a posição da câmera desse espelho
   pela traseira, calculada pela telemetria. É a parte difícil. O jogo pode deixar de desenhar o que o espelho de
   verdade não veria, e é preciso descobrir onde ele guarda a posição do espelho. Só dá para saber com o jogo rodando.

## O que a etapa 1 faz (`crates/plugin/src/mirror_probe.rs`)

- Cerca de **1 s depois de engatar a ré** (com a parte elétrica ligada e o jogo sem pausa), o plugin pega **um quadro
  inteiro do jogo**. Cada vez que o jogo termina um passo de desenho (quando troca de alvo), o plugin copia o que foi
  desenhado e salva como imagem, até 160 por quadro.
- Esse quadro **trava um instante** (a placa de vídeo espera cada cópia). Por isso o plugin faz isso no máximo
  **3 vezes por jogo aberto**, com pelo menos ~20 s entre uma e outra, e só uma vez por ré.
- Grava em `Documentos\WestPlay\logs\retrovisor\captura-1`, `captura-2` e `captura-3`. Cada pasta tem as imagens
  (`001_1024x512_f10.bmp`: ordem, tamanho real, formato) e uma `lista.txt` com o que cada uma é. Imagens largas são
  salvas reduzidas a 640 px de largura. As que estão em HDR são clareadas para dar para ver.
- No `plugin.log` aparece `retrovisor: capture N taken in frame ...` e depois `retrovisor: capture saved in ...`.

## Teste para o dono

Reinicie o jogo, porque o plugin mudou. Precisa do jogo rodando.

- [ ] Entre no caminhão, **na câmera da cabine (1)**, com os retrovisores aparecendo na tela, de preferência de dia.
- [ ] Engate a ré e espere 2 s. O jogo trava um instante uma vez: é a captura.
- [ ] Faça de novo, com mais de 20 s entre uma e outra: uma vez **com carreta** e uma **sem**. Se quiser, também uma
      olhando para o retrovisor da direita (mouse).
- [ ] No `plugin.log`, apareceram as linhas `retrovisor: capture ...`?
- [ ] Abra `Documentos\WestPlay\logs\retrovisor\captura-1` e procure uma imagem que seja **a vista do retrovisor**
      (a lateral do caminhão ou da carreta e a estrada atrás, talvez espelhada). Anote o nome do arquivo de cada
      espelho que achar (esquerdo, direito, grande-angular, de baixo).
- [ ] Mande a pasta `retrovisor` (ou os nomes e a `lista.txt`) para a próxima etapa.
- [ ] Anote se o travamento incomodou ou se a captura não aconteceu.

A captura foi instalada junto com os sensores por raio da outra sessão (ponte 39), na mesma DLL. Para voltar ao que
era antes das duas, use as cópias `.antes-raios` (do app, da `ui` e da DLL de cada jogo). A DLL foi trocada de novo
às 20:08 (só uma correção dos raios, ainda com a captura); a anterior a essa ficou como `.antes-raios2`. E de novo às 20:19 (a traseira da carreta medida pela física, ainda
com a captura); a anterior ficou como `.antes-carreta`.

## Etapa 2: o retrovisor na tela (instalada 2026-11-13, 23:50, ponte 40)

Nas capturas do dono (`captura-1`, passo 17; `captura-2`, passo 18), a imagem dos espelhos é uma textura 512x1024
(RGBA8 sRGB, 11 mips): em cima os dois espelhos principais lado a lado (esquerdo e direito, como o motorista vê),
embaixo os grande-angulares. A segunda textura do mesmo tamanho no quadro é a mesma de cabeça para baixo.

- `crates/plugin/src/mirror.rs`: copia essa textura a cada quadro (só enquanto a tela de ré pede) e, no Present,
  monta a tela: os dois espelhos principais (um pouco esticados na largura, centrados, preto em volta) no lugar da
  imagem da câmera, e o quadro do app por cima (selo "Câmera de ré", painel dos sensores).
- A telemetria diz `mirror` (o jogo desenhou os espelhos há menos de meio segundo). Com isso o app deixa a área da
  imagem transparente e manda onde ela fica (`mirrorCamera`). Sem espelhos (câmera de fora), volta o desenho do mapa.
- A captura da etapa 1 foi desligada (`MAX_CAPTURES = 0`): não trava mais o jogo.
- Para voltar ao de antes: cópias `.antes-espelho` (app, `ui` e a DLL de cada jogo).

### Teste para o dono

Reinicie o jogo (o plugin mudou).

- [ ] Na câmera da cabine (1), engate a ré: a tela mostra os dois retrovisores, de verdade, no lugar do verde?
- [ ] O selo e o painel dos sensores aparecem por cima, como antes? O lado direito (atrás do painel) fica preto?
- [ ] A imagem se mexe junto com o jogo (não fica parada) e não pisca?
- [ ] Olhando para os lados com o mouse, a imagem continua?
- [ ] Com a câmera de fora (2), na ré: volta o desenho de antes (o verde), sem tela preta?
- [ ] Teste em outro caminhão (outra marca): os espelhos aparecem certos, ou cortados/trocados?
- [ ] No `plugin.log`: `retrovisor: the game's mirrors found`, `ready to be shown`, `shown on the screen, at [...]`.
- [ ] Ao sair da ré, a tela normal volta inteira (nada transparente, nada faltando)?
- [ ] (2026-11-14) O bipe dos sensores toca só com a câmera dentro da cabine? Na câmera de fora, na ré, fica mudo?
      E voltando para dentro, volta a bipar?
- [ ] (2026-11-14) Ajustes > Câmera de Ré > abaixo de Bipe dos Sensores: a barra de volume toca um bipe a cada passo? Na ré, o bipe sai no volume escolhido (do mais baixo, 10%, ao mais alto, 150%)?

## Etapa 3: a nossa câmera, atrás do caminhão e atrás da carreta (teste instalado 2026-11-14, 11:01)

O dono não quer os dois espelhos juntos: quer uma câmera só nossa, uma atrás do caminhão e outra atrás da carreta.
Não dá para pôr um espelho novo em cada caminhão, porque cada modelo traz os seus (e os de mod pago são fechados).
O plano é refazer o desenho de um espelho com a câmera onde a gente quiser, numa imagem só nossa. Os retrovisores
de verdade continuam iguais.

Para isso é preciso saber onde o jogo guarda a posição da câmera de cada espelho. O
`crates/plugin/src/mirror_camera_probe.rs` descobre isso. Um segundo depois de pôr a ré, com a câmera dentro da
cabine, ele acompanha um quadro inteiro do jogo. Em cada parte (cada espelho e a vista principal), lê os dados que
os shaders recebem nos primeiros desenhos. Grava tudo em `Documents\WestPlay\logs\retrovisor\camera-1.txt`, junto
com a posição do caminhão e da carreta. São 2 capturas por jogo, com 20 s de intervalo (`camera-2.txt`). O jogo dá
uma travadinha de um instante em cada uma.

O dado procurado tem o mesmo conteúdo em todos os desenhos de um espelho, muda de um espelho para o outro e muda
quando o caminhão anda. Por isso a segunda captura vem com o caminhão em outro lugar.

Para voltar ao plugin de antes: cópias `westplay.dll.antes-camera-teste` em cada pasta `plugins` e em
`%LOCALAPPDATA%\WestPlay`.

### Teste para o dono

- [ ] Com carreta engatada, câmera **dentro da cabine** (1ª pessoa), engate a **ré** e espere uns 2 s parado
      (vai dar uma travadinha).
- [ ] Ainda de ré, ande uns 10 m para trás e espere mais 20 s de ré (segunda travadinha).
- [ ] Mande `Documents\WestPlay\logs\retrovisor\camera-1.txt`, `camera-2.txt` e o `plugin.log`.
- [ ] No `plugin.log`: `retrovisor: camera capture 1 taken` e `camera capture saved in`.

### Resultado do teste (2026-11-14, 11:28, `camera-1.txt`)

- Os espelhos são desenhos à parte, cada um no seu alvo em HDR (R11G11B10): um de 256x128, dois de 256x256 e dois
  de 256x512 (os dois principais). Depois eles são montados no 512x1024. A vista principal é 1920x1080.
- Cada desenho recebe, no b0 (`VSSetConstantBuffers1`, um buffer em anel), um bloco de 256 bytes. Ele traz a
  matriz modelo-vista do objeto (linhas 0 a 3), e essa matriz já vem relativa à câmera (a translação é a do objeto
  visto da câmera). Traz também ela vezes a projeção (linhas 4 a 7, e de novo 8 a 11 com outro z). A projeção do
  espelho é 1,92098 x 0,96049.
- Por isso, mover a câmera de um espelho exige corrigir o bloco de cada objeto, em cada desenho do espelho. Esses
  dados estão na placa de vídeo, e o jogo os escreve direto no buffer em anel.

### Etapa 3b: achar a câmera do espelho na memória do jogo (instalada 2026-11-14, 11:41)

Ideia do dono: mexer na câmera como o modo personagem mexe no boneco, direto na memória do jogo. Assim o próprio
jogo desenha o espelho com a câmera atrás da carreta, e não é preciso corrigir cada desenho na placa de vídeo.

A cada captura, o `mirror_memory_scan.rs` pega a direção de cada espelho (e a da vista principal) num desenho do céu.
Depois procura essa direção na memória do jogo: linhas, colunas, quaternion, em float e em double. Para cada lugar
achado, anota os números em volta, com `*` nos que estão perto da posição do caminhão. Grava tudo em
`camera-N-memoria.txt`. A captura 2 também diz o que os lugares da captura 1 guardam naquele momento: o lugar
certo acompanha o caminhão quando ele vira.

- [ ] Cabine, ré, parado: espere 2 s (travadinha; a busca roda depois, sem travar).
- [ ] Ainda de ré, **vire o volante** e ande de ré fazendo uma curva por uns 20 s (segunda travadinha). A direção
      dos espelhos precisa mudar entre as duas capturas.
- [ ] Avise: eu leio `camera-1-memoria.txt`, `camera-2-memoria.txt` e o `plugin.log`.

### Resultado da 3b e próximo passo (2026-11-14, 11:57)

A busca leu 9,6 GB em 15 s. Ela não achou a direção dos espelhos guardada de forma fixa. Os lugares achados eram
cópias de passagem dos dados de desenho (na captura 2 já guardavam outra coisa) ou a pilha da própria busca. O jogo
recalcula a direção a cada quadro a partir do caminhão.

Outro caminho, mais barato: antes de cada desenho do espelho, abrir o buffer em anel (`Map` NO_OVERWRITE) e
reescrever o bloco do objeto com a nossa câmera, `MV_novo = V_novo · R_espelho^T · MV`, e as linhas da projeção
refeitas. Para isso é preciso saber onde fica o espelho. A captura agora tem uma tabela com cada desenho dos dois
espelhos principais e da vista principal: a malha (index buffer, quantidade, início) e onde o objeto está visto da
câmera. A mesma malha vista nos dois lugares dá a posição do espelho em relação à cabeça, e a cabeça o SDK já dá.
O teste é o mesmo de antes (ré na cabine; depois de ré com o volante virado).

### Etapa 3c: a nossa câmera pelo espelho (primeiro teste instalado 2026-11-14, 12:06)

O `crates/plugin/src/rear_camera.rs` só age enquanto a câmera de ré está na tela. Em cada desenho do segundo
espelho principal do quadro, ele abre o bloco do objeto no buffer em anel (`Map` NO_OVERWRITE) e o reescreve como
visto da nossa câmera: modelo-vista movido, e as projeções tiradas do próprio bloco (P = PMV · MV⁻¹) e aplicadas
de novo. Neste primeiro teste a câmera só anda 12 m para onde o espelho olha (para trás, ao longo da carreta), para
ver se o mecanismo funciona. As capturas da etapa 3 estão desligadas (`MAX_CAPTURES = 0`).

- [ ] Na ré, na tela, a metade da **direita** mostra a vista uns 12 m mais para trás (passou da carreta)?
- [ ] A metade da esquerda continua o espelho normal?
- [ ] O espelho direito de verdade, na cabine, também mostra a vista movida? É esperado neste teste.
- [ ] O jogo não perdeu FPS nem travou? No `plugin.log`, `our camera on`, e a cada 10 s
      `blocks moved, ... left, ... failed`.

Resultado (12:21): **funcionou**. O espelho esquerdo (o segundo principal do quadro) passou a ver uns 12 m mais
para trás, e o jogo ficou a 60 fps. Mas o chão sumiu (cinza): o espelho olha um pouco para baixo, então andar na
direção dele levou a câmera para baixo do chão. Corrigido às 12:13 (DLL nova): agora ela anda 14 m na horizontal,
para onde o espelho olha visto de cima, na mesma altura. O "horizontal" vem da própria direção do espelho, lida no
bloco do céu.

- [ ] Na ré, a metade **esquerda** da tela (e o espelho esquerdo da cabine) mostra o chão e a vista de uns 14 m
      mais para trás, na altura do espelho?

Resultado (12:30): o chão voltou, a câmera ficou 14 m para trás na altura do espelho, 60 fps. Ainda olhava torto,
no ângulo do espelho.

Terceiro teste (DLL de 12:19): a câmera fica **no meio da traseira**. Sem carreta, 1,8 m de altura e 1,4 m atrás
da quinta roda. Com carreta engatada, 3,2 m de altura e 2 m atrás do eixo traseiro da carreta. Ela olha reto para
trás, 18° para baixo. Tudo vem das posições e direções que o SDK dá. A direção (o "para trás" do heading é
(sen, 0, cos)) foi confirmada nas 8 capturas. A posição do espelho em relação à cabeça é estimada (1,45 m para o
lado dele), então a câmera pode sair uns palmos para um lado.

- [ ] Sem carreta: a metade esquerda da tela mostra o chão logo atrás do caminhão, olhando reto para trás?
- [ ] Com carreta: mostra o que está atrás da carreta?
- [ ] Ao virar, a câmera acompanha o caminhão (sem pular nem tremer)?

Resultado (12:38): no lugar certo. Com carreta, ela via a colheitadeira por trás; sem carreta, o chão atrás do
caminhão. Ficou a 60 fps, mas **de cabeça para baixo**. O jogo desenha o espelho girado e desvira na hora de
mostrar. Corrigido (DLL de 12:28): a nossa câmera copia o giro do espelho original em volta do eixo de visão
(`rolled_as`).

- [ ] A imagem da câmera está de pé (céu em cima, chão embaixo) e não espelhada?

Resultado (13:41): de pé e atrás do caminhão (via a carreta estacionada por trás), 60 fps. O log (`our camera plan`)
mostrou que a direção tirada do céu está torta: o céu é um modelo girado no próprio modelo, com z para cima. Só
dava certo com o caminhão virado para o norte. Corrigido (DLL de 12:54): a direção do espelho agora é a mais comum
entre os objetos dele num quadro (o chão e as estradas, que não têm giro próprio), usada no quadro seguinte.

- [ ] Com o caminhão virado para várias direções (norte, sul, leste, oeste), a câmera sempre mostra o que está
      atrás dele?

Resultado (13:00, com a carreta solta atrás): estava certo, mas a lente era fechada demais (a do espelho, uns 55°).
A câmera também ficava alta demais (1,8 m) e entrava na plataforma da carreta ao encostar.

Quarto teste (DLL de 13:09):
- **Altura do para-choque:** 1,05 m no caminhão e 1,1 m na carreta. A câmera olha 22° para baixo.
- **Lente de 120°** na largura da tela. A altura acompanha o formato da área da câmera (1,28 : 1), para não
  distorcer.
- **Tela cheia:** a câmera é desenhada deitada (um quarto de volta) dentro do espelho, para aproveitar a imagem
  alta dele. A tela mostra só ela, desvirada, na área toda (`mirror::ours_place`; o lugar dela no retrovisor é
  `rear_camera::PLACE`, e o jeito de desvirar é `FLIP`).

- [ ] A tela mostra só a nossa câmera, inteira e sem a divisão no meio?
- [ ] Está de pé, nem deitada nem de cabeça para baixo? Do lado certo? Se não, mande um print: eu acerto o `FLIP`.
- [ ] Enxerga bem mais aberto, na altura do para-choque, sem entrar na carreta ao encostar?

Resultado (13:15): tela cheia funcionando e de pé. A câmera estava embaixo da carreta, entre os eixos (o fim dela
fica 5,46 m depois da origem, não 2 m) e fora do centro no caminhão. Corrigido (DLL de 13:16):
- o fim de cada um vem de `reversing::VEHICLES` (rear_z, tirado do último eixo); a câmera fica 0,15 m depois dele;
- a distância do espelho ao meio do caminhão agora é **medida**: as peças do caminhão (giradas como ele) vistas
  do espelho dão a origem do caminhão. O valor sai no log como `mirror side measured`.

Quinto teste (13:25, DLL e `ui` novos):
- a câmera ocupa a área toda; o quadro dos sensores fica por cima, à direita, com fundo quase transparente;
- **linhas de guia na nossa câmera** (`ui/src/cabin/reversing/ourCamera.ts`): o mesmo trajeto de ré da câmera do
  mapa (`geometry.ts`, que vira com o volante e segue a carreta), projetado com a geometria da nossa câmera. Os
  números são os mesmos do `rear_camera.rs`, que tem de andar junto. Com `MIRRORED` o lado direito do caminhão sai
  à direita da tela;
- a lente usa o formato real da área (`mirror::shape`), com pixels quadrados.

- [ ] A câmera ocupa a tela toda e o sensor fica por cima, transparente?
- [ ] As linhas aparecem no chão, saindo da traseira? Ao virar o volante, curvam para o lado para onde a traseira
      vai de verdade? Se curvarem ao contrário, é o `MIRRORED`.
- [ ] As marcas de 1, 2 e 3 m batem com a distância real (compare com o sensor)?

Resultado (13:45): tela cheia, sensor por cima, **linhas de guia aparecendo** e acompanhando a imagem. Mas a
câmera da carreta passou um pouco do para-choque (quase na bomba atrás dele), porque o fim calculado pelo último
eixo exagera cerca de 1 m. E a do caminhão ainda entrava um pouco na carreta. Ajustado (DLL e `ui` de 13:28):
- carreta: a câmera e o começo das linhas ficam 1 m antes do fim calculado (`TRAILER_IN`, nos dois lados);
- caminhão: a altura passou de 1,05 m para 0,85 m (`HEIGHT_TRUCK`, nos dois lados).

- [ ] Carreta: a câmera fica logo no para-choque, sem passar dele e sem ficar embaixo da carreta?
- [ ] Caminhão: não entra mais na carreta quando encosta?

Resultado (13:50): com o caminhão sozinho, as linhas curvavam para o lado contrário ao volante; com a carreta
pareciam certas. A causa era a imagem, não o volante: ela saía sem espelhar, como vista olhando para trás (a bomba à
direita da carreta aparecia à esquerda da tela). Já as linhas seguem o padrão de câmera de ré, com o lado direito
do caminhão à direita da tela. Corrigido (DLL de 13:34): a tela espelha a imagem (`FLIP = [false, true]`), como os
retrovisores e as câmeras de carro.

- [ ] O que está à direita do caminhão aparece à direita da tela?
- [ ] Caminhão sozinho: virando o volante, as linhas curvam para onde a traseira vai?
- [ ] Com carreta: as linhas continuam certas?

### Outros caminhões (2026-11-14, 13:40)

No Actros e no MAN, que têm espelho de vidro, a tela mostrou a câmera do mapa: o plugin não achou o retrovisor
montado (512x1024). Até agora só o DAF XG com espelhos de câmera foi testado. Para ver como o jogo desenha o
espelho de vidro, a captura da etapa 1 (`mirror_probe.rs`) voltou para 1 por jogo, e só enquanto os espelhos não
são achados (`mirror::seen`). As capturas antigas do DAF foram renomeadas para `daf-captura-*`.

- [ ] Num caminhão de espelho de vidro (Actros ou MAN), na cabine, engatar a ré e esperar uns 2 s (dá uma
      travadinha). Avisar: eu leio `logs\retrovisor\captura-1` (a lista e as imagens de cada passo do quadro).

Resultado do Scania (`captura-1`, 13:45). Ele desenha o espelho principal igual ao DAF: um alvo seu, 256x512
R11G11B10. Mas tem só um desse tamanho (o outro lado sai em 256x256), e **não tem** o retrovisor montado de
512x1024 (os 512x512 dele são as telas do painel). O plugin procurava o segundo 256x512 e a montagem, então não
achava nada.

Para qualquer caminhão (DLL de 13:49):
- o nosso espelho é o **último** alvo "retrato" R11G11B10 (altura = 2 × largura) do quadro, contados no quadro
  anterior. No DAF dá o mesmo de antes; no Scania, o único (`mirror::drawing_ours`);
- a tela pega a imagem **direto desse alvo**, e não da montagem. A cor vem crua (HDR, sem tom), e a tela ajusta
  com 1 − e^(−x · `EXPOSURE`);
- `mirror::seen` (e o `mirror` da telemetria) agora quer dizer "o nosso espelho foi desenhado há pouco";
- o jeito de desvirar mudou junto (`FLIP = [false, false]`), porque o alvo vem de cabeça para baixo em relação à
  montagem.

- [ ] Scania, Actros, MAN, DAF: a câmera aparece em todos?
- [ ] A imagem está de pé e com o lado certo (o da direita à direita)? Se não, mande print: é o `FLIP`.
- [ ] O brilho está bom (nem escuro nem estourado)? É o `EXPOSURE`.

Resultado do Scania (14:00): o plugin achou o espelho certo e moveu a câmera, que aparece no espelho grande
esquerdo da cabine. Mas a tela ficou **preta**. Com zoom, voltou para a câmera do mapa, provavelmente porque o
espelho não é desenhado quando sai de vista. Diagnóstico (DLL de 13:56):
- a cada 5 s o log diz a cor média do meio da cópia (`our mirror's copy ... middle's mean colour`), para separar
  imagem vazia de imagem só escura;
- quando muda, o log diz quais espelhos o jogo desenhou no quadro (`the game drew these mirrors this frame`). Assim
  dá para ver se, com zoom ou olhando para o lado, ele para de desenhar todos ou só o nosso.

- [ ] Scania: ré na cabine por uns 10 s; depois dê zoom na tela e olhe para os lados, devagar. Avise.

Diagnóstico (14:05):
- A cópia chega com conteúdo, mas **escura demais**: média de cerca de 0,0007. O alvo do espelho guarda a cor antes
  da exposição do jogo. Corrigido (DLL de 14:00) com **exposição automática**: a cópia ganha todos os mips
  (`GenerateMips` a cada quadro), o menor dá a média, e a imagem é ajustada para que a média fique num cinza
  médio (`EXPOSURE` 0,35) antes de tonar.
- **Espelho fora de vista:** quando nenhum espelho aparece na tela (zoom na tela, olhar para longe), o jogo não
  desenha **nenhum**, e a câmera cai para a do mapa. Isso é limite do jeito "pelo espelho". O retrovisor aparece
  na vista normal da cabine.

- [ ] A imagem aparece, nem preta nem estourada, no Scania (e no DAF)?

Ajustes de 14:10 (DLL):
- **Altura a partir do chão:** a origem do modelo não fica no chão em todo caminhão (no Scania, a câmera ficou
  quase no chão). O plugin agora lê o raio das rodas (`wheel.radius`) e calcula o chão como a altura do eixo menos o
  raio (`reversing::Layout::ground_y`). A câmera fica em `ground_y + HEIGHT`, no caminhão e na carreta.
- **Investigação do zoom** (`crates/plugin/src/query_probe.rs`): conta as consultas à placa de vídeo do tipo
  "isto está aparecendo?" (as de oclusão), quantas respondem "nada visível" e se há desenho condicionado a elas
  (`SetPredication`). Escreve no log a cada 2 s em ré (`queries: ...`), junto com `mirrors seen`. Se o jogo
  decidir assim se desenha o espelho, dá para responder "visível" e o espelho passa a ser desenhado sempre.

- [ ] Scania: a câmera está na altura do para-choque?
- [ ] Em ré, uns 10 s na vista normal e uns 10 s com zoom na tela. Avise: eu leio as linhas `queries:` do log.

Resultado (14:25): a imagem aparece com brilho certo, até à noite (exposição automática OK).
- **Zoom:** o jogo não usa nenhuma consulta de oclusão (`queries: made {}, answered {}`, nenhum `SetPredication`).
  Ele decide no próprio código (CPU) se desenha o espelho, então não dá para enganá-lo pela placa de vídeo.
- **Câmera baixa no Scania:** o chão estava na origem (`ground_y` ≈ 0). O erro era a altura do espelho, estimada
  como cabeça + 0,15 m; a câmera é posta a partir do espelho e desce junto. Corrigido (DLL de 14:19): a altura e a
  posição no comprimento também são **medidas** pelas peças do caminhão vistas no espelho, como já era o lado
  (`MIRROR_UP_BACK`, com checagem de plausibilidade). O log mostra em `up and back`.

- [ ] Scania: a câmera está na altura do para-choque (a da empilhadeira, mais ou menos)?

### Achar no jogo a regra "espelho fora de vista não é desenhado" (2026-11-14, 14:25)

O dono quer a câmera com zoom na tela (a tela é pequena), sem janela no monitor. No DAF o zoom não tirava a câmera,
porque os espelhos de câmera (MirrorCam) são desenhados sempre. Os de vidro, só quando aparecem na vista. O jogo
decide isso no código dele: não usa consulta de oclusão. A captura com zoom mostrou que nesse momento nenhuma outra
vista colorida da cena é desenhada, então não há outro desenho em que pendurar a câmera.

Primeiro passo (`crates/plugin/src/mirror_stack.rs`, DLL de 14:24): quando o espelho principal começa a ser
desenhado, a pilha de chamadas é gravada 3 vezes por jogo, em `logs\retrovisor\pilha-N.txt`. Para cada função do
jogo, grava onde ela começa e o código de máquina até o retorno. A regra fica numa dessas funções, antes de mandar
desenhar o espelho. Ela será lida com um desmontador e desligada durante a ré, como o modo personagem faz.

- [ ] Scania: ficar uns 10 s na cabine, na vista normal, com o espelho esquerdo à vista. Não precisa dar ré. Avisar.

Resultado (14:35): com os **retrovisores virtuais** do jogo ligados, a câmera não some mais com zoom, porque o
jogo desenha os espelhos virtuais sempre. Os comandos de console (`g_mirrors_visibility_setup` estava em 0, o
padrão é 5; `g_mirrors` estava em 0, o padrão é 1) não resolveram. Sobraram umas piscadas. Pelo log, com os
virtuais ligados o jogo desenha **dois** 256x512 (o de vidro e o virtual), em ordem que muda de quadro para quadro,
e o plugin alternava entre eles.

Corrigido (DLL de 14:41): o plugin **trava** num espelho, pela textura dele. Cada quadro em que um espelho é
desenhado soma 1 ponto, e os pontos de todos vão diminuindo. O nosso é o mais constante. Só troca se ele sumir por
mais de 1 s, ou se outro for desenhado bem mais vezes; com zoom isso leva ao virtual. O log diz
`our mirror is now ...` a cada troca.

- [ ] Com os retrovisores virtuais ligados: sem piscar, com e sem zoom?
- [ ] Sem os virtuais: tudo como antes na vista normal?

Resultado (14:45): **funcionou perfeitamente** com os retrovisores virtuais ligados. O dono não quer os espelhos
virtuais na tela.

Esconder os virtuais (`crates/plugin/src/hide_virtual.rs`, DLL de 14:46): o jogo continua desenhando os
espelhos, mas o desenho que os cola na tela é pulado. Esse desenho é o que vai para a imagem final (8 bits, tela
inteira) com a imagem de um espelho ligada no pixel shader. As imagens de espelho são os alvos de 256 de largura em
R11G11B10 ou RGBA8 sRGB, ou a montagem do DAF, anotados quando o jogo desenha neles. Os espelhos da cabine (que vão
para a cena) e a nossa câmera (montada pelo plugin) não mudam. Fica sempre ativo (`HIDE`). O log diz
`the virtual mirrors hidden` na primeira vez.

- [ ] Com os retrovisores virtuais ligados no jogo: eles somem da tela, e a câmera continua com zoom?
- [ ] Nada mais sumiu da interface (mapa, velocímetro, avisos do jogo)?

Câmera no ponto do sensor (DLL e `ui` de 14:48). O dono notou que o sensor fica certo em qualquer carreta, mas a
câmera não: às vezes entrava no objeto enquanto o sensor ainda via distância. O sensor mede a partir do fim
calculado pelos eixos (`rearOf` no `ui`, `rear_z` no plugin), sem ajuste nenhum. A câmera tinha dois ajustes
somados nos testes: `PAST_REAR` 0,15 m e `TRAILER_IN` 1 m. O de 1 m compensava a posição do espelho, que era
chutada e hoje é medida. Agora os dois são 0, no plugin e nas linhas: a câmera fica exatamente onde o sensor mede.

- [ ] Caminhão e carreta: quando o sensor marca uma distância, a câmera mostra o objeto a essa distância (sem
      entrar nele antes)?

Resultado (14:55):
- **Posição:** ainda entrava no objeto com o sensor no vermelho. Causa: o sensor da carreta usa o fim **medido**
  pelos raios da física do jogo (`ray.rs` soma a medida ao `trailer_rear_z` da telemetria publicada), e a câmera
  usava o fim calculado pelos eixos, sem a medida. Corrigido (DLL de 14:54): a câmera lê o fim da telemetria
  publicada, o mesmo do sensor e das linhas.
- **Esconder os virtuais:** sumiu a imagem, mas ficaram as **molduras cinza**. E o dono quer poder usar os
  espelhos virtuais quando quiser. A ideia é virar uma opção nos Ajustes, e durante a ré o espelho mostrar a imagem
  normal (com a troca de quadros). Antes, testar no console se `r_mirror_scale_x` e `r_mirror_scale_y` diminuem só a
  janela na tela, sem diminuir a imagem do espelho.

- [ ] Carreta e caminhão: a câmera chega no objeto junto com o "pare" do sensor?
- [ ] Console: `r_mirror_scale_x 0.1` e `r_mirror_scale_y 0.1`. Os virtuais encolhem na tela? A câmera continua
      nítida e com zoom? (Para voltar: os dois em 1.)

Resultado (15:00): **a posição da câmera ficou como o dono queria**, no ponto do sensor. Os espelhos virtuais
continuam com as molduras cinza: `r_mirror_scale` só aceita de 0,5 a 2 e não mexe na janela;
`g_mirrors_keep_hidden 1` não esconde. Para esconder também a moldura (DLL de 15:01), uma captura grava os
desenhos da imagem final de um quadro, com o que cada um tem ligado. Ela roda uma vez por jogo, depois de uns
segundos com os virtuais escondidos, e grava em `logs\retrovisor\interface.txt`. Depois vem a opção nos Ajustes
para mostrar ou esconder os virtuais.

- [ ] Ligar o jogo com os retrovisores virtuais ligados e ficar uns 20 s na cabine (sem precisar de ré). Avisar.

`interface.txt` (15:05): na passada final, o jogo desenha só uma cópia da tela inteira e, para cada espelho virtual,
a moldura (6 índices, textura 1x1) logo seguida da imagem. A moldura agora também é pulada (DLL de 15:08).

O que o dono quer: com a opção "esconder retrovisores" ligada, o plugin **liga os virtuais só durante a ré** e
desliga depois, porque eles pesam. Com a opção desligada, eles ficam como o jogo estiver. Para isso, achar a
variável que liga os virtuais em tempo real (`g_mirrors` é a candidata: estava 0 com eles desligados, e o padrão é
1) e um jeito de o plugin mudá-la.

- [ ] Molduras sumiram (com os virtuais ligados)?
- [ ] Console, com os virtuais **ligados** nas opções: o que diz `g_mirrors`? E com eles **desligados**?
- [ ] Com eles desligados nas opções, digitar `g_mirrors 1`: a câmera passa a funcionar com zoom?

Resultado (15:12): **erro meu.** A regra "todo retângulo liso da passada final" escondeu também os fundos dos
painéis do menu e das mensagens do jogo. Corrigido (DLL de 15:15): só some o retângulo liso que, no quadro
anterior, veio **logo antes** de uma imagem de espelho. A ordem dos desenhos é a mesma de um quadro para o outro.
- `g_mirrors` é o interruptor dos virtuais: **2** com eles ligados nas opções, **0** desligados. `g_mirrors 1` não
  serviu; falta testar `g_mirrors 2`.

- [ ] Os painéis do menu e das mensagens voltaram? As molduras dos espelhos sumiram?
- [ ] Virtuais desligados nas opções, `g_mirrors 2` no console: a câmera funciona com zoom?

Resultado (15:20): `g_mirrors 2` sozinho não liga os virtuais; só o F2. Com o F2, os virtuais ficam escondidos e
a câmera funciona com zoom, mas com umas piscadinhas.

DLL de 15:21:
- `crates/plugin/src/virtual_toggle.rs`: na ré, na cabine, com os virtuais desligados, o plugin aperta F2 sozinho
  (código físico da tecla, por `SendInput`, que o DirectInput do jogo lê). Ao sair da ré, se foi ele que ligou e
  eles continuam ligados, aperta de novo. Se estão ligados vem de os virtuais estarem sendo desenhados
  (`hide_virtual::virtual_drawn`), então ele não desfaz um F2 do dono. No log: `turned on for reversing` /
  `turned off again`.
- Piscadas: quando o espelho travado some (o de vidro fora de vista), o plugin passava para outro só depois de 1 s.
  Agora passa depois de 3 quadros, e fica com o virtual, que nunca some.

- [ ] Sem apertar F2: engatar a ré liga os virtuais (escondidos), e a câmera funciona com zoom?
- [ ] Ao sair da ré, eles desligam (o FPS volta)?
- [ ] Sem piscadas?

O F2 é um ciclo: desligado → um lado → os dois → desligado. Com um lado só, a tela falha. DLL de 15:29:
- `hide_virtual::virtual_count`: quantos espelhos virtuais o jogo desenha (o maior de cada um dos últimos 10
  quadros), escondidos ou não;
- `virtual_toggle`: na ré, na cabine, aperta F2 até haver **2**, com 45 quadros entre os apertos. Ao sair, se foi
  ele que ligou, aperta até não haver nenhum. No máximo 3 apertos para ligar e 3 para desligar. Se o dono já tinha
  os dois ligados, deixa como está. No log: `F2 pressed (...)`.

- [ ] Na ré: dois apertos, câmera ok com zoom, sem piscar? Ao sair: um aperto, FPS volta?

Resultado (15:40): **o liga e desliga automático funcionou perfeitamente**, e o painel não falha mais. Sobraram
piscadinhas escuras onde ficam os espelhos virtuais: a moldura escapando num quadro em que a ordem dos desenhos
mudou. DLL de 15:40: as posições de moldura lembradas são as dos últimos 30 quadros, e não só as do anterior.
Valem só enquanto os virtuais estão sendo desenhados (`virtual_count() > 0`), então os menus não são afetados.

- [ ] Sem piscadas na região dos espelhos virtuais? Menus e mensagens normais?

Modo personagem (15:44): os virtuais ficaram ligados e visíveis, com a imagem de teste do jogo ("close"/"far"),
porque fora de veículo o jogo não tem vista para eles. Pelo log, aconteceram duas coisas:
- logo depois de um F2 para desligar, a contagem ainda via 1 (o jogo leva um tempo para tirá-los), e um segundo
  aperto religou um;
- fora da cabine a contagem não vale (a imagem de teste não é de espelho), então o plugin achava que estavam
  desligados.

Corrigido (DLL de 15:45): o plugin guarda o ponto do ciclo do F2 (`CYCLE`: 0, 1, 2), avançando um a cada aperto.
Dentro da cabine, a contagem corrige esse ponto. Depois de um aperto para desligar, espera 3 s (`SETTLE_OFF`)
antes de contar de novo.

- [ ] Ré, depois sair da ré e descer do caminhão (F8): os virtuais desligam e nada aparece no modo personagem?

15:50: pelo log, o ciclo do F2 tem **quatro** passos: desligado → um lado → os dois → o outro lado → desligado.
Com os dois ligados, um aperto deixava um; com um ligado, um aperto desligava. O plugin se perdia, e às vezes
ficava apertando três vezes. Corrigido (DLL de 15:52):
- `STEP` segue os quatro passos (`SHOWN` = 0, 1, 2, 1). Dentro da cabine, a contagem acerta o passo;
- 1,5 s (`SETTLE` 90 quadros) depois de cada aperto antes de contar ou apertar de novo; no máximo 4 apertos;
- "fora da cabine" vale assim que a tecla de descer é apertada (`character::walking`), e não só quando a cabine
  some. Os virtuais desligam antes da animação de descida.
- Falta saber **o que pisca** andando para frente: o rear_camera não age fora da ré.

- [ ] Ré: os dois virtuais ligam (dois apertos)? Para frente: desligam? Descendo do caminhão: nada aparece?

O que pisca (15:54): a sombra da moldura dos virtuais, de vez em quando, sempre que eles estão ligados. Regra
nova (DLL de 15:55), somada à das posições lembradas: logo depois do desenho da tela inteira (`Draw` de 3 pontos)
começa uma sequência (`RUN`). Enquanto vierem retângulos lisos ou imagens de espelho, todos são pulados. O
primeiro desenho diferente encerra a sequência até a próxima passada. Só com os virtuais ligados
(`virtual_count() > 0`).

- [ ] Com os virtuais ligados (na ré): nenhuma sombra piscando? Menus e mensagens normais?

16:15, muitos problemas: virtuais aparecendo, somindo sozinhos, câmera caindo para a tela verde, F2 apertado 4
vezes seguidas, sensor parado. Duas causas:
- **Erro meu:** a lista de texturas de espelho (`hide_virtual::MIRRORS`) tinha limite de 64 e só crescia. O jogo
  recria as texturas (ao trocar de caminhão, ao dar zoom), a lista lotou com as velhas e as novas não eram
  reconhecidas. Os virtuais deixavam de ser escondidos, a contagem dava 0 e o F2 se perdia. Agora cada textura
  guarda o quadro em que foi desenhada pela última vez, e só contam as dos últimos 600 quadros (DLL de 16:16).
- **Sensor:** o `ets2\rays.txt` estava em "off", porque o jogo fechou durante o teste dos raios (aquela vez em que
  ele não abriu). Esse é o guarda do `ray.rs`. O arquivo foi renomeado para `rays.txt.antes`, e os raios tentam de
  novo na próxima vez que o jogo abrir.

- [ ] O sensor voltou?
- [ ] Na ré: dois apertos de F2, virtuais escondidos, câmera estável? Ao sair: desligam?

18:00, câmera "estranha": ela via a parte de baixo do caminhão, como se estivesse dentro do chassi. No log, a
direção do espelho (`looks`) ficava idêntica em todo registro, mesmo com o caminhão virando. Com os virtuais
ligados há vários espelhos do mesmo tamanho (o de vidro e os dois virtuais, de lados opostos), e o plugin troca
de um para outro. As medidas guardadas (direção, lado, altura e posição) se misturavam entre espelhos diferentes.
Corrigido (DLL de 18:04): `mirror::ours()` diz qual é o espelho atual. Quando ele muda, o `rear_camera` zera
`TURN`, `MIRROR_SIDE`, `MIRROR_UP_BACK` e as contagens, e mede de novo só com o espelho novo.

- [ ] Câmera atrás do caminhão de novo, estável, com e sem zoom?

Retrovisor de vidro normal na ré (DLL de 18:07): o dono quer o espelho da cabine mostrando a imagem normal
durante a ré. Como na ré os virtuais estão ligados e escondidos, a câmera passa a ser feita só de um espelho
**virtual**. O `hide_virtual` anota as texturas que o jogo cola na tela (as dos virtuais; as de vidro vão para a
cena) em `VIRTUAL`, e `is_virtual()` responde se uma textura é delas. O `mirror.rs` só escolhe entre as virtuais
quando há alguma, e troca se estiver num de vidro. Sem virtuais, vale a regra de antes. Isso depende de o jogo
desenhar o virtual numa imagem separada da do vidro.

- [ ] Na ré: o retrovisor de vidro da cabine mostra a imagem normal, e a câmera no painel continua certa?

### 3d. O espelho de vidro e o virtual são a mesma imagem (2026-11-14, 18:30)

O teste das 18:07 mostrou que o jogo usa **uma só textura** para o retrovisor de vidro da cabine e para o virtual (F2): ligar os virtuais não cria espelho novo, sempre há duas 256x512. Por isso a câmera, feita dessa textura, aparecia também no vidro.

Correção: com a câmera ligada, os quadros alternam. Num, o espelho é desenhado normal e o plugin guarda uma cópia. No seguinte, ele é desenhado como câmera, copiado para o painel, e recebe de volta a imagem normal antes de o jogo usá-la no vidro (no `PSSetShaderResources`). Câmera e vidro ficam com metade dos quadros cada. O log diz a cada 600 quadros: "the glass mirror given back its own view N times, not M".

A medida do lugar do espelho agora fica guardada por espelho (antes era zerada ao trocar). O log também mostra por que ela falha: "the truck's parts looked for: N blocks near the mirror, the closest turn off by X".

- [ ] Na ré: o vidro mostra a imagem normal e o painel mostra a câmera atrás do caminhão?
- [ ] O vidro ou a câmera tremem/piscam?

Resultado 18:45: o vidro ficou normal na ré ("given back 300 times, not 0"). Mas a câmera foi parar debaixo do caminhão: numa rua em ladeira, as peças do caminhão ficavam 0.081 fora do ângulo esperado (o limite é 0.04), porque a medida usava só a direção, sem a inclinação. Correção: a medida agora usa a direção, o pitch e o roll do caminhão, e só vale com lado, altura e distância juntos.

Resultado 19:03: sensor voltou (guarda dos raios: 120 rodadas em vez de 600). A medida do espelho deu certo (lado 1,32, altura 2,48), mas a câmera olhava para a frente. A direção do espelho era votada entre os objetos desenhados nele, e na cidade as casas e cercas alinhadas com a rua ganhavam do chão, deixando o espelho 130 graus fora da traseira. Correção: só vale um voto que olhe para trás do caminhão, num raio de uns 50 graus, e com o "para cima" do mundo. O log mostra "the mirror's turn taken N times; the most voted one refused".

Resultado 19:20: direção certa (portas 01/02 aparecem). Mas, conforme o ângulo da cabeça, aparecia uma peça escura em cima e a imagem "andava". O jogo passa as imagens dos espelhos por um rodízio de texturas: 4 endereços para 2 espelhos, e uma textura fica ora com o espelho esquerdo, ora com o direito. Por isso a medida misturava os lados (1,33 → −0,44 → −1,38), e a câmera ia para uns 2,7 m fora do lugar. Correções:
- as medidas agora são guardadas pelo espelho (a direção que ele olha), não pela textura;
- a cada quadro de câmera, o plugin confere pelos próprios objetos desenhados se o espelho é o esperado; se não for, o quadro não vai para o painel. O log mostra "our camera's picture not shown N times, of the other mirror".
- também corrigido: a linha de diagnóstico era escrita uma vez por objeto, e o log chegou a 138 MB.

Resultado 19:30: ainda trocava de lado, com uma barra em cima. A direção [0,116, 0, 0,993] aparecia igual com o caminhão em 0,63 e em 0,13, então não era espelho: era das casas do bairro. Com o caminhão em 0,13, ela ficou a 40° da traseira e passou no filtro de 50°. Os espelhos de verdade olham a 8–14° da traseira (Scania mod e o outro caminhão), então o filtro foi para 20°.

Resultado 19:40: lado certo e estável (espelho medido 1,33 m). Ainda havia uma barra escura em cima (o chassi do Scania mod) e mexidinhas para o lado. Correções:
- a câmera não desenha as peças do próprio caminhão, que o plugin já reconhecia para medir o espelho (giradas como o caminhão, a menos de 12 m). O vidro continua mostrando o caminhão. A constante HIDE_TRUCK desliga isso;
- a suavização da medida do espelho ficou mais rápida (0,5 em vez de 0,1 por amostra), para acompanhar o balanço da cabine.
- [ ] Sem a barra em cima? Sem pedaços do caminhão faltando de forma estranha?
- [ ] Sem mexidinhas para o lado?

Resultado 19:50 (camera-draws.txt): a barra era de peças do Scania mod cujo bloco vem num formato deslocado. A primeira linha é [8.24, 20, 4.5, 1] e a matriz do caminhão vem nas três linhas de baixo. O plugin não as movia e elas eram desenhadas do ponto de vista do retrovisor. Agora qualquer bloco que não pode ser movido, e que não seja a identidade dos passes de tela cheia, fica fora da câmera. O F2 ficou certo: 2 apertos para ligar e 2 para desligar, com SETTLE de 150 quadros.

Resultado 19:50 (log "frame let through"): normalmente as molduras ficam nas posições 6 e 8 do passe final. Nas piscadas ficavam em 7 e 9, porque outro elemento do painel do jogo aparecia antes de vez em quando. A regra por posição as perdia, e a RUN não cobre porque há outros desenhos entre o de tela cheia e as molduras. Correção: as molduras também são reconhecidas pela posição entre os quadrados lisos do quadro, que não muda com um ícone a mais.
- [ ] Ainda piscam? Se piscar, o log mostra "a virtual mirror's frame let through".

Resultado 20:05: mudando as opções de retrovisor do jogo (alcance, resolução, qualidade), a câmera quebrava. O plugin só reconhecia espelhos de 256 de largura em R11G11B10, e com as opções mais altas o jogo passou a usar 512x1024 em RGBA16F (formato 10), com outras imagens de 16 bits junto (formato 12) e profundidade (formato 20). Agora o plugin aceita de 128 a 2048 de largura, em R11G11B10 ou RGBA16F. O log "the game drew these mirrors" mostra também o formato e quantas imagens o jogo desenha juntas, quando não é o padrão.
- [ ] Testar cada opção de retrovisor sozinha (resolução; alcance; qualidade/luz) e ver se a câmera continua certa.

Resultado 20:20: com a qualidade dos retrovisores alta, o jogo desenha cada espelho em etapas. Primeiro os objetos em meia resolução, em RGBA16F (4 imagens de uma vez); depois a luz (2 e depois 1); por fim a imagem final em R11G11B10. A câmera só muda a imagem final, onde não há objetos, então não funciona nesse modo. Também os ícones do menu sumiam: o menu desenha em RGBA16F, e o plugin os tomava por espelhos. Correções: espelho de novo só em R11G11B10, mas em qualquer resolução; com o jogo pausado, nada é escondido e o F2 não é apertado.
- [ ] Ícones do menu aparecem com a ré engatada?
- [ ] Resolução maior dos retrovisores (com qualidade normal): a câmera funciona?

### 3e. Qualidade alta dos retrovisores (2026-11-14, 20:30)

O dono escolheu suportar a qualidade alta (opção B). Cada retrovisor grande é desenhado num grupo: primeiro as camadas (RGBA16F, meia resolução, 4 imagens de uma vez, onde os objetos são desenhados), depois a luz (2 imagens, depois 1), por fim a imagem final em R11G11B10. O plugin reconhece o grupo pela ordem no quadro e usa o primeiro grupo de retrovisor grande como o nosso (OUR_GROUP). A câmera é feita nas três etapas, e a imagem final segue como antes (troca de quadros, vidro devolvido, conferência do espelho). A cada nova etapa, o bloco de peça do caminhão deixa de valer, para um desenho de luz de tela cheia não ser escondido por engano.
Risco conhecido: a luz usa dados que o plugin não mexe (projeção inversa, direção do sol vista do espelho), então a iluminação da câmera pode sair diferente.
- [ ] Qualidade alta: a câmera mostra a traseira? A luz fica razoável?
- [ ] Qualidade normal: tudo continua como antes?

Resultado 20:29: na qualidade alta a câmera ficava preta. As etapas de luz usam blocos de parâmetros de tela, como [[0,0,1,1],[1,-0.5,0.5,x],[1,1,1,1],[128,256,1,1]], e um Draw 3 de tela cheia. A regra "deixar de fora o que não dá para mover" cortava esses desenhos. Agora só ficam de fora os blocos com a posição uma linha abaixo (linhas 1 a 3 formando uma rotação), que eram os da barra.

Resultado 20:40: a qualidade alta funcionou. A câmera dava pulinhos ao girar muito o volante na ré: as medidas do espelho eram guardadas pela direção dele no mundo, e o caminhão girando mais de 5° fazia o espelho parecer outro. Agora a direção é tomada em relação ao caminhão.
- [ ] Girando todo o volante na ré, a câmera fica firme?

Resultado 20:45: ao girar muito o volante, a câmera dava flashes para a frente. A direção das casas, [0,116, 0, 0,993], passava no filtro de 20° quando o caminhão ficava alinhado com a rua (direção perto de 0). Agora a direção do espelho só vale se as peças do caminhão a confirmam: a direção de cada objeto vezes os eixos do caminhão (TRUCK_TURNS) precisa bater, dentro de 0,04, com pelo menos 3 objetos.
Na qualidade alta, árvores e postes ao fundo apareciam esticados para baixo a partir do horizonte ("aves flutuando"). Hipótese: a luz e a névoa são calculadas com a lente do espelho. Teste instalado: na qualidade alta, a câmera usa a lente do próprio espelho, sem abrir o campo de visão.
- [ ] Qualidade alta: as "aves" somem? (a imagem fica mais fechada nesse teste)
- [ ] Girando muito o volante: sem flashes?

Resultado 20:55: a câmera não seguia a inclinação, ficava nivelada numa rampa. Agora a posição, a direção e o "para cima" da câmera do caminhão usam os eixos inclinados dele (rumo, pitch e roll). O sinal do pitch e do roll vem do jeito que as peças do caminhão casaram (WAYS/WAY). A posição do espelho continua nos eixos nivelados em que foi medida. A câmera da carreta segue nivelada por enquanto, porque a telemetria da carreta não tem pitch e roll.
- [ ] Numa rampa, a câmera inclina com o caminhão?

Resultado 21:00: dono aprovou a câmera do caminhão na rampa ("excelente"), sem as aves e sem os flashes. Agora a câmera da carreta também inclina: a telemetria da carreta passa a guardar pitch e roll (trailer.world.placement já os trazia), e o sentido de cada um segue o WAY do caminhão.
- [ ] Com carreta, numa rampa: a câmera da carreta inclina junto?

Resultado 21:05 (MAN do frete rápido): a câmera pulava para cima e para baixo no mesmo lugar, com a altura alternando entre 1,16 e 1,06 e o olhar em y entre −0,353 e −0,396. Com o caminhão quase plano, as quatro combinações de sinal do pitch e do roll quase empatavam, e a vencedora trocava a cada quadro. Agora a contagem é acumulada no tempo (WAY_TOTALS), só com quadros de vencedor claro (pelo menos 5 e o dobro do segundo), e o sentido mais contado fica.

Resultado 21:10: ao descer do caminhão logo depois da ré, um espelho virtual ficou ligado no modo personagem, com a imagem de teste "far". O primeiro F2 saiu às 211 s e a descida às 212 s; o segundo F2 saiu às 213 s, já andando, quando o jogo não recebe teclas. Agora, se houver espelhos virtuais ligados, o F8 pede para desligá-los primeiro (virtual_toggle::turn_off_first) e a descida espera até 600 quadros (MIRRORS_ON no character.rs). F8 de novo cancela.
- [ ] Dar ré e descer logo em seguida: não aparece espelho virtual no modo personagem? A descida demora só 1 ou 2 segundos?

Resultado 21:15 (MAN):
- O F2 se perdia: o ciclo do MAN não é o do Scania (nenhum → um → dois → um). Agora o F2 não supõe ciclo nenhum: na cabine, conta os espelhos, aperta, espera 150 quadros e conta de novo até chegar no que quer (2 na ré, 0 fora dela). São no máximo 6 apertos, e nada é apertado fora da cabine.
- A descida esperou, mas só 0,9 s depois do último aperto, antes de o jogo aplicá-lo. Agora mirrors_on vale enquanto a contagem não for 0 ou o último aperto não tiver assentado.
- Os ícones das mensagens do jogo sumiam, escondidos como moldura. Agora, se um quadrado escondido não é seguido da imagem de um espelho, a posição dele (ordem no passe e ordem entre os quadrados lisos) fica marcada como "não é moldura" por 2 s.
- [ ] MAN: ré e saída da ré com 2 apertos cada, sem ficar apertando?
- [ ] Descer logo após a ré: sem "far"?
- [ ] Mensagem do freio de mão com o ícone da tecla?

Resultado 21:30 (DAF XD com carreta, qualidade alta): a câmera da carreta ficou torta e velha. O DAF XD tem dois espelhos pequenos em pé (128x256) desenhados à parte, sem as etapas, e um deles foi tomado como o nosso. A direção nunca passou no filtro, e 375 de 375 imagens foram descartadas. Agora só conta como nosso o maior retrovisor em pé do quadro e, na qualidade alta, só os desenhados em etapas.
F8: um segundo F8 cancelava a espera pelos espelhos, então era preciso apertar 3 ou 4 vezes. Agora ele é ignorado nessa espera. O aperto de F2 também assenta assim que a contagem muda (a partir de 20 quadros), sem esperar os 150.
Ainda pendente: a figura de teste "far" no modo andando. A sonda interface-andando.txt já está instalada, mas ainda não rodou.

Resultado 21:35 (DAF XD numa rampa): nenhuma direção era aceita (taken 0, 0 peças medidas). A confirmação pelas peças usava só um sentido de pitch e roll, e esse sentido só é descoberto depois de haver uma direção aceita: as duas regras ficavam esperando uma pela outra. Agora a confirmação testa os quatro sentidos, cada chave contada uma vez por bloco.

Resultado 21:41:
- A câmera do DAF XD continuava congelada: o retrovisor dele aponta 26° para fora da traseira e o filtro era de 20°. Como a confirmação pelas peças do caminhão já barra as casas, o filtro foi para 45°.
- O "far" no modo andando aparece em interface-andando-3.txt, logo depois do Draw 3: [liso 1x1][64x128 BC3 (formato 78)] uma vez para cada espelho. Andando, essa figura conta como espelho virtual e é escondida junto com a moldura (RUN também andando). As posições lembradas da cabine não valem andando.
- F8 duas vezes: a espera dos espelhos consumia a espera da câmera 0, e o plugin desistia. Agora, quando os espelhos desligam, a câmera 0 é pedida e a espera dela começa do zero.

Resultado 21:50 (DAF): na ré, a câmera inclinava de leve de vez em quando e voltava. rolled_as copiava o giro exato do espelho, e o esquerdo e o direito têm inclinações próprias um pouco diferentes; quando o jogo trocava o espelho, a imagem pendia. Agora o giro é copiado só em passos de 90 graus.

Resultado 22:00: coisas atrás da câmera sumiam. Dois motivos:
1. Esconder as peças do caminhão pegava tudo o que estava girado igual a ele a menos de 12 m: muro, cerca, carreta ou caminhão parado em paralelo. Agora só some o que estiver dentro da caixa do caminhão, calculada a partir da origem dele vista do espelho no quadro anterior (ORIGIN_SEEN), nos eixos dele: até 1,7 m para cada lado, de −1,5 a 4,6 m de altura, e de 3,5 m à frente até rear_z + 0,5 atrás.
2. Limitação: o jogo decide no processador o que desenhar em cada espelho, pelo que esse espelho vê. O que só a câmera veria (rente ao para-choque, escondido pela carroceria vista do espelho) pode não ser desenhado.
- [ ] Muro ou carreta paralela atrás: aparecem na câmera? A barra do Scania mod continua sumida?
