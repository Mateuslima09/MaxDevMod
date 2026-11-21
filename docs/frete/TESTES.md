# Nota fiscal e engate manual — relatório de teste (para o dono)

Instalado no PC em 2026-11-11 e **ainda não testado dentro do jogo**. Testar no **single player, com o jogo salvo**,
o **app WestPlay aberto**, o **Modo Personagem ligado** e o **jogo reiniciado** (app e plugin mudaram juntos: a
ponte entre eles é a versão 35, desde o abastecimento manual). Marque cada item e anote o que achou. Nada foi para o GitHub.

Liga/desliga: celular do jogo › **Ajustes › Nota fiscal e engate manual** (vem ligado). Se algo travar o jogo
(não conseguir engatar ou entregar), **desligue ali** e tudo volta ao normal na hora.

## Como funciona (resumo)

- Vale só para frete com o **seu caminhão**: buscar uma carreta, ou carregar a sua carreta. Frete rápido (caminhão
  da empresa, carreta já engatada e carregada) não tem nota.
- **Coleta:** um conferente fica ao lado da frente da carreta. Você desce (F8), chega nele, aperta **E**, a nota
  fiscal aparece grande no meio da tela; assina com o mouse, o conferente carimba, você clica **Coletar**. Só então o
  engate é liberado.
- **Engate manual:** a tecla de engatar (T) não engata mais de dentro da cabine. Encoste a quinta roda sob o pino,
  desça, vá até a quinta roda e aperte **E**: o personagem agacha, a carreta engata. O conferente vai embora andando.
- **Entrega:** chegando ao destino e parando, o destinatário aparece ao lado da carreta. **E** nele, ele assina e
  carimba o canhoto, você coleta. Só então o Enter (finalizar entrega) funciona.
- Os avisos aparecem no canto da tela, como os do jogo ("Colete a nota fiscal com o conferente para engatar a
  carreta", "Desça do caminhão e engate a carreta na quinta roda" etc.).

## 1. Ajustes e frete rápido

- [ ] Celular › Ajustes mostra **Nota fiscal e engate manual** (Ligado/Desligado).
- [ ] Num **frete rápido**: nada muda (sem conferente, entrega normal com Enter).
- [ ] Com a opção **desligada**: T engata da cabine como antes.

## 2. Coleta (frete com seu caminhão, buscando a carreta)

- [ ] Chegando perto da carreta (até ~150 m) o **conferente aparece** ao lado da frente dela, em pé, sem estar
      dentro de nada. (Se ficou dentro de um objeto, parede ou outra carreta: mande uma foto.)
- [ ] Apertar **T na cabine**: não engata e aparece o aviso para coletar a nota fiscal.
- [ ] A pé, perto do conferente, a barra de teclas mostra **E — Assinar a nota fiscal**; ele se vira para você.
- [ ] **E** abre a nota no meio da tela com os dados certos (remetente, destinatário, carga, peso, distância,
      valor, caminhão). O mouse mexe nela **sem segurar Alt**; o personagem fica parado.
- [ ] Desenhar a assinatura com o botão do mouse segurado; **Limpar** apaga; **Assinar** só libera com um traço de
      verdade.
- [ ] Depois de assinar: carimbo **CONFERIDO / LIBERADO** com som, botão **Coletar a nota fiscal**, aviso "Nota
      fiscal coletada: engate liberado".
- [ ] O **×** fecha a nota sem coletar (continua bloqueado).

## 3. Engate manual

- [ ] T na cabine, já com a nota: aviso "Desça do caminhão e engate a carreta na quinta roda".
- [ ] Com a quinta roda sob o pino, a pé perto dela: barra mostra **E — Engatar a carreta**.
- [ ] **E**: o personagem agacha ~2 s, som de trava, a carreta **engata**, aviso "Carreta engatada".
- [ ] Longe do pino: aviso "Encoste a quinta roda sob o pino da carreta para engatar" e nada engata.
- [ ] Depois do engate o conferente **vai embora andando** e some.
- [ ] Desengatar (T na cabine, carreta engatada) continua funcionando.
- [ ] O **E** a pé não liga/desliga mais o motor.

## 4. Sua carreta (mercado de cargas)

- [ ] Parado no local de carga com a sua carreta vazia: o conferente aparece; **Enter não carrega** antes da nota
      (aviso "Colete a nota fiscal com o conferente para carregar"); depois da nota, carrega normal.

## 5. Entrega

- [ ] Parado no destino (rota do GPS com menos de ~300 m), o **destinatário aparece** ao lado da carreta.
- [ ] **Enter** antes da assinatura: não entrega, aviso "Pegue a assinatura do destinatário para entregar a carga".
- [ ] **E** no destinatário: a nota aparece com a **sua assinatura da coleta**; "Entregar ao destinatário" → ele
      assina, carimbo **RECEBIDO**, "Coletar o canhoto", aviso "Canhoto assinado: entrega liberada".
- [ ] Enter agora finaliza a entrega normalmente.
- [ ] Fechar e abrir o jogo no meio do frete: não pede de novo o que já foi assinado.

## Limites conhecidos (não deu para fazer)

- **O aviso original do jogo** ("pressione T para engatar") **continua aparecendo**: não dá para tirar sem mexer no
  código do jogo. A tecla só deixa de engatar, e o aviso do WestPlay aparece junto.
- **Obstáculos:** o plugin não enxerga os objetos do pátio (o único jeito, o raio da física do jogo, fechou o jogo
  em 2026-11-05). Por isso, desde 2026-11-12, o conferente fica onde quase nunca há nada: **na frente do caminhão**
  (com o caminhão na carreta) ou **na frente do nariz da carreta**, para o lado (com a carreta sozinha). Ele evita o
  **caminhão** e **você**; um muro ou outra carreta exatamente ali ele não vê.
- **Controle/volante:** só as teclas do **teclado** são retidas. Um botão de controle ligado a "engatar" ou "ativar"
  ainda funciona (aí o WestPlay considera a coleta feita e não cobra mais a nota).
- **Animação de engate:** é o agachar do personagem + som; não há animação própria de mexer na trava.
- Se a rota do GPS estiver desligada (distância 0), o WestPlay não sabe que você chegou: o destinatário não aparece
  e a entrega fica livre.
- Se o jogo não tiver as pessoas do WestPlay (nem as do jogo), não há conferente e a nota é dispensada sozinha.

Se algo não funcionar, mande `Documentos\WestPlay\logs` — as linhas `freight:` contam o que o plugin viu.

## Voltar atrás

Arquivos de antes com o final `.antes-nota-fiscal` em `%LOCALAPPDATA%\WestPlay` (`westplay.exe`, `westplay.dll`,
pasta `ui`) e a DLL de cada jogo em `bin\win_x64\plugins`. Com o jogo e o WestPlay fechados, apague os novos e tire
o `.antes-nota-fiscal` do nome dos antigos.

## Correção de 2026-11-11 (17h50) — depois do primeiro teste do dono

O dono testou e **nada de engate nem de abastecimento manual funcionava**. Causa (no `plugin.log`: "input device
registered (-2)"): o comando `attach` estava duas vezes no dispositivo de comandos do plugin e o jogo recusava o
dispositivo inteiro. Corrigido (só o plugin mudou: basta **reiniciar o jogo**).

Junto com a correção:

- **Desengate manual:** com a carreta engatada, a pé na **lateral do caminhão** (dos dois lados, de trás da porta
  até a quinta roda), a barra mostra **E — Desengatar a carreta**. Sem carreta, o mesmo lugar mostra **E — Engatar a
  carreta**. A tecla T na cabine não engata nem desengata mais (aviso: "Desça do caminhão e desengate a carreta pela
  lateral").
- **Abastecer só sobre o ícone verde:** nesse mesmo lugar, a opção **E — Abastecer** só aparece com o caminhão
  parado sobre o ícone verde da bomba (e o tanque não cheio); aí ela vem antes do engate. Fora da bomba o E não fala
  mais de bomba: é o engate/desengate.
- Em mapa cujas bombas o WestPlay não conhece (mapa mod sem mapa do WestPlay): é o próprio jogo que diz onde é o
  ícone — aperte Enter na cabine sobre o ícone; o jogo começa a abastecer, o WestPlay para e manda descer.
- Se o jogo recusar o dispositivo de comandos de novo, T e Enter voltam a ser do jogo (nada fica travado).

- [ ] `plugin.log` mostra "commands: input device registered (0)".
- [ ] E na lateral, carreta engatada: agacha, desengata, aviso "Carreta desengatada".
- [ ] E na lateral, sem carreta, quinta roda sob o pino: engata.
- [ ] Sobre o ícone verde: a lateral oferece Abastecer; fora dele, Engatar/Desengatar.
- [ ] Os comandos do celular (vidros, suspensão…) voltaram a funcionar.

### Sem apertar Enter (2026-11-11, 18h)

Ideia do dono: o jogo avisa "desligue o motor para abastecer" ao chegar no ícone. O plugin agora **lê os avisos do
próprio jogo** e, com o caminhão parado e um aviso de abastecer na tela, oferece **E — Abastecer** na lateral, sem
Enter. Com o motor ligado, o E avisa "Desligue o motor para abastecer". Vale nos mapas cujas bombas o WestPlay não
conhece; o Enter na cabine continua servindo de segunda via.

- [ ] Parado sobre o ícone verde, sem apertar Enter: a lateral oferece **E — Abastecer**.
- [ ] `plugin.log` tem linhas `refuel: the game says "..."` com o texto do aviso do jogo (se não tiver nenhuma com
      "(fuel)", mande o log: é com ele que se ajusta a leitura).

### Ajustes de 2026-11-11 (18h30), depois das fotos do dono

- **Distância:** engatar/desengatar só colado no caminhão (até 2,1 m do meio dele, entre a porta e 0,7 m depois
  da quinta roda). Antes pegava até ao lado do pé da carreta.
- **Animação de mãos:** em vez de só agachar, o personagem desce num joelho e trabalha com as duas mãos à frente,
  perto do chão (a direita girando como numa manivela, a esquerda segurando), e levanta no fim. Feita no Blender a
  partir do agachar de cada personagem (`tools/personagem/engatar.py`), nos três (tatuado, mulher, cowboy).
- **Celular:** ao sair de um app volta para a página da tela inicial em que estava.
- **Desengate:** o aviso "Carreta desengatada" esperava pouco (o jogo leva ~5 s para soltar); agora espera até 9 s.

Atenção: a animação nova entra no mod do WestPlay, que **só é refeito com o jogo fechado**. Feche o jogo, espere
~30 s com o WestPlay aberto e abra o jogo de novo.

- [ ] Engatar/desengatar só aparece colado no caminhão (não ao lado do pé da carreta).
- [ ] O personagem mexe as mãos ao engatar/desengatar (não fica com os braços parados).
- [ ] Sair do TikTok (página 2) volta para a página 2.

### 2026-11-11 (18h45): animação uma vez só e área só na quinta roda

- A animação de engatar agora toca **uma vez**: desce, trabalha com as mãos ~2 s, levanta (antes o meio dela
  repetia e parecia agachar três vezes). A carreta engata/desengata quando ele começa a levantar.
- **Engatar/desengatar** só ao lado da quinta roda, entre as rodas traseiras (de 0,6 m antes a 1 m depois dela,
  dos dois lados, até um braço de distância do caminhão).
- **Abastecer** só ao lado do tanque, que termina onde começa a área da quinta roda: as duas nunca se sobrepõem.
- O `plugin.log` agora anota onde você estava ao apertar E ("freight: E at …"), para afinar a área se precisar.

De novo: feche o jogo e espere ~30 s com o WestPlay aberto para o mod pegar a animação nova.

- [ ] Engatar/desengatar: agacha **uma vez**, mexe as mãos, levanta.
- [ ] Ao lado do tanque não aparece engatar; entre as rodas traseiras não aparece abastecer.

### 2026-11-11 (19h): ignição no celular e botões de página

- Celular › **Comandos** › **Ignição**: "Ligar a ignição" (a primeira volta da chave, liga a parte elétrica),
  "Dar partida (segure)" (segure até o motor pegar) e "Desligar o motor".
- Tela inicial: setinhas nos cantos, ao lado dos pontinhos, para ir à outra página sem arrastar.
- Como o jogo registra os comandos ao abrir, **reinicie o jogo** para os botões de ignição funcionarem.

- [ ] Ligar a ignição acende o painel; Dar partida (segurando) liga o motor; Desligar o motor desliga.
- [ ] As setinhas trocam de página.

### 2026-11-11 (19h30): conferente do jogo e aviso do T

- **Conferente:** agora é um trabalhador do próprio jogo, não o seu personagem. São três, e cada frete pega
  sempre o mesmo: o de colete (cruza os braços), o segurança (mãos para trás) e o mecânico de serviço. Na entrega
  aparece outro deles. Indo embora, anda com o andar do jogo.
- **Aviso do T:** o texto do jogo "Pressione o seguinte botão para engatar o reboque: T" foi trocado (pelo mod do
  WestPlay, em todos os idiomas) para "Engate a carreta a pé, na quinta roda (tecla E). Pela cabine não funciona: T".
  O jogo ainda mostra o aviso e a letra T no fim; só o texto mudou. (Espanhol tem o seu; os outros idiomas, inglês.)
- As duas coisas estão no mod: **feche o jogo**, espere ~30 s com o WestPlay aberto e abra de novo.

- [ ] O conferente é um trabalhador do jogo (colete, segurança ou mecânico), em pé ao lado da carreta.
- [ ] O aviso do T aparece com o texto novo.

## Engatar só com carreta perto (2026-11-11)

- [ ] Longe da carreta (ela deixada em outra empresa), na quinta roda: a barra NÃO mostra 'E Engatar a carreta'.
- [ ] Com a carreta de ré até a quinta roda (até uns 8 m do pino): aparece 'Engatar a carreta'.
- [ ] Engatada: continua aparecendo 'Desengatar'.


## Conferente no chão e aviso do T mascarado (2026-11-11)

- [ ] Chegando de caminhão (ainda na cabine): o conferente aparece em pé no chão, não enterrado até a cintura.
- [ ] Depois de descer: pés no chão (nem enterrados nem flutuando).
- [ ] O aviso fixo 'Pressione o seguinte botão para engatar o reboque: T' passa a dizer 'Engate a carreta a pé, na quinta roda: tecla E'.

- [ ] (2ª tentativa) Conferente: ao chegar a pé a até 3 m dele, ele assenta no chão onde você pisa (nem flutuando nem enterrado).
- [ ] (2ª tentativa) Aviso do T: o jogo passa a mostrar 'Engate a carreta a pé, na quinta roda: tecla E' desde o início.

- [ ] O conferente só aparece depois que você desce do caminhão (de dentro da cabine, ao chegar, ele ainda não está lá); voltando à cabine, ele continua.

- [ ] Desengatar a pé: uma única animação; durante uns 4 s depois de desengatar, apertar E na quinta roda não faz nada (não reengata).

- [ ] Entrega: o aviso fixo de desengatar com T passa a dizer 'Desengate a carreta a pé, na quinta roda: tecla E'.
- [ ] Entrega: o conferente só aparece depois de descer, no chão, e não vai embora e volta enquanto você estaciona em cima do ponto de entrega.

- [ ] Entrega: no ponto de entrega, E na quinta roda sem assinar o canhoto mostra 'Pegue a assinatura do destinatário para entregar a carga' e não desengata; depois de assinar, desengata e entrega.

- [ ] Engatar/desengatar a pé: a animação começa do início (abaixa, mãos, sobe) uma vez só, também na 2ª e 3ª vez seguidas.
- [ ] Conferente: os pés acertam no chão já a uns 15 m, e de novo mais perto, sem tremer.


## ATS fechando sozinho (2026-11-12)

- [ ] ATS: jogar uns 15 minutos, descer e subir (F8) e engatar: o jogo não fecha mais. Causa: o conferente 'segurança' não existe no ATS (game.log: 'Unable to load model (mover.wp_clk2_idle)'); o mod do ATS agora só leva o funcionário de colete e o de serviço.
- [ ] Se fechar de novo: mandar o Documentos\WestPlay\logs\plugin.anterior.log.


## Coleta com carreta própria (2026-11-12)

- [ ] Pegar um frete com a própria carreta, já no pátio da empresa: ao descer, o conferente aparece; o Enter da cabine não carrega antes da nota fiscal.
- [ ] Assinar e coletar a nota: a carreta é carregada sozinha logo em seguida (sem apertar Enter). Se não carregar, o log diz 'loading asked of the game (the cargo loaded: ...)'.

- [ ] Mercado de cargas: coletar a nota com a carreta fora da área de carregamento e depois estacionar nela: carrega sozinho assim que o jogo mostra que pode carregar ali (log: 'the game can load the trailer here'). Com a carreta já na área ao coletar: carrega na hora.

- [ ] Mercado de cargas: o aviso 'Pressione o seguinte botão para carregar a carga no seu reboque: Enter' passa a dizer 'Colete a nota fiscal com o conferente: a carga é carregada em seguida', e ainda carrega sozinho depois da nota.


## Descarga com carreta própria (2026-11-12)

- [ ] Mercado de cargas, no destino: ao descer, o conferente aparece mesmo com o GPS mostrando alguns km (o plugin usa o aviso do jogo de descarga).
- [ ] Enter da cabine não descarrega antes de assinar; ao assinar o canhoto, descarrega sozinho (ou assim que o jogo mostrar o Enter de descarregar).
- [ ] O aviso de Enter para descarregar passa a dizer 'Entregue a nota fiscal ao conferente: a carga é descarregada em seguida' (se o código do aviso for outro, o log mostra o texto).

- [ ] Coletar a nota com o motor ligado e a carreta no chão verde: o motor desliga sozinho e a carga entra em seguida (log: 'the engine switched off to load', depois 'the game can load the trailer here'). O mesmo na descarga ao assinar.

## Conferente na frente do caminhão (2026-11-12, plugin novo instalado)

Antes ele nascia ao lado da carreta e, com uma parede colada nela, ficava dentro da parede (suas fotos de 2026-11-12,
no corredor entre o prédio e a carreta-tanque). O plugin não consegue ver paredes sem arriscar fechar o jogo, então
agora ele vai direto para onde quase nunca há obstrução:

- [ ] Coleta, com o caminhão já encostado na carreta (de ré até ela, ou engatado): ao descer, o conferente aparece
      **na frente do caminhão, do lado do motorista**, a uns 2 m do para-choque, olhando para o caminhão. No mesmo
      lugar das fotos (parede ao lado da carreta): ele aparece livre, fora da parede.
- [ ] Coleta, descendo com o caminhão ainda longe da carreta: ele aparece **na frente do nariz da carreta**, uns 3 m
      para o lado do motorista (fora do caminho por onde o caminhão dá ré).
- [ ] Assinar a nota com **E** perto dele funciona igual; depois do engate na quinta roda ele vai embora andando
      **para a frente e para fora**, sem atravessar a parede, e some.
- [ ] Carregar (sua carreta) e entregar: com a carreta engatada, ele aparece na frente do caminhão; assinar funciona.
- [ ] ATS (caminhão de bico): ele fica na frente do capô, não dentro dele. Se ficar dentro ou longe demais, mande o
      log: a linha "freight: the clerk's spot [x, z] from the truck" diz onde ele foi posto.
- [ ] Caminhão estacionado de frente para uma parede (raro): ele pode ficar na parede. Se acontecer, me avise.
- Voltar atrás: as DLLs antigas ficaram como `westplay.dll.antes-conferente-frente` (nos dois jogos e em
  `%LOCALAPPDATA%\WestPlay`).

## Quinta roda e pés da carreta à mão (19/11; app, telas e plugin novos: **feche e abra o jogo**)

Pedido do dono: "um minigame da quinta roda: pra desengatar puxa a trava, pra engatar aperta; e um minigame dos pés
da carreta, girando a manivela, igual na vida real". O jogo só faz o engate/desengate de verdade depois dos passos:

**Desengatar:** pés para baixo (manivela) → puxar a trava (quinta roda) → sair com o caminhão.
**Engatar:** encostar sob o pino → empurrar a trava (quinta roda) → pés para cima (manivela) → o jogo engata.

- Os **pés** ficam na lateral da carreta, de 1,2 m a 5 m atrás do pino, do mesmo afastamento da quinta roda. A barra de
  teclas mostra **Baixar/Subir os pés da carreta**.
- **Manivela:** segure a manopla vermelha e gire em círculos com o mouse: **4 voltas**, **horário para baixar**,
  **anti-horário para subir** (tique a cada quarto de volta; no sentido errado, volta o progresso).
- **Trava:** segure o puxador vermelho e arraste: **para fora** para abrir (desengatar), **para dentro** para travar
  (engatar). Soltando antes do fim, ele volta. No fim, "clack" e fica verde.
- O **×** fecha sem fazer nada. Depois de 90 s sem terminar, o personagem levanta e nada acontece.
- Sem o app aberto, engata e desengata como antes (sem os passos).

- [ ] Carreta engatada, E na quinta roda **com os pés em cima**: aviso "Abaixe os pés da carreta antes de desengatar".
- [ ] E nos pés (lateral da carreta): abre a manivela; 4 voltas no sentido horário: "Pés da carreta abaixados".
- [ ] E na quinta roda: abre a trava; puxe até o fim: o personagem levanta e a carreta **desengata**.
- [ ] Engatar: encoste sob o pino, E na quinta roda: abre a trava; empurre até o fim: "Quinta roda travada: agora
      suba os pés". A carreta **ainda não** engatou.
- [ ] E nos pés: 4 voltas anti-horário: o jogo **engata** ("Carreta engatada").
- [ ] Antes de travar, E nos pés com a carreta solta: "Encaixe a carreta e trave a quinta roda antes de subir os pés".
- [ ] Travou e saiu andando com o caminhão sem subir os pés: a trava abre de novo (log: `the truck moved off`).
- [ ] O mouse pega bem o puxador e a manopla? Girar em círculos é confortável? (dá para mudar as voltas: hoje 4).
- [ ] A posição dos pés bate com a carreta de verdade? Se o E não pegar lá, me diga onde ficou (o log diz o lado
      e a distância só para a quinta roda, por enquanto).

### Correções de 19/11 (noite; app novo)
- O **celular piscava** um instante antes da manivela ou da trava: a janela aparecia antes de a página trocar de tela.
  Agora ela só aparece 150 ms depois, já com o minigame desenhado. Vale para a nota fiscal também.
- **Sons novos**, feitos no próprio app: a trava com aço ressoando, um raspado e um baque pesado; o puxador voltando
  com raspado de metal; a manivela com a catraca (um clique a cada quarto de volta) e um ronco de engrenagem que só
  toca enquanto gira, mais alto quanto mais rápido.
- [ ] E nos pés ou na quinta roda: o celular **não pisca** mais antes?
- [ ] Os sons parecem trava e manivela? Se não, posso trocar por gravações de verdade (com a sua licença/arquivo).

### Sons de verdade e pés sem agachar (19/11, noite; app e plugin: **reinicie o jogo**)
- Os sons que o dono escolheu: `desengate.mp3` (ElevenLabs: só o estalo, de 0,10 a 0,42 s) na trava (puxar e
  empurrar), e `catraca.mp3` (Pixabay: 19 cliques recortados um a um, tocados em rodízio) na manivela, agora com
  **um clique a cada 1/8 de volta** (32 nas 4 voltas). O ronco de engrenagem ficou mais baixo. Os dois estão
  registrados em `THIRD_PARTY_NOTICES.md`.
- Nos **pés da carreta** o personagem **não agacha** mais: fica parado em pé enquanto gira.
- [ ] A trava faz o estalo e a manivela faz a catraca, clique a clique?
- [ ] Nos pés o personagem fica em pé?
