# Abastecimento manual — relatório de teste (para o dono)

Instalado no PC em 2026-11-11 e **ainda não testado dentro do jogo**. Testar no **single player, com o jogo salvo**,
o **app WestPlay aberto**, o **Modo Personagem ligado** e o **jogo reiniciado** (app e plugin mudaram juntos: a
ponte entre eles é a versão 35). O **mapa foi refeito** para guardar o lugar de cada bomba (o do ETS2 já está pronto;
o do ATS é refeito sozinho na primeira vez que o WestPlay abrir com o ATS escolhido e o jogo fechado). Marque cada item e anote o que achou. Nada foi para o GitHub.

Liga/desliga: celular do jogo › **Ajustes › Abastecimento manual** (vem ligado). Se algo travar (não conseguir
abastecer de jeito nenhum), **desligue ali** e o Enter na bomba volta a abastecer na hora.

## Como funciona (resumo)

- O WestPlay sabe onde fica cada bomba pelo mapa: é o **ícone verde do galão**. Pare o caminhão sobre ele, como
  sempre. Ali a tecla de abastecer (**Enter**) na cabine não abastece: aparece o aviso **"Desça do caminhão e
  abasteça pelo tanque"**.
- Numa bomba que o mapa não tenha (mapa mod, ou mapa ainda não refeito), vale o jeito antigo: o primeiro Enter na
  cabine deixa entrar um pinguinho de combustível, e é aí que o WestPlay descobre a bomba.
- Desça (F8) e vá para o **lado do caminhão, atrás da porta da cabine** (qualquer um dos dois lados). A barra de
  teclas mostra **E — Abastecer**.
- **E**: o personagem fica parado, o tanque enche sozinho. **E** de novo para. Com o tanque cheio para sozinho e
  aparece **"Tanque cheio: N litros"** (galões no ATS). O jogo cobra o combustível como sempre.
- Saiu de cima do ícone com o caminhão (uns 7 m): tudo volta ao normal até a próxima bomba.

## 1. Ajustes

- [ ] Celular › Ajustes mostra **Abastecimento manual** (Ligado/Desligado), abaixo de "Nota fiscal e engate manual".
- [ ] Com a opção **desligada**: Enter na bomba abastece da cabine como antes.

## 2. Na cabine, na bomba

- [ ] Sobre o ícone verde, Enter: **não entra combustível nenhum**; aparece "Desça do caminhão e abasteça pelo tanque".
- [ ] Se entrou um pouquinho antes do aviso, o mapa não tinha essa bomba: anote o posto (cidade) e quanto entrou.
- [ ] A área bate com o ícone? (Bloqueia longe demais do ícone / não bloqueia em cima dele: anote.)
- [ ] Enter de novo na cabine: não abastece, o aviso aparece de novo.
- [ ] Enter **fora** de posto (dormir, pegar frete, pedágio, balsa) continua funcionando normalmente.

## 3. A pé, no tanque

- [ ] Ao lado do caminhão, atrás da porta: a barra mostra **E — Abastecer**. Dos dois lados.
- [ ] O lugar está bom? (Muito para frente / muito para trás / longe do tanque de verdade: anote o caminhão.)
- [ ] **E**: som, aviso "Abastecendo: aperte E para parar", o personagem não anda, o combustível sobe (olhe no
  celular › Caminhão, ou no painel depois).
- [ ] A barra mostra **E — Parar de abastecer**; **E** para e aparece "Abastecido: N litros".
- [ ] Deixando encher: para sozinho com "Tanque cheio: N litros" e o personagem volta a andar.
- [ ] O dinheiro foi cobrado certo (confira no Nupay ou no jogo).
- [ ] No **ATS** o aviso fala em **galões**; no ETS2, em litros.
- [ ] Com o tanque já cheio: a barra não oferece abastecer.
- [ ] Sem apertar Enter na cabine: parou sobre o ícone, desceu, e o **E — Abastecer** já aparece no tanque.
- [ ] Fora do ícone (estacionado em outro lugar): **E** no tanque mostra "Pare o caminhão sobre o ícone verde da
  bomba para abastecer" e nada acontece.
- [ ] Perto da quinta roda sem carreta: aparece **Engatar a carreta** em vez de abastecer (dê um passo para a
  frente e vira **Abastecer**).

## 4. Depois

- [ ] De volta na cabine com o tanque cheio: Enter funciona normal para outras coisas.
- [ ] Parou de abastecer pela metade e voltou para a cabine: Enter continua sem abastecer ali (é para descer).
- [ ] Andando com o caminhão para outra bomba: o ciclo recomeça (Enter, aviso, descer).
- [ ] Nota fiscal e engate manual continuam funcionando com o abastecimento ligado.

## Limites conhecidos

- As bombas vêm do mapa que o WestPlay monta do seu jogo. Uma bomba que não esteja nele só é descoberta pelo
  primeiro Enter na cabine (entra um pouquinho de combustível).
- A área em volta do ícone é um círculo de uns 7 m a partir do ponto do caminhão; pode não bater exatamente com a
  do jogo.
- O lugar do tanque é aproximado (o mesmo para todos os caminhões): ao lado, atrás da cabine.
- Não há mangueira nem animação: o personagem fica parado em pé enquanto enche. O som é o da porta.
- Só a tecla de abastecer do **teclado** é segurada: num **controle/volante**, o botão de ativar ainda abastece da
  cabine.
- No TruckersMP e no multiplayer do jogo fica desligado.
- Caminhão elétrico (recarga): não testado, pode não funcionar.

## Voltar atrás

Os arquivos de antes estão em `%LOCALAPPDATA%\WestPlay` com o final `.antes-abastecimento` (`westplay.exe`,
`westplay.dll` e a pasta `ui`), e a DLL de cada jogo em `bin\win_x64\plugins` com o mesmo final. Com o jogo e o
WestPlay fechados, apague os novos e tire o `.antes-abastecimento` do nome dos antigos.

## Tanque reconhecido mais à frente (2026-11-11)

- [ ] Na bomba, motor desligado, a pé ao lado do degrau da cabine / tanque: aparece 'E Abastecer' sem precisar apertar Enter antes.
- [ ] Se não aparecer, apertar E ali mesmo: o plugin.log anota onde você estava ('refuel: E at a pump, not by the tank') para eu ajustar.

- [ ] Tanque já cheio, na bomba, ao lado do tanque: aparece 'E Abastecer'; apertando E diz 'O tanque já está cheio'.

- [ ] Painel grande do posto ('Pressione e segure... Enter', preço, litros): passa a dizer 'Abasteça a pé: desça do caminhão e aperte E ao lado do tanque'. Se continuar igual, o plugin.log agora guarda o texto dele ('refuel: the game says ...') para eu ajustar.


## Painel do posto escondido (2026-11-12)

O painel grande do jogo no alto da tela (ícone da bomba, saldo, "Pressione e segure... Enter", preço, litros,
valor total) agora **não aparece** enquanto o abastecimento manual está ligado: o WestPlay impede o jogo de criá-lo
(antes só trocava o texto, e o jogo escrevia de novo a cada quadro). **Reinicie o jogo** (a DLL nova só carrega ao
abrir). Cópias de antes: `westplay.dll.antes-painel-bomba` nos dois jogos e em `%LOCALAPPDATA%\WestPlay`.

- [ ] Na bomba, a pé ao lado do caminhão: **só** a barra do WestPlay (E Abastecer), sem o painel do jogo no alto.
- [ ] Na cabine, sobre o ícone verde: o painel também não aparece; Enter mostra "Desça do caminhão e abasteça pelo tanque".
- [ ] **E** no tanque: o tanque continua enchendo, o painel não aparece enquanto enche, e no fim "Abastecido / Tanque cheio".
- [ ] O dinheiro continua sendo cobrado (o painel escondido não muda a cobrança).
- [ ] Avisos "Desligue o motor" e "Tanque cheio" do jogo continuam aparecendo normalmente.
- [ ] Com **Abastecimento manual desligado** (Ajustes) ou o Modo Personagem desligado: o painel do jogo volta e o Enter abastece como sempre.
- [ ] Se o painel ainda aparecer: o `plugin.log` deve ter a linha "notice: the game's pump panel kept from the screen"; mande o log.

## Bico na mão, em primeira pessoa (2026-11-12)

Ao apertar **E** no tanque para abastecer, a câmera vai para a **primeira pessoa** (sem o corpo), virada para o
caminhão e olhando para baixo, e aparece o **braço com luva segurando o bico da bomba** ao lado do tanque, com um
pedaço de mangueira descendo até o chão. Dá para olhar em volta com o mouse: o bico fica parado no tanque. Ao parar
(E de novo ou tanque cheio) o bico some e a câmera volta como estava (terceira pessoa, se estava nela). O modelo é
simples (caixas e tubos, sem textura) e foi feito pelo WestPlay. Precisa do **app novo aberto antes do jogo** (é ele
que põe o bico no mod) e do **jogo reiniciado**.

- [ ] E no tanque: vai para a primeira pessoa e o bico aparece na mão, perto do tanque.
- [ ] Posição: o bico está alto / baixo / longe / perto demais? O braço aparece estranho? (Anote ou mande print.)
- [ ] Cores: o bico preto, o cano prateado, a luva marrom, a manga escura. (Se aparecer tudo preto ou branco, anote.)
- [ ] Olhando em volta com o mouse: o bico fica parado no tanque; a mangueira desce até o chão.
- [ ] V não troca a câmera enquanto abastece.
- [ ] E de novo (ou tanque cheio): o bico some e a câmera volta para como estava antes.
- [ ] Dos dois lados do caminhão.
- [ ] Se o bico não aparecer: o `plugin.log` diz "character: no nozzle (...)"; mande o log.

Voltar atrás (bico): em `%LOCALAPPDATA%\WestPlay` estão `westplay.exe.antes-bico`, `westplay.dll.antes-bico` e
`ui.antes-bico`; nos dois jogos, `bin\win_x64\plugins\westplay.dll.antes-bico`. Com o jogo e o WestPlay fechados,
apague os novos e tire o `.antes-bico` do nome.

## Braço de verdade do personagem segurando o bico (2026-11-12)

No lugar do braço de caixas, ao apertar **E** para abastecer aparece **o corpo do próprio personagem** (sem a
cabeça, como na primeira pessoa), com o **braço direito esticado e a mão fechada no punho do bico**, polegar para
cima, e a mangueira descendo até o chão. Feito para os três (tatuado, mulher, cowboy) com a pose criada no Blender
a partir do "parado" de cada um (`tools/personagem/abastecer.py`; nada do Mixamo novo). Se o mod não tiver esse
corpo, volta o bico com braço de caixas. **Abra o app novo antes do jogo** (ele refaz o mod) e reinicie o jogo.

- [ ] E no tanque: primeira pessoa, o braço do personagem com a mão segurando o bico (com a roupa e a pele dele).
- [ ] Com cada personagem (tatuado, mulher, cowboy).
- [ ] A mão segura bem o bico? (Bico solto da mão, dedos atravessando: anote o personagem e mande print.)
- [ ] O braço está numa altura boa para o tanque? (Alto / baixo / longe demais: anote.)
- [ ] Ao parar, o corpo com o bico some e a câmera volta como estava.
- [ ] O `plugin.log` diz "character: the body holding the nozzle (mover.xxf_fuel)"; se disser "no body holding the nozzle", mande o log.

Voltar atrás (braço): em `%LOCALAPPDATA%\WestPlay` estão `westplay.exe.antes-braco`, `westplay.dll.antes-braco`,
`ui.antes-braco` e `personagem.antes-abastecer`; nos jogos, `westplay.dll.antes-braco`. As cenas do Blender de
antes ficaram como `*_anima*.blend.antes-abastecer` em `personagem_scs\fonte`.

## Bico vermelho, mangueira por baixo e painel da bomba (2026-11-12)

- Bico **vermelho** (cano prateado, proteção do gatilho e mangueira pretos), um pouco **mais baixo** na mão, e a
  mangueira agora sai **por baixo do punho**, descendo direto, sem atravessar o braço (era o caso da mulher).
- **Painel da bomba** no alto, à direita da tela, enquanto abastece: **Litros** subindo, **Preço** por litro e
  **Total** em reais (galões e dólares no ATS). O preço é o do próprio painel do jogo naquela bomba. Ao terminar,
  o painel diz "ABASTECIDO" e fica uns 6 segundos.

- [ ] O bico aparece vermelho, e a mangueira não passa pelo braço (nos três personagens).
- [ ] O painel aparece ao apertar E e os litros sobem junto com o tanque.
- [ ] O preço bate com o do posto (o painel do jogo mostrava, por exemplo, R$5,46) e o total = litros × preço.
- [ ] O total bate (mais ou menos) com o que o jogo cobrou (veja no Nupay ou no extrato do jogo).
- [ ] Ao parar: "ABASTECIDO" com o total final, e some sozinho depois de uns segundos.
- [ ] Se o preço aparecer como "—": o jogo não mostrou o painel naquela bomba; anote o posto.

Voltar atrás (painel): `westplay.dll.antes-painel-litros` no WestPlay e nos dois jogos.

## Painel com cara de bomba de verdade (2026-11-12)

O painel no alto, à direita, agora imita a frente de uma bomba: faixa azul com **DIESEL** e "ABASTECENDO"/"FIM",
e três visores de cristal líquido com números de 7 segmentos (os "8" apagados ao fundo): **TOTAL A PAGAR (R$)**,
**LITROS** e **PREÇO / LITRO**. Galões e dólares no ATS.

- [ ] O painel aparece ao apertar E, com os números subindo enquanto enche.
- [ ] Dá para ler bem na sua resolução (tamanho bom, não tampa nada importante).
- [ ] Ao terminar mostra "FIM" e some depois de uns segundos.

Voltar atrás: `westplay.dll.antes-painel-bomba-real` no WestPlay e nos dois jogos.

## Pagar o abastecimento no frentista, pelo celular (2026-11-12)

Depois de abastecer a pé, o abastecimento fica **em aberto**:

1. Ao terminar, aparece "Pague o frentista: vá até ele e aperte E" e o painel da bomba fica com **A PAGAR**.
2. Um **frentista** (um dos trabalhadores do jogo, o de serviço) aparece em pé uns 2 m para fora do caminhão, um
   pouco à frente de onde você abasteceu, virado para você.
3. Perto dele, a barra mostra **E — Pagar o abastecimento**. O **E** abre o celular grande com o **Nupay**:
   "Pagamento para Posto", litros, preço, total e o botão **Pagar R$ …** (o "×" fecha sem pagar).
4. **Pagar**: aparece "Pago" no celular, ele volta como estava, o jogo diz "Pago: R$ …", o frentista some, o painel
   some, e o **Nupay** ganha a entrada **Posto de combustível** (Diesel · litros).
5. **Sair sem pagar** (o caminhão a uns 25 m de onde abasteceu): "Você saiu sem pagar o abastecimento!" e a conta
   some (nada é cobrado a mais: o jogo já tinha descontado o diesel ao encher).

Abastecer de novo antes de pagar soma na mesma conta. O dinheiro do jogo continua saindo na hora que enche (o
WestPlay não mexe no saldo do jogo); o Nupay converte o total pela taxa fixa dele, então o próximo save pode mostrar
uma pequena diferença como "Outros gastos". **App e plugin mudaram juntos** (a ponte é a versão 36): abra o app novo
e reinicie o jogo.

- [ ] Ao terminar de abastecer: o aviso de pagar e o painel em "A PAGAR".
- [ ] O frentista aparece perto, em pé, no chão (não enterrado nem flutuando), e vira para você.
- [ ] Perto dele: "E — Pagar o abastecimento"; o E abre o Nupay com a conta certa (litros, preço, total).
- [ ] O mouse funciona no celular; "Pagar" mostra "Pago" e fecha; o jogo diz "Pago: R$ …".
- [ ] O Nupay mostra "Posto de combustível" no histórico com o valor.
- [ ] O "×" fecha sem pagar e dá para abrir de novo com o E.
- [ ] Entrar no caminhão com o celular aberto: ele fecha e a conta continua (dá para descer e pagar).
- [ ] Sair com o caminhão sem pagar: o aviso aparece e o frentista some.
- [ ] Sem o frentista (se não aparecer): o E perto do caminhão paga; anote se aconteceu.

Voltar atrás: `westplay.exe.antes-pagamento`, `westplay.dll.antes-pagamento` e `ui.antes-pagamento` no WestPlay;
`westplay.dll.antes-pagamento` nos jogos.

### Ajustes do pagamento (2026-11-13)

- O **E no frentista abre o celular grande** no meio da tela, com o mouse (antes o app só fazia isso para a nota
  fiscal, e o celular não aparecia).
- **Depois de pagar o celular fecha**, mesmo se tinha sido aberto no Alt+4.
- O **painel da bomba** foi para o **alto, à esquerda**, e some enquanto o celular está aberto (não fica mais por
  cima dele). Os litros e o total no fim batem com os do celular (centésimos).
- O aviso **"Pague o frentista: vá até ele e aperte E"** aparece uns 3 segundos depois de terminar, depois do
  "tanque cheio" do jogo.

- [ ] E no frentista: o celular abre sozinho, grande, e o mouse funciona.
- [ ] Pagar: o celular fecha sozinho.
- [ ] O painel fica à esquerda e não tampa o celular.
- [ ] O aviso de pagar aparece depois do "tanque cheio".

Voltar atrás: arquivos `.antes-pagamento2` (exe, dll, ui) no WestPlay e nos jogos.
- [ ] (2026-11-13) O frentista agora fica no corredor entre o caminhão e a bomba, onde você abasteceu, uns 1,7 m à
  frente (perto da porta da cabine), e não mais dentro da ilha da bomba. Confira se ficou livre (fora da bomba e do
  caminhão). Voltar atrás: `westplay.dll.antes-frentista`.

### Frentista na frente do caminhão, saindo a pé, e bico com cano de metal (2026-11-13)

- O frentista agora fica **na frente do caminhão, do lado da bomba**, como o conferente da nota fiscal (uns 3,8 m à
  frente da cabine). Longe do tanque, o E não briga mais entre "abastecer" e "pagar".
- Depois de pagar, ele **sai andando** (para a frente e para fora, uns 9 s) e só então some.
- O **bico** saía todo vermelho: o corpo do personagem guarda a textura numa camada de nome "Attribute" e o bico
  em outra, que se perdia na exportação. Agora o bico usa a mesma: corpo **vermelho**, cano e gatilho **prateados**,
  mangueira e proteção **pretas**.

- [ ] O frentista aparece na frente do caminhão, do lado da bomba, livre (não dentro de nada).
- [ ] Perto dele, E paga; perto do tanque, E abastece (sem confusão entre os dois).
- [ ] Depois de pagar ele sai andando e some uns segundos depois.
- [ ] O cano do bico aparece prateado (não vermelho).

Voltar atrás: `westplay.dll.antes-frentista2` no WestPlay e nos jogos; os personagens em `personagem.antes-abastecer`.

## Liga/desliga em todo lugar (2026-11-13)

Agora são **três opções separadas**, guardadas no próprio WestPlay e iguais em todo lugar:

- **Nota fiscal e engate manual**
- **Abastecimento manual**
- **Pagar no frentista** (nova: desligada, abastece a pé mas não tem conta nem frentista)

Ficam no **celular do jogo › Ajustes**, no **seu telefone** (a página do WestPlay pelo QR Code › Ajustes) e no
**aplicativo do PC › Personagem**. Mudou num lugar, muda nos outros. (Antes ficavam só no celular do jogo, guardadas
nele mesmo: se você tinha desligado alguma ali, confira de novo, porque agora todas começam ligadas.)

- [ ] No app do PC, página Personagem: aparecem as três opções.
- [ ] Desligar no PC: o celular do jogo e o telefone mostram desligado.
- [ ] Desligar no celular do jogo: o PC mostra desligado.
- [ ] "Pagar no frentista" desligado: abastece a pé, aparece o painel, mas sem conta, sem frentista.

Voltar atrás: arquivos `.antes-opcoes` (exe, dll, ui) no WestPlay e nos jogos.

## O bico mais bonito (2026-11-14, ainda não visto no jogo)

Corpo e cabo com as quinas arredondadas, o cabo de borracha escura, a guarda numa peça curva só (eram duas caixas),
um anel no cano e uma luva de metal na mangueira, tudo com sombreado suave. Vale para os quatro personagens. Para
voltar ao bico de antes: `%LOCALAPPDATA%\WestPlay\personagem.antes-bico` e `westplay.exe.antes-bico`.

- [ ] Abastecendo a pé, em 1ª pessoa: o bico aparece arredondado, vermelho com o cabo escuro, sem buracos nem
      pedaços soltos?
- [ ] A mão continua fechada no cabo (nada atravessando a mão)?

## Preço guardado — novo (plugin, 2026-11-14 22:30)

Num teste, o dono desceu do caminhão antes de o jogo mostrar o painel da bomba com o preço (o jogo só tinha dito "desligue o motor"). O abastecimento encheu 70,9 L, mas sem preço lido o plugin não criou a conta: o frentista não apareceu e o mostrador ficou sem preço e sem total. Agora o último preço lido fica guardado em `Documents\WestPlay\<jogo>\preco-da-bomba.txt` e é usado quando o painel não aparece.
- [ ] Abastecer uma vez do jeito normal (motor desligado na bomba, esperando o painel aparecer, depois descer): o arquivo é criado?
- [ ] Depois, descer logo e abastecer: o frentista aparece e o mostrador mostra preço e total?
- [ ] O mostrador marcou 183,50 L com 70,9 L abastecidos: acontece de novo? Em que momento?
- Resultado 2026-11-15 01:35, com o app aberto: abasteceu 20,3 L, mas o mostrador ficou sem preço. O preço (R$ 6,42) só veio no painel de "tanque cheio" do jogo, e o plugin lia o preço apenas de painéis sem "tank_full". Agora lê de qualquer aviso que traga o preço. O arquivo `preco-da-bomba.txt` (ETS2) foi criado com 6.42.
- Antes disso, com o app fechado, o painel do jogo voltava a aparecer: era esperado, porque sem o app o abastecimento a pé fica desligado.
- Resultado 2026-11-15 12:00: preço, total e litros certos no mostrador (R$ 4,75), e o aviso de pagar o frentista apareceu. O painel do jogo na bomba ainda aparecia ao desligar o motor: o texto dele usa as chaves `@@diesel_price@@` e `@@litres@@`, sem "fuel", e não era reconhecido como de combustível. Agora `says_fuel` também aceita `_price@@`, `@@litres@@` e `@@gallons@@`.
- [ ] Desligar o motor na bomba: o painel "pressione e segure Enter" some (ou aparece com as palavras do WestPlay)?

## Segurar para abastecer + aviso de pagar (2026-11-20, ainda não visto no jogo)

O dono pediu: o aviso do jogo lá em cima ("aperte Enter") ainda aparecia; apertar E enchia o tanque até o fim
sozinho; e faltava um aviso para pagar o frentista. Agora:

- **Segure Enter (ou E) ao lado do tanque** para abastecer; **solte** para parar. Entra quanto você quiser. Enchendo,
  para sozinho com "Tanque cheio".
- Ao chegar ao lado do tanque aparece o **nosso** aviso no canto: "Segure Enter (ou E) para abastecer; solte para
  parar". A barra de teclas mostra **E — Segure para abastecer** e, abastecendo, **Solte para parar**.
- O painel do jogo na bomba ("pressione e segure Enter") é bloqueado de mais jeitos (pelas chaves do próprio
  painel, mesmo curto, e também na segunda linha do aviso). Se ainda aparecer, o log do plugin
  (`Documents\WestPlay\logs\plugin.log`) agora diz "notice: the game's pump panel shown": mande esse log.
- **Pagar:** depois de abastecer aparece "Pague R$ X ao frentista: ele está na frente do caminhão, aperte E perto
  dele", **repetido a cada 30 s** enquanto não pagar; ao voltar para a cabine sem pagar, "Você ainda não pagou o
  frentista: R$ X". E o **celular do cantinho** sobe com a notificação do Nupay "Pague R$ X ao frentista"
  (desliga em Ajustes › Notificações do jogo › Combustível).
- A ponte subiu para a versão **49**: reinicie o jogo.

- [ ] Parado na bomba, motor desligado: o painel do jogo com "Enter" **não** aparece lá em cima.
- [ ] Ao lado do tanque: aparece "Segure Enter (ou E) para abastecer".
- [ ] Segurando **Enter**: abastece; soltar para na hora, "Abastecido: N litros".
- [ ] Segurando **E**: o mesmo.
- [ ] Um toque rápido (soltar antes de começar a entrar): não diz "a bomba não abasteceu", só repete como fazer.
- [ ] Segurando até encher: para sozinho com "Tanque cheio".
- [ ] Abastecer um pouco, soltar, segurar de novo: soma na mesma conta do frentista.
- [ ] Depois de soltar: aviso de pagar com o valor, e o celular sobe com "Pague R$ X ao frentista".
- [ ] Sem pagar: o aviso volta a cada ~30 s; entrar na cabine mostra "Você ainda não pagou o frentista".
- [ ] Pagar no frentista (E perto dele, pagar no Nupay): os avisos param.
- [ ] Com "Pagar no frentista" desligado: abastece segurando, sem aviso de pagar.

Voltar atrás: arquivos `.antes-abastecer-segurando` (exe, dll, ui) no MaxDevMod e nos jogos.
