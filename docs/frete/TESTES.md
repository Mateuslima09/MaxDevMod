# Nota fiscal e engate manual — relatório de teste (para o dono)

Instalado no PC em 2026-11-11 e **ainda não testado dentro do jogo**. Testar no **single player, com o jogo salvo**,
o **app WestPlay aberto**, o **Modo Personagem ligado** e o **jogo reiniciado** (app e plugin mudaram juntos: a
ponte entre eles é a versão 33). Marque cada item e anote o que achou. Nada foi para o GitHub.

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
- **Obstáculos:** o plugin não enxerga os objetos do pátio. O conferente escolhe entre 7 pontos em volta da carreta e
  evita o **caminhão** e **você**; outras carretas, muros e empilhadeiras ele não vê.
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
