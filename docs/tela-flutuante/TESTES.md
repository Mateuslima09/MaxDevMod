# Tela flutuante: testes para o dono

Instalada em 2026-11-20 (app + plugin juntos, ponte 52). Ainda não testada no jogo. Como funciona e os riscos:
`PLANO.md`.

Antes: feche e abra o jogo de novo (o plugin novo só entra com o jogo reiniciado) e abra o MaxDevMod.

Para voltar ao de antes: as cópias `.antes-tela-flutuante` ao lado do `westplay.exe` e do `westplay.dll`
(`%LOCALAPPDATA%\MaxDevMod` e a pasta `bin\win_x64\plugins` de cada jogo), e a pasta `ui.antes-tela-flutuante`.

## Resultado do 1º teste (2026-11-20, 12:05) e correção

Em três caminhões a tela não apareceu, e o app dizia "Não aparece". O log mostrou o motivo: o plugin lia só os
primeiros 600 desenhos de cada quadro, e a cabine é desenhada depois deles. Ele achou a cabine só em poucos
quadros (no máximo 37 em 10 s), então a tela no máximo piscou.

Correção (plugin só, instalado às ~12:15): ele procura a cabine em janelas de 600 desenhos que andam pelo quadro
inteiro. Depois de achar, lê só perto de onde a cabine apareceu. Para voltar: `westplay.dll.antes-tela-janela`.

- [ ] **Feche e abra o jogo** (o jogo aberto ainda usa o plugin antigo).
- [ ] Na cabine, com a tela ligada, o app diz **Aparecendo** e a tela aparece, sem piscar?
- [ ] Onde ela aparece? Se for num lugar estranho, mande um print e o `plugin.log` (a linha `screen seen at` diz
      onde ela está em relação à câmera).
- [ ] Se você mexeu nela antes, toque em **Voltar ao lugar padrão** primeiro.

## Resultado do 2º teste (2026-11-20, 12:30) e sonda

Continuou sem aparecer. O log mostrou que, na vista normal da cabine, nenhum desenho do jogo bate com a cabeça do
motorista: o mais perto fica a 3 m. Nas poucas vezes em que "achou", era outra vista do jogo (a tela teria ficado
atrás da câmera). Ou seja, o jogo entrega a cabine de um jeito diferente do que o plugin espera.

Sonda instalada (plugin só, ~12:40). Ela grava o que o jogo manda em um quadro inteiro. Com a tela ligada, faz isso
a cada 20 s, no máximo 4 vezes por jogo, e o jogo dá uma travadinha em cada uma. Para voltar:
`westplay.dll.antes-tela-sonda`.

- [ ] Feche e abra o jogo. Entre num caminhão, câmera de dentro da cabine, e deixe a tela **Ligada** no app Tela.
- [ ] Fique parado olhando para frente uns **90 segundos** (4 travadinhas).
- [ ] Avise. Eu leio `DocumentosWestPlaylogs	ela-flutuante-1.txt` até `-4.txt` e o `plugin.log`.

## Resultado da sonda (2026-11-20, 12:19) e correção

A sonda mostrou o motivo: o jogo desenha o interior da cabine com o centro **na cabeça do motorista**, e não no centro
do caminhão, como o plugin achava. As peças do painel ficam a 1 ou 2 cm da câmera, e o volante a 66 cm à frente,
inclinado. O plugin agora reconhece a cabine assim e põe a tela nesse espaço. A sonda foi desligada (sem mais
travadinhas). Plugin só, instalado às ~12:45. Para voltar: `westplay.dll.antes-tela-cabeca`.

- [ ] Abra o jogo, câmera de dentro, tela **Ligada**. Antes, toque em **Voltar ao lugar padrão**.
- [ ] Ela aparece um pouco à direita do volante, na altura do painel, virada para você? O app diz **Aparecendo**?
- [ ] Olhe em volta: ela fica presa na cabine?
- [ ] Se aparecer num lugar estranho, mande um print e o `plugin.log` (linha `screen seen at`).

## Resultado do 3º teste (2026-11-20, 13:05) e correções

**Apareceu e funcionou** (Actros). Problemas que você achou:
1. Olhando para o banco do passageiro, a tela sumia. O log mostrou que, olhando para o lado, o jogo desenha poucas
   peças da cabine (às vezes uma só), e o plugin pedia pelo menos 6.
2. Mexendo na posição da câmera (descer o banco, ir para trás), ela sumia: a câmera saía dos 15 cm aceitos.
3. A moldura estava quadrada.
4. Faltava o logo.

Correções (plugin só, instalado às ~13:20; para voltar: `westplay.dll.antes-tela-moldura`):
- O plugin agora **segue** a cabine de um quadro para o outro: uma peça no mesmo lugar e no mesmo giro do quadro
  anterior conta, mesmo sendo uma só, e mesmo com a câmera até 80 cm longe da cabeça.
- Cantos arredondados na moldura e na imagem, com uma bordinha mais clara.
- Uma faixa embaixo da tela com o logo **MaxDevMod**.

- [ ] Olhe para o banco do passageiro (e devagar de volta): a tela continua lá?
- [ ] Mexa na câmera (banco para baixo, para trás): ela continua?
- [ ] Moldura arredondada e o logo embaixo ficaram bons? (Tamanho do logo, cor da borda: diga o que mudar.)
- [ ] Virando a cabeça **muito rápido** ela pode piscar um instante. Acontece?

## Resultado do 4º teste (2026-11-20, 13:18) e correção

A tela parou de aparecer, mas o app dizia "Aparecendo". O log mostrou que o plugin "seguiu" uma peça errada: um
desenho de tela cheia do jogo, sem giro. A tela era desenhada fora de vista (ou atrás da câmera).

Correção (plugin só, ~13:30; para voltar: `westplay.dll.antes-tela-lente`): só contam peças vistas pela lente da
câmera (com perspectiva e com giro), e a cabine com mais peças vence a que só "segue".

- [ ] A tela voltou, com a moldura arredondada e o logo?
- [ ] Olhando para o banco do passageiro e mexendo no banco, ela continua?

## Resultado do 5º teste (2026-11-20, 13:24) e correção

A cabine já era achada certinho (a tela 84 cm à frente, um pouco à direita), mas a tela não aparecia. O defeito
estava na moldura nova: os números dela (tamanho, cantos, logo) não chegavam à parte do desenho que pinta os
pixels, então ela saía vazia. Agora um teste desenha a tela de verdade numa placa de vídeo simulada e confere os
pixels; a prévia está em `previa-moldura.png` (o verde é a imagem do app).

Plugin só, ~13:50; para voltar: `westplay.dll.antes-tela-pixel`.

- [ ] A tela aparece com a moldura arredondada e o logo embaixo?
- [ ] Olhando para o banco do passageiro e mexendo no banco, ela continua?

## Resultado do 6º teste (2026-11-20, 13:55) e correção

**Apareceu, com moldura e logo.** (Ela estava preta porque o caminhão estava desligado: o app apaga a tela sem
ignição.) Problemas:
1. Olhando mais para a cama, ou para o teto, ela sumia.
2. Depois de mexer no banco ou no volante, ela piscava ou sumia de vez em quando.

Motivo: o plugin se localizava só pelas peças do painel, e nessas direções o jogo quase não as desenha. Com o banco
mexido, quando ele perdia o painel, não sabia reencontrar.

Correção (plugin só, ~14:15; para voltar: `westplay.dll.antes-tela-pecas`):
- Enquanto vê o painel, o plugin **aprende onde ficam as outras peças da cabine** (cama, bancos, teto, volante...).
  Sem o painel, ele acha a cabine pelas peças aprendidas (pelo menos duas concordando).
- Ele usa a posição da cabeça que o jogo informa (muda quando você mexe no banco), para achar o painel de novo.
- Se perder a cabine num quadro, segura a tela no último lugar por até 4 quadros (sem piscar).

- [ ] Ligue o caminhão (para a tela ter imagem). Olhe para frente uns segundos (para ele aprender as peças).
- [ ] Olhe para a cama, bem para trás, e para o teto: a tela continua no lugar?
- [ ] Mexa no banco (para baixo, para trás) e no volante: ela continua, sem piscar?
- [ ] Se sumir, mande o `plugin.log` (a linha `by blocks …, by parts …, held …` diz como ela foi achada).

Risco conhecido: parado muito tempo ao lado de algo (um muro, outro caminhão), o plugin pode achar que aquilo é peça
da cabine. Se a tela aparecer no lugar errado depois disso, me avise.

## Nitidez (2026-11-20, ~14:35)

Na visão normal a tela ficava meio embaçada e com as letras tremendo; com zoom, nítida. Motivo: a imagem do app
(1280x720) é encolhida para uns 370x210 pixels. Agora o plugin guarda versões já reduzidas da imagem (mipmaps) e
encolhe do jeito certo, um pouco puxado para o nítido. Plugin só; para voltar: `westplay.dll.antes-tela-nitida`.

- [ ] Reinicie o jogo. Na visão normal, as letras ficam mais limpas e param de tremer?
- [ ] Ficou nítido demais (serrilhado) ou ainda mole? Diga qual, eu ajusto.

## A pé (2026-11-20, ~14:45)

No Modo Personagem, fora do caminhão, a tela aparecia flutuando na estrada, à frente da personagem (o plugin achava
que a câmera da personagem era a cabeça do motorista). Agora ela some enquanto você está a pé, e o plugin não aprende
peças nesse tempo. Plugin só; para voltar: `westplay.dll.antes-tela-a-pe`.

- [ ] Reinicie o jogo (o aberto às 14:40 ainda usava o plugin de antes da nitidez).
- [ ] Saia do caminhão (F8): a tela some? Volte para a cabine: ela volta no lugar?

## Resultado do 7º teste (2026-11-20, ~15:00) e correções

Problemas que você achou:
1. Olhando para trás pela janela e voltando, a tela ia parar na cama; mexendo muito a câmera, sumia.
2. Olhando para o banco ou a cama, a tela subia sozinha.
3. Em câmeras de fora, a tela aparecia flutuando no pátio e no posto.
4. Na ré, a tela mostrava só as linhas-guia, no preto.
5. No ATS, a tela tremia.
6. Com a suspensão da cabine mexendo, a tela não acompanhava.
7. A tela devia ficar desligada ao trocar de caminhão.
8. Contas e Mapa do app dão erro (STATUS_BREAKPOINT).

Correções (plugin só, ~15:20; para voltar: `westplay.dll.antes-tela-v3`):
- 1, 2, 3: o "aprender peças" saiu. Era ele que levava a tela para a cama e para fora. A tela agora só usa as peças
  do painel; quando não as vê, segura 4 quadros e some.
- Com o banco mexido: aceita a cabine mais longe da cabeça só quando muitas peças concordam.
- 5: ela gruda na mesma peça do quadro anterior (antes podia alternar entre duas peças a 1 cm, o que dava a
  tremedeira).
- 4: na ré, a tela flutuante mostra a mesma imagem montada da tela do caminhão (câmera + linhas).
- 7: trocou de caminhão, a tela desliga.
- 6: deve melhorar com as outras correções (a tela segue as peças do painel, que sobem com a cabine). Teste de novo.
- 8: não é da tela flutuante; fica para uma investigação à parte.

- [ ] Olhe para trás pela janela e volte: a tela continua no painel?
- [ ] Câmeras de fora (2, 3, 4...): nenhuma tela flutuando?
- [ ] Ré: a tela mostra a câmera de ré?
- [ ] ATS: parou de tremer?
- [ ] Suspensão da cabine subindo/descendo: a tela acompanha?
- [ ] Troque de caminhão: a tela desliga?
- [ ] Olhando para a cama, a tela some (o painel fora de vista). Isso é esperado agora.

## 8º teste: a tela sobe e desce ao mirar, e fica parada quando a cabine balança (2026-11-20)

Você contou que, virando muito a câmera, a tela sobe e desce conforme mira, e que ela fica parada enquanto a cabine
balança. Não dá para saber pelo log de antes se a peça em que a tela se prende está mesmo presa à cabine. Instalei
um registro leve (sem travadinhas): uma vez por segundo, ele anota essa peça e onde ficam as outras peças da cabine
em relação a ela. Plugin só; para voltar: `westplay.dll.antes-tela-diag`.

- [ ] Reinicie o jogo, câmera de dentro, tela ligada. Fique **10 s parado** olhando para frente.
- [ ] Depois **mire devagar** para cima, para baixo, e bem para os lados, uns 20 s (é quando ela sobe e desce).
- [ ] Depois **ande numa estrada ruim** ou faça a cabine balançar (frear forte), uns 20 s.
- [ ] Avise. Eu leio as linhas `tela flutuante: diag` do `plugin.log`.

## Resultado do diagnóstico e teste dos pontinhos (2026-11-20)

O registro mostrou que a peça onde a tela se prende **está presa à cabine** (as outras peças ficam sempre à mesma
distância dela; olhando para o lado a cabeça avança 33 a 50 cm, e a peça acompanha). Então a conta está certa e o
erro aparece na hora de desenhar: perto do meio da imagem a tela fica no lugar, perto da borda ela escorrega.
Suspeita: um efeito de imagem (do jogo, ou de algum programa ou mod de imagem) que entorta a cena depois de pronta, e a
tela, desenhada por cima, não entorta junto.

Teste (plugin só; para voltar: `westplay.dll.antes-tela-pontos`): agora aparecem **quadradinhos pequenos** (mini
telas) no centro de algumas peças da cabine, entre elas o **centro do volante** (o logo).

- [ ] Olhe reto: algum quadradinho está no centro do volante?
- [ ] Vire a câmera para o lado e para baixo (como nos prints do rádio): o quadradinho continua no centro do volante,
      ou escorrega junto com a tela?
- [ ] Mande dois prints (reto e virado). Diga também se usa ReShade ou algum programa/mod que mexe na imagem (o
      contador de FPS no canto é de qual programa?).

## Resultado dos pontinhos: era paralaxe (2026-11-20, ~17:10)

O quadradinho ficou certinho no logo do volante: o desenho bate com o jogo. O problema era o lugar da tela. No MAN
ela estava 74 cm para frente e 60 cm para baixo, uns 1,65 m dos olhos, **atrás** do painel. Olhando reto ela cobria
o rádio (mais longe, parece do tamanho certo), mas olhando para o lado a cabeça do motorista avança 33 a 50 cm e um
objeto atrás do painel se move diferente dele: ela "subia" e escorregava. É paralaxe, como mexer a cabeça e ver um
objeto perto andar diferente do fundo.

Novo (app + plugin, ~17:10; para voltar: `.antes-tela-tamanho` no exe, dll e pasta ui): botões **Tamanho**
(Maior/Menor, 5% por toque) no app Tela. Os quadradinhos de teste saíram.

Como acertar:
1. Toque em **Voltar ao lugar padrão**.
2. Use **Tamanho → Menor** até ela ter o tamanho do rádio.
3. Use **Frente/Trás** até ela **encostar no painel**. O teste: olhe para os lados; se ela ainda escorregar em relação
   ao rádio, ajuste Frente/Trás até parar de escorregar.
4. Ajuste o lado, a altura e o giro.

- [ ] Depois de acertar, olhando para os lados ela fica parada sobre o rádio?
- [ ] Com a cabine balançando, ela acompanha o painel?

## Tamanho deu certo; voltar da câmera 2 (2026-11-20, ~17:30)

Com o Tamanho, a tela encostou no painel e parou de escorregar. Problema novo: indo para a câmera 2 e voltando para
a 1, a tela demorava a voltar. Motivo: depois da câmera de fora o quadro tinha três vezes mais desenhos, e a busca
pela cabine continuava de onde a cabine estava antes, com a cabine ficando para trás. Agora, ao perder a cabine, a
busca recomeça do início do quadro, com mais folga. Plugin só; para voltar: `westplay.dll.antes-tela-janela2`.

- [ ] Câmera 2 (ou outras) e de volta para a 1: a tela volta na hora?
- [ ] O FPS ficou igual?

## Piorou com a janela maior; agora só a etapa da cabine (2026-11-20, ~17:45)

Com a busca recomeçando do início, a tela demorou ainda mais (olhando pela janela para trás, e voltando da câmera 2).
O log mostrou que a busca "grudava" num pedaço do quadro com poucas peças da cabine (menos de 6) e não chegava onde
a cabine é desenhada de verdade. Agora o plugin lê só a etapa em que o jogo pinta as superfícies da cena (a que
grava quatro imagens ao mesmo tempo), inteira, todo quadro, sem janela. Se num jogo essa etapa não existir, ele volta
ao jeito antigo sozinho (o log diz `every main-view pass read`). Plugin só; para voltar:
`westplay.dll.antes-tela-gbuffer`.

- [ ] Reinicie o jogo. Câmera 2 e volta para a 1: a tela aparece na hora?
- [ ] Olhando para trás pela janela e voltando: aparece na hora?
- [ ] FPS igual?

## A versão anterior nem chegou a funcionar (2026-11-20, ~21:15)

O log do teste mostrou que, ao abrir o jogo, no menu e no carregamento (sem cabine na tela), o plugin desistiu da
"etapa da cabine" aos 21 s e voltou ao jeito antigo para sempre. Por isso nada mudou. Agora ele decide a cada quadro:
viu a etapa no quadro anterior, lê só ela; não viu, lê do jeito antigo só naquele quadro. O log diz em cada linha de
10 s `G-buffer only true`. Plugin só; para voltar: `westplay.dll.antes-tela-gbuffer2`.

- [ ] Abra o jogo. Câmera 2 e volta para a 1; descer do caminhão e voltar; olhar para trás pela janela e voltar: a
      tela aparece na hora, sem precisar mexer a câmera?
- [ ] Se ainda falhar em algum caminhão, diga qual e mande o `plugin.log`.

## 1. Ligar

- [ ] No celular do jogo, segunda página: tem o app **Tela** (ícone vermelho de monitor)?
- [ ] Abra, toque em **Ligada**. Na câmera de dentro da cabine, aparece uma tela com o MaxDevMod à frente do painel,
      um pouco para o meio, virada para você?
- [ ] O app diz **Aparecendo**?
- [ ] Olhe em volta com o mouse: a tela fica parada no lugar da cabine (não segue o olhar como um adesivo)?
- [ ] Em movimento, numa estrada ruim: ela balança junto com a cabine, sem tremer sozinha?
- [ ] Câmera de fora (tecla 2 ou outra): a tela some? O app diz "Não aparece"?
- [ ] Pause (Esc): a tela some?
- [ ] O FPS caiu? Compare ligada e desligada.

## 2. Mover e girar

- [ ] No app Tela: **Subir / Descer / Esquerda / Direita** movem 2 cm por toque, na hora?
- [ ] **Frente / Trás** aproxima e afasta?
- [ ] **Girar**, **Inclinar** e **Tombar** giram 2° por toque, no sentido que o botão diz? (Se algum estiver ao
      contrário, me diga qual.)
- [ ] **Voltar ao lugar padrão** traz ela de volta.
- [ ] Saia do jogo e volte: ela está ligada e no mesmo lugar?
- [ ] Troque de caminhão: o outro tem o lugar dele (começa no padrão)?

## 3. Usar com o mouse

- [ ] Segure a tecla da tela: o cursor aparece na tela flutuante e os cliques funcionam?
- [ ] Num caminhão em que a tela do MaxDevMod **não** aparece no GPS (sem suporte), a tela flutuante funciona com o
      mouse também?
- [ ] Num frete rápido (sem tablet), ela aparece?

## 4. O que já se sabe que falta

- O volante e os braços ainda **não** passam na frente dela (ela é desenhada por cima). Se o resto der certo, esse é
  o próximo passo.
- À noite ela fica bem clara (é uma tela acesa). Diga se incomoda.

## Se não aparecer

Mande o `Documentos\WestPlay\logs\plugin.log`. As linhas que importam começam com `tela flutuante:`:
`switched on`, `ready to be drawn` e, a cada 10 s, `10 s: main-view blocks …, the cab's …, nearest head … cm; cab
found in … frames, drawn …`. Com `the cab's 0`, o plugin não achou a cabine; o `nearest head` diz quão perto chegou.

### Desliga com o caminhão (21/11, 2.4.1)
Teste do dono: "desligando o caminhão, a tela flutuante não desliga". Agora ela some com a energia do caminhão
desligada, como a tela do próprio caminhão, e o mouse também não vai para ela.
- [ ] (2.4.2, pedido do dono: a tela fica, a imagem apaga) Desligar o caminhão de vez: a moldura fica e a imagem
      fica **preta**. Ignição ligada com o motor desligado: continua acesa. Religando: aparece o MaxDevMod e depois a
      tela, como a do caminhão.
