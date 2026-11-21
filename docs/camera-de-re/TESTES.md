# Câmera de ré e sensor de estacionamento: relatório de teste (para o dono)

Instalado no PC em 2026-11-13 e **ainda não testado dentro do jogo**. Testar com o **app WestPlay aberto** e com o
**jogo reiniciado**: app e plugin mudaram juntos, e a ponte entre eles agora é a **versão 38**. Com um jogo que já
estava aberto antes, o app mostra "jogo não visto" até reiniciar. Marque cada item e anote o que achou. Nada foi
para o GitHub.

Liga e desliga: tela do caminhão › **Ajustes › Câmera de Ré**, com três chaves: **Mostrar ao Engatar a Ré**,
**Bipe dos Sensores** e **Linhas Guia**. Todas vêm ligadas. Se algo pesar ou atrapalhar, desligue ali.

Os logs ficam em `Documentos\WestPlay\logs`: o `plugin.log` (do jogo) e o do app.

## Como funciona (resumo)

- Ao engatar a ré, a tela do WestPlay no painel vira a câmera de ré. Ela some 1,2 s depois de tirar a ré, ou na hora
  se o caminhão andar para frente.
- **A imagem não é a do jogo**: o WestPlay desenha a vista para trás a partir do mapa do perfil (ruas, prédios em 3D,
  caminhões de outros jogadores do TruckersMP), espelhada como numa câmera de verdade. Sem mapa do perfil (mod de
  mapa protegido), aparece uma grade no chão.
- Com carreta engatada, a câmera fica na traseira da carreta; sem carreta, na traseira do caminhão.
- **Linhas guia** no chão (vermelho até 1 m, amarelo até 2 m, verde até 3 m) curvam com o volante. Com carreta, a
  carreta vai para o lado oposto, como na vida real.
- **Sensores**: bipe lento a partir de 3 m, mais rápido e mais alto ao chegar perto, tom contínuo a 0,35 m ("PARE").
  Eles só enxergam os **prédios do mapa** e os **caminhões de outros jogadores no TruckersMP**. Não enxergam o
  trânsito da IA, cercas, docas nem carretas paradas.

## 1. O plugin leu o caminhão e a carreta (olhar o `plugin.log`)

- [ ] Ao entrar no caminhão aparece `reversing: truck laid out as ...` com números que fazem sentido: entre-eixos de
      3 a 6 m e meia largura perto de 1,2 a 1,3 m.
- [ ] Aparece `telemetry: the steering of N wheels registered`, com N maior que 0.
- [ ] Ao engatar uma carreta aparece `trailer: its coupling pin at ...` e logo depois `reversing: trailer laid out as
      ...`, com o comprimento da carreta.
- [ ] Repetir em um caminhão **4x2** e em um **6x4**, com e sem carreta, no **ETS2** e no **ATS**.
- [ ] Se alguma dessas linhas não aparecer, mande o `plugin.log`.

## 2. A câmera aparece e some na hora certa

- [ ] Engatar a ré (parado ou andando devagar): a tela vira a câmera na hora.
- [ ] **Câmbio automático, parado, com a ré selecionada e sem acelerar**: a câmera aparece e **fica** aberta. Soltar
      o acelerador não fecha. (Corrigido em 2026-11-13, depois do primeiro teste: antes, ela só abria acelerando e
      fechava ao soltar. A causa era olhar a marcha do motor, que fica em neutro parado, e não a do painel.)
- [ ] O selo no alto diz **Câmera de Ré · Carreta** com carreta e **Caminhão** sem.
- [ ] Tirar a ré e deixar em neutro: a câmera some depois de um instante (cerca de 1 s).
- [ ] Ré › neutro › ré rápido: a câmera não pisca.
- [ ] Engatar a 1ª e acelerar: a câmera some logo.
- [ ] Desligar a parte elétrica: a câmera some.
- [ ] Ao voltar, o mapa do painel continua de onde estava.

## 3. Linhas guia (calibração)

- [ ] **Sem carreta**, volante para a esquerda, dando ré: as linhas curvam para a **esquerda da tela** e o caminhão
      vai mesmo para onde elas mostram.
- [ ] **Com carreta**, volante para a esquerda: as linhas curvam para a **direita da tela** e a carreta vai para lá.
      Com o volante todo virado, as linhas param antes do "canivete".
- [ ] Se as linhas curvarem para o lado errado, anote: o sinal do esterço (`wheel_steering`) está invertido.
- [ ] Com o volante reto, as linhas apontam reto para trás (na linha tracejada).
- [ ] A distância das marcas (1, 2 e 3 m) parece certa no chão do jogo.

## 4. Sensores e bipes

- [ ] Dar ré devagar contra um **prédio** (de uma cidade, não de uma empresa): começa a bipar a ~3 m, bipa mais rápido
      e mais alto ao chegar perto, e dá tom contínuo e **PARE** encostado.
- [ ] O **PARE** bate com o para-choque de verdade. Se parar longe ou bater antes, anote quantos metros erra: é o
      balanço traseiro (`rear_overhang`) a ajustar.
- [ ] O bipe sai nas caixas do PC (o som da cabine não está mudo).
- [ ] O botão de alto-falante no painel silencia até a próxima ré.
- [ ] **Ajustes › Câmera de Ré › Bipe dos Sensores** desligado: nenhum bipe.
- [ ] No TruckersMP, dando ré perto do caminhão de outro jogador, os sensores o veem.
- [ ] Docas, cercas e carretas paradas **não** são vistas (é a limitação conhecida). Anote se isso atrapalha muito.

## 5. Imagem e desempenho

- [ ] A imagem parece uma câmera de ré: chão, ruas e prédios atrás do caminhão, com os lados certos (o que está à
      sua esquerda aparece à esquerda da tela).
- [ ] Altura e inclinação da câmera estão boas. Na prévia, quase não aparece céu, porque a câmera olha bem para baixo.
      Anote se quer ver mais longe.
- [ ] Fps do jogo com a câmera aberta numa **cidade grande**, comparado com o mapa normal na mesma cidade. Anote se
      cair muito.
- [ ] À noite fica com o visual escuro.
- [ ] Sem mapa do perfil (Eldorado), aparece a grade no chão e as linhas guia funcionam.

## Primeiro teste do dono no jogo (2026-11-13, ETS2, DAF, Mapa Eldorado)

- O `plugin.log` leu o caminhão (entre-eixos de ~4 m, meia largura de 1,25 m), a carreta e a direção de 14 rodas.
- A câmera mostrou só o chão verde com as linhas guia. É o esperado com o Eldorado: é um mapa protegido, e o
  WestPlay não lê ruas nem prédios dele. A grade do chão quase não aparece na tela pequena.
- A câmera só abria acelerando e fechava ao soltar o acelerador, mesmo na ré. **Corrigido** (veja a seção 2).
- Nenhum bipe, nem dando ré contra um caminhão amarelo que faz parte do cenário do mapa. Também é o esperado: no
  Eldorado o WestPlay não conhece nenhum prédio, e o jogo não passa aos plugins onde estão os modelos do cenário, o
  trânsito ou as carretas. Bipe "de verdade" em qualquer coisa exige a investigação da física do jogo (PLANO.md,
  Parte 3, item 5). O som do bipe em si ainda não foi ouvido no jogo (seção 4).

## 6. Sensores pela física do jogo (novo, 2026-11-13, ponte 39)

Agora os sensores também perguntam à física do próprio jogo o que há atrás, ou seja, o mesmo com que o caminhão
bate: muros, postes, o cenário do mapa, o trânsito e as carretas. Funciona em qualquer mapa, inclusive no Eldorado.
É o primeiro teste disso: **num save de teste, no single player, sem TruckersMP** (no TMP fica desligado).

**Segurança:** da outra vez que o WestPlay usou a física do jogo (Modo Personagem), o jogo fechou. Agora há uma
trava. Na primeira vez, o arquivo `DocumentosWestPlayets2ays.txt` (no ATS, `DocumentosWestPlayays.txt`) diz
"testing". Depois de uns 20 s de ré com os raios funcionando, ou ao fechar o jogo normalmente, ele passa a dizer "ok".
Se o jogo fechar no meio (crash), na próxima vez os raios **ficam desligados sozinhos** e o arquivo diz "off". Para tentar de
novo, apague o arquivo.

- [ ] Ao entrar no caminhão, o `plugin.log` tem `ray: found the game's physics ray at +0x1635990 (scene at +0x178,
      raycast slot 0x2f0)` (no ATS, `+0x16967e0`).
- [ ] Rodando, a cada 30 s: `ray: the game cast N rays in 30 s`. **N maior que 0 é o que importa**: os raios do
      WestPlay só saem junto com os do jogo. Se N for sempre 0, mande o log: o jogo não lança raios nessa hora e eu
      procuro outro momento seguro.
- [ ] Logo depois: `ray: world 0x...: the ground X m under the truck's placement`, com X entre -1 e 3 mais ou menos.
- [ ] Engatar a ré perto de um muro, poste, caminhão do cenário ou carreta: a cada segundo, `ray: ... rear sensors
      [a, b, c, d]`. Os números (metros) diminuem ao chegar perto, e -1 quer dizer nada a menos de 2 ou 3 m.
- [ ] E **apita**: lento a 3 m, mais rápido perto, contínuo encostado, com a distância e as faixas acesas no painel.
- [ ] O sensor certo acende: um poste à direita acende a direita do desenho.
- [ ] Chão, rampa e meio-fio baixo **não** apitam. Se apitar no nada, anote onde (pode ser uma camada do jogo que não é
      sólida).
- [ ] Depois de uns 20 s de ré, o `plugin.log` diz `the guard says ok` e o `rays.txt` diz "ok".
- [ ] **O jogo não fecha.** Se fechar, anote o que estava fazendo e mande o `plugin.log` anterior
      (`plugin.anterior.log`).
- [ ] Desempenho: os fps não mudam ao dar ré.

**1º teste (2026-11-13, 20:00, ETS2):** o jogo **não fechou**. Ele lançou raios próprios (cerca de 20 por segundo
andando, nenhum parado), e os do WestPlay saíram junto. Mas **não apitou**: o resultado do PhysX era lido 4 bytes fora do
lugar. O raio para baixo achou o chão, mas a leitura foi errada, e o mundo foi descartado. Corrigido (o layout do MSVC),
junto com:
- a última leitura é mantida enquanto o caminhão está parado em ré (o jogo não lança raios parado);
- fechar o jogo normalmente durante o teste agora deixa a trava em "ok".

Reinstalado às 20:15 (só a DLL). A trava foi zerada.

**2º teste (2026-11-13, 20:20):** tudo funcionou, os sensores apitam certo e a trava foi para "ok" sozinha. Com a
carreta (colhedeira numa prancha), o aviso vinha tarde: o sensor começava no fim **estimado** da carreta, e a
traseira de verdade (com a carga) ia mais para trás. Agora o fim da carreta é **medido pela física**: ao engatar,
raios de trás para a frente, só contra carretas, em 5 alturas. O `plugin.log` diz
`ray: the trailer's rear measured +X m from the guess` (X positivo: a carreta é mais comprida que a estimativa).

- [ ] Engatado na carreta, andar um pouco: aparece a linha `the trailer's rear measured ...`.
- [ ] Dando ré com a carreta, o aviso começa a ~3 m da traseira de verdade (com a mesma antecedência do caminhão sozinho).
- [ ] A câmera e as linhas na tela também começam na traseira medida.
- [ ] Trocar de carreta: mede de novo.

## Já conferido fora do jogo (2026-11-13)

Numa prévia no navegador (`cabin.html?reverse`, uma carreta simulada dando ré contra uma parede falsa), conferi:

- a imagem aparece e está espelhada certo;
- as linhas curvam com o volante, para lados opostos com e sem carreta;
- os sensores acendem e a distância desce até parar;
- a página dos Ajustes liga e desliga as três chaves e guarda a escolha.

Nessa conferência apareceu e foi corrigido um defeito da Parte 1: a imagem ficava preta, porque o MapLibre montava a
câmera sem o "roll".

## Espelhos virtuais por lado e trava do sensor (2026-11-18, ainda não testado no jogo)

O que mudou (plugin e app juntos: a ponte agora é a **versão 42**, então reinicie o jogo e o app):

- **Escolha dos espelhos virtuais:** celular › **Ajustes › Espelhos virtuais**: Nenhum, Esquerdo, Direito, Os dois. Vem
  em **Direito**.
- Com qualquer escolha menos "Nenhum", o plugin deixa os **dois** virtuais ligados o tempo todo na cabine (aperta F2
  sozinho ao entrar) e só esconde o que não foi escolhido. Com "Nenhum", liga só na ré, como antes.
- **A câmera de ré vem sempre do espelho do lado do motorista** (o esquerdo nos caminhões de volante à esquerda). Antes
  ela pegava o que o jogo desenhasse mais, e às vezes trocava de lado.
- **Trava do sensor:** se o jogo fechar durante o teste da física, o plugin tenta de novo na próxima vez. Só desliga
  depois de **3 fechamentos seguidos**, e uma versão nova do plugin tenta de novo sozinha.

### 1. Escolha dos espelhos

- [ ] Celular › Ajustes mostra **Espelhos virtuais** com as quatro opções, marcado **Direito**.
- [ ] Entrando na cabine, o F2 é apertado sozinho (no máximo umas 2 ou 3 vezes) e aparece **só o espelho virtual
      direito**, com a moldura dele.
- [ ] **Esquerdo:** aparece só o esquerdo. **Os dois:** aparecem os dois. **Nenhum:** nenhum, e fora da ré o plugin
      desliga os virtuais.
- [ ] O espelho que aparece mostra a vista normal dele (não a câmera de ré), também durante a ré.
- [ ] Nenhuma moldura preta sozinha na tela (moldura de espelho escondido) e nenhuma moldura faltando no que aparece.
      Se piscar, anote quando (olhando em volta, zoom, ré).
- [ ] Descer do caminhão (F8) com eles ligados: desce normal, sem espelho nem imagem de teste "far" na tela.
- [ ] Voltar para a cabine: os virtuais ligam de novo sozinhos.

### 2. Câmera de ré do lado do motorista

- [ ] Engatar a ré: a câmera aparece e fica estável, sem pular entre dois pontos de vista.
- [ ] No `plugin.log` aparece `our mirror's view is of the left one (the driver's: left)`.
- [ ] Olhar em volta e dar zoom na tela durante a ré: a câmera continua. Se ela congelar um instante e voltar, procure
      no log `our mirror is now ... (the one before held the other side's view)`.

### 3. Sensor no Eldorado

- [ ] Com o jogo **salvo**, engatar a ré no Eldorado perto de uma parede ou carreta: o sensor apita e mostra a distância.
- [ ] No `plugin.log`: `ray: the first rays through the game's physics` e, depois de um tempo de ré,
      `ray: 120 batches cast without trouble: the guard says ok`.
- [ ] Se aparecer `ray: not found` ou `isn't as known`, a física do ETS2 1.62 mudou: mande o `plugin.log`.

### Correção de 18/11 20:45

- A escolha dos espelhos (e as chaves de Nota fiscal, Abastecimento manual e Pagar no frentista) não chegava ao app
  pelo celular: o app só deixava passar mensagens de uma lista, e elas não estavam nela. Agora estão.
- O lado de cada espelho agora é medido em cada textura, não deduzido. No plugin.log, a cada 5 s:
  `retrovisor: virtual mirrors (chosen 2, ours 0x...): 0x... left hidden, 0x... right shown` (2 = Direito, 3 =
  Esquerdo, 4 = Os dois, 1 = Nenhum). Se o que aparece na tela não bater com essa linha, mande o log.

## Cartão na troca da ré (19/11, noite; app e plugin novos, ponte 46: **reinicie o jogo**, abra o app antes)

Teste do dono: "engatando a ré, o painel dá uma travadinha: aparece o mapa do jogo, depois o espelho normal, depois a
câmera; e saindo da ré, a mesma coisa". Causa: a tela de ré abria na hora, e por baixo passavam a câmera feita do mapa,
o espelho cru (antes de a nossa câmera assumir) e só então a câmera.

Agora:
- **Entrando:** um cartão escuro cobre a tela de ré (ícone amarelo, "Câmera de ré" e uma barrinha correndo) até o
  plugin dizer que a **nossa câmera assentou** (20 quadros ligada, cerca de 0,3 s). Aí some suave (0,2 s). No máximo
  2,5 s: num caminhão sem espelho, aparece a câmera do mapa depois disso.
- **Saindo:** o mesmo cartão cobre 0,45 s enquanto o plugin desliga a câmera e a tela volta ao normal.

- [ ] Engatar a ré: só o cartão e depois a câmera, **sem** mapa nem espelho cru no meio?
- [ ] Sair da ré: o cartão um instante e a tela normal, sem o espelho aparecendo?
- [ ] O cartão demora demais (parece travado)? Dá para ajustar os 20 quadros ou os 2,5 s.

### Ajuste do cartão (19/11, 22h; vai instalado junto com o volante, ponte 47)
O cartão saía cedo: pelo log, a câmera ligava às 63,97 s, mas o F2 ainda apertava de novo às 64,24 s e o nosso
espelho trocava 5 vezes até 64,97 s. Agora "pronta" = 20 quadros ligada **e** o F2 terminou (os dois virtuais ligados)
**e** o nosso espelho ficou o mesmo por 30 quadros (cerca de 0,5 s). O limite foi para 3,5 s. Na **saída**, se o
espelho some (o F2 desliga na hora) enquanto a tela ainda espera um instante pela ré, o cartão cobre, e não aparece
mais a câmera do mapa.
- [ ] Entrando: nada de espelho trocando depois que o cartão some?
- [ ] Saindo para a frente: nada de mapa, só o cartão e a tela normal?

### O painel do jogo piscando ao engatar para frente (19/11, 23h40; vai com a próxima instalação do volante)
Teste do dono: "a ré funciona; engatando para frente, aparece o mapa original do jogo bem rápido, depois o cartão
preto". Causa: quando o app recria a imagem da tela (uma "geração" nova), o plugin ficava sem quadro um instante e
deixava a nossa tela transparente, e o painel do jogo aparecia por baixo. Agora, com a tela ligada, ele mantém a
última imagem por até 60 quadros (cerca de 1 s) até a nova chegar.
- [ ] Engatar para frente saindo da ré: o painel original do jogo **não** pisca mais?
- [ ] Desligar a ignição: a tela ainda apaga normalmente (isso não mudou)?

## Câmera dentro da carreta (2026-11-20, plugin)

Com uma carreta aleatória a câmera ficou dentro dela. O log mostrou o motivo: os raios dos sensores mediram a
traseira dessa carreta 0,99 m antes da conta feita pelos eixos (6,47 m contra 7,45 m). A "caixa de colisão" do
jogo para essa carreta acaba antes da carroceria, e a câmera ia para onde os raios mediram. Agora a câmera
nunca fica antes da traseira calculada pelos eixos: fica 0,3 m depois dela (ou onde os raios mediram, se for mais
para trás). Os sensores de ré continuam usando a medida dos raios.

- [ ] Reiniciar o jogo (o plugin trocou). Com a mesma carreta: a câmera fica atrás dela, sem ver o "lado de dentro".
- [ ] Com as carretas de antes: a câmera não ficou longe demais da traseira? (Se ficou, anote a carreta.)

Voltar atrás: `westplay.dll.antes-camera-carreta` no MaxDevMod e nos jogos.

## Ajustar a câmera você mesmo (2026-11-20, app + plugin, ponte 50)

Na tela da câmera de ré (engatando a ré), ao lado do botão de som tem um botão novo com setas: **Ajustar a câmera**.
Ele troca o desenho dos sensores por botões: **Mais para trás**, **Mais para frente**, **Subir**, **Descer** (0,25 m
por toque), **Padrão** (volta para onde ela vai sozinha) e **Pronto**. Embaixo de cada par mostra quanto já foi
movido (ex.: "+0,50 m"). Fica guardado **para aquela carreta** (ou para o caminhão sozinho, pelo modelo) no arquivo
`Documents\WestPlay\<jogo>\camera-de-re.txt`. Lembre: para clicar na tela da cabine, segure a tecla de interação.

- [ ] Reiniciar o jogo (ponte 50). Engatar a ré: o botão de setas aparece ao lado do som.
- [ ] Tocar: aparecem os botões; "Mais para trás" leva a câmera para trás na hora, e o número muda.
- [ ] Subir/Descer funcionam; "Padrão" volta tudo para 0.
- [ ] Sair da ré e engatar de novo: o ajuste continua.
- [ ] Trocar de carreta: volta ao padrão; voltar para a primeira carreta: o ajuste dela volta.
- [ ] Caminhão sem carreta: o ajuste é separado do da carreta.

Voltar atrás: arquivos `.antes-ajuste-camera` (exe, dll, ui) no MaxDevMod e nos jogos.

### Também pelo celular (2026-11-20, só o app)

O mesmo ajuste está no **celular › Caminhão**, no fim da página: **Câmera de ré**, com Distância (Para frente /
Para trás), Altura (Descer / Subir) e **Voltar ao padrão**. Funciona no celular do jogo e no seu telefone de verdade
(a página pelo QR Code). Vale para a carreta engatada agora (ou o caminhão sozinho) e é o mesmo ajuste da tela da
cabine: mudou num, muda no outro.

- [ ] Celular do jogo › Caminhão: aparece "Câmera de ré" com os números.
- [ ] De ré, olhando a tela da cabine: "Para trás" no celular move a câmera, e o número muda nos dois lugares.
- [ ] No seu telefone de verdade: os mesmos botões funcionam.

Voltar atrás: `westplay.exe.antes-camera-celular` e `ui.antes-camera-celular` no MaxDevMod.

### Agora num app só dele: Câmera de Ré (2026-11-20, só o app)

O dono não achou o ajuste no fim da página Caminhão. Agora ele tem **app próprio no celular: Câmera de Ré**, na
**2ª página** da tela inicial (ao lado da Suspensão). Setas grandes: **Subir**, **Descer**, **Para frente**, **Para trás**
(0,25 m por toque), no meio quanto já moveu, em cima se vale para a carreta engatada ou o caminhão sozinho, e embaixo
**Voltar ao padrão**. O ajuste saiu da página Caminhão (ficou só no app). Com o jogo fechado, o app diz que o jogo não
está aberto.

- [ ] Celular do jogo, 2ª página: aparece o ícone **Câmera de Ré** (câmera com a seta amarela).
- [ ] Abrindo com o jogo rodando: as setas e os números aparecem; diz "carreta engatada" ou "caminhão sozinho".
- [ ] De ré, olhando a tela da cabine: cada seta move a câmera e o número muda (no app e na tela da cabine).
- [ ] Voltar ao padrão zera os dois números.
- [ ] No seu telefone de verdade (QR Code): o mesmo app funciona.

Voltar atrás: `westplay.exe.antes-app-camera` e `ui.antes-app-camera` no MaxDevMod.

### Girar, inclinar, campo de visão e passos menores (2026-11-20, app + plugin, ponte 51: **reinicie o jogo**)

Pedido do dono: girar a câmera para cima/baixo e para os lados, aumentar o campo de visão, e cada toque mover menos
("25 é muito, uns 5"). No app **Câmera de Ré** do celular, abaixo das setas, a parte **Girar e campo de visão**:
**Inclinação** (Para baixo / Para cima, 1° por toque, até 30°), **Girar** (Esquerda / Direita, 2° por toque, até 45°) e
**Campo de visão** (Menos / Mais, 5° por toque; o normal é 120°, vai de 60° a 160°). As setas de distância e altura agora
andam **5 cm** por toque (também na tela da cabine). "Voltar ao padrão" zera tudo. Fica guardado por carreta, como antes;
o arquivo `camera-de-re.txt` antigo continua valendo.

- [ ] Reiniciar o jogo (ponte 51). Distância e altura andam 5 cm por toque ("+0,05 m").
- [ ] Inclinação Para cima: a imagem mostra mais longe (menos chão). Para baixo: mais chão.
- [ ] Girar Direita: a imagem vai para o **lado direito da tela**. Se for para o lado contrário, me avise que eu inverto.
- [ ] Campo de visão Mais: aparece mais coisa (imagem mais "aberta"); Menos: mais "zoom".
- [ ] Voltar ao padrão: tudo volta a 0.

Voltar atrás: `.antes-girar-camera` (exe, dll, ui) no MaxDevMod e `westplay.dll.antes-girar-camera` nos jogos.

## Barra de rolagem no celular (2026-11-20, só o app)

Pedido do dono: em página cheia de botões não dava para arrastar (os botões de segurar, como os Comandos e a
Suspensão, pegam o dedo de propósito). Agora toda página do celular que não cabe na tela tem uma **barra na borda
direita**: arraste para subir e descer; um toque acima ou abaixo dela pula uma tela.

- [ ] No seu telefone (QR Code), Comandos: a barra aparece na direita e arrastar ela rola a página.
- [ ] Tocar abaixo da barra desce uma tela; acima, sobe.
- [ ] Em página que cabe inteira, a barra não aparece.

Voltar atrás: `westplay.exe.antes-barra-rolagem` e `ui.antes-barra-rolagem` no MaxDevMod.
