# TikTok no celular e notificações — relatório de teste (para o dono)

O que foi instalado no PC em 2026-11-11 e ainda **não foi testado dentro do jogo**. Testar com o **app WestPlay
aberto** e o **jogo reiniciado** (o plugin mudou: um jogo que já estava aberto continua com o plugin antigo).
Marque cada item e anote o que achou. Nada foi para o GitHub.

## Como funciona (resumo)

- O app TikTok do celular agora é o **site oficial do TikTok** (tiktok.com), rodando no PC por baixo da tela do
  celular. O mouse em cima dele é o mouse da página do TikTok: rolar o feed com a rodinha, curtir, seguir, abrir
  comentários, tudo é o próprio TikTok.
- A conta entra em **WestPlay › Contas › TikTok** (janela de login do próprio TikTok, igual YouTube e Spotify).
  O WestPlay só fica sabendo se está logado e o @ da conta.
- Com a conta logada, o WestPlay abre escondida a página da **sua LIVE** e lê os eventos da sala: **seguiu,
  presente, compartilhou, inscreveu**. Cada um vira uma notificação no celular, com som.
- Celular guardado: só o **topo do celular sobe** no canto direito, mostra a notificação e desce sozinho.

## 1. Conta

- [ ] WestPlay › Contas mostra a linha **TikTok** com "Entrar".
- [ ] "Entrar" abre a janela de login do TikTok; depois de entrar a janela fecha e a linha vira "Conectado".
      (Login por QR code, e-mail ou Google: anote qual usou e se algum não funcionou.)
- [ ] "Sair" desconecta.

## 2. App TikTok no celular (Alt+4, segunda página de apps)

- [ ] Abre com "Abrindo o TikTok…" e depois o feed **Para você** aparece dentro da tela do celular.
- [ ] O vídeo toca com som; guardar o celular (Alt+4) **pausa e silencia**; tirar de novo continua.
- [ ] Rodinha do mouse (segurando Alt) passa para o próximo vídeo.
- [ ] Curtir, seguir, abrir comentários, compartilhar: funcionam como no site.
- [ ] Abas de baixo: **Voltar, Para você, Seguindo, Explorar, LIVE, Perfil** abrem as páginas certas.
- [ ] Escrever um comentário: clicar no campo e digitar no teclado (como nos outros campos do WestPlay, com o
      caminhão parado). Enter envia, Backspace apaga.
- [ ] A barrinha do início (embaixo) volta para a tela inicial e o TikTok para de tocar.
- [ ] O tamanho da página ficou bom? (Letras pequenas demais / cortado: mande uma foto.)
- [ ] Sem conta logada: aparece a faixa "Entre na sua conta em Contas, no WestPlay".

## 3. Notificação (sem precisar de live)

Celular › **Ajustes › Notificações da LIVE do TikTok › Testar notificação**.

- [ ] Com o celular na mão: aparece o banner "TikTok · Mateus seguiu você" no topo, com som, e some em ~4 s.
- [ ] Aperte "Testar notificação" e **guarde o celular logo em seguida** (ou teste durante uma live): o **topo do
      celular sobe** no canto direito de baixo, mostra o banner e desce sozinho.
- [ ] O som está num volume bom? (Alto / baixo: anote.)
- [ ] Apertar Alt+4 enquanto o topo está aparecendo traz o celular inteiro.
- [ ] O topo aparecendo não atrapalha o mouse nem as teclas do jogo.

## 4. Notificações de verdade (fazendo live no TikTok)

Com a conta logada e "Notificações da LIVE" ligadas, comece uma live (pelo celular de verdade ou OBS).

- [ ] Em até ~2 minutos o app TikTok do celular mostra a faixa "Sua LIVE está no ar · notificações ativas".
- [ ] Alguém **seguir** → "Fulano seguiu você".
- [ ] Alguém mandar **presente** → "Fulano enviou Rose ×5" (som diferente, mais longo).
- [ ] Alguém **compartilhar** → "Fulano compartilhou sua LIVE".
- [ ] O jogo continua liso com a live sendo lida? (Queda de FPS: anote.)

**Atenção:** esta é a parte menos garantida. O TikTok não tem um jeito oficial de avisar esses eventos; o WestPlay
lê as mensagens da sala como a página da live as recebe. Não consegui testar com uma live de verdade. Se nada
chegar, mande o `app.log` (Documentos\WestPlay\logs): as linhas `[tiktok]` dizem se a live foi vista
("the LIVE is on").

## Limites conhecidos

- O TikTok só aparece no **celular do jogo**; no seu celular de verdade (QR code) o app mostra só um aviso.
- Curtidas, entradas na sala e comentários **não** viram notificação (são muitos numa live).
- Se o TikTok mudar o site, o encaixe na tela ou a leitura da live podem parar até um ajuste.

## Voltar atrás

Os arquivos de antes estão em `%LOCALAPPDATA%\WestPlay` com o final `.antes-tiktok-real` (`westplay.exe`,
`westplay.dll` e a pasta `ui`), e a DLL de cada jogo em `bin\win_x64\plugins` com o mesmo final. Com o jogo e o
WestPlay fechados, apague os novos e tire o `.antes-tiktok-real` do nome dos antigos.

- [ ] (2026-11-12) Notificação de presente/seguidor: o celular sobe o suficiente para mostrar a barra de status e o aviso inteiro (antes cortava pela metade).


## Aba LIVE sem a barra lateral (2026-11-12)

Antes, na aba **LIVE** do TikTok no celular, a coluna da esquerda (ícone do TikTok, voltar, perfil, câmera,
"Seguindo" com os avatares, "Criadores de LIVE") ficava fixa e espremia as LIVEs para a direita, cortadas.
Agora essa coluna é escondida só na aba LIVE e a lista ocupa a largura toda. Não deu para testar com a conta
logada (sem login o TikTok manda para a tela de entrar), então confira:

- [ ] Aba **LIVE**: sem a coluna da esquerda; as abas "Seguindo / Jogos / Estilo de vida" e as LIVEs ocupam a
  largura do celular, sem corte à direita.
- [ ] Rolar a lista de LIVEs e tocar numa LIVE: abre normal.
- [ ] Voltar para **Para você**, **Seguindo**, **Explorar** e **Perfil**: continuam como antes.
- [ ] A barra de baixo do celular (Voltar / Para você / Seguindo / Explorar / LIVE / Perfil) continua funcionando.

Se a coluna ainda aparecer, mande um print: o TikTok pode ter mudado o desenho da página.

Voltar atrás: em `%LOCALAPPDATA%\WestPlay` os arquivos de antes têm o final `.antes-tiktok-live-lateral`
(`westplay.exe` e a pasta `ui`).


## Botões de volume na lateral + controles do TikTok de volta (2026-11-21)

Por que tinham sumido: para o vídeo ocupar a tela inteira do celular, a barra de baixo do player do TikTok (onde
ficam volume, velocidade e rolagem automática) era escondida, ficando só a linha do progresso. Agora o próprio
WestPlay põe esses controles **no canto de cima à direita** do vídeo. Só o app WestPlay mudou (o jogo não precisa
reiniciar).

Botões na lateral do celular:

- [ ] Do lado **esquerdo** do celular aparecem dois botões pequenos, como os de volume de um iPhone.
- [ ] Com o **TikTok aberto**: o de cima aumenta, o de baixo diminui o volume do vídeo; aparece uma barrinha de
      volume do lado esquerdo e some sozinha.
- [ ] Com **outro app** (ou a tela inicial) e uma música tocando (Spotify, YouTube Music, YouTube): os botões
      mudam o volume dela e mostram a barrinha. Sem nada tocando, não fazem nada.
- [ ] Os botões são fáceis de acertar com o mouse? (Pequenos demais / no lugar errado: mande um print.)

Controles no vídeo do TikTok (canto de cima, à direita):

- [ ] **Alto-falante**: abre um controle deslizante de volume; tocar de novo nele com o controle aberto silencia
      (e de novo volta o som).
- [ ] **1x**: cada toque muda a velocidade (0.5x, 0.75x, 1x, 1.25x, 1.5x, 2x). Vale para os próximos vídeos também.
- [ ] **Auto** (fica vermelho ligado): quando o vídeo termina, passa sozinho para o próximo.
- [ ] Volume, velocidade e Auto continuam como estavam depois de fechar e abrir o TikTok.
- [ ] Nas LIVEs só aparece o volume (velocidade e Auto não servem para live).
- [ ] Os controles não ficam em cima de nada importante do TikTok?

Voltar atrás: em `%LOCALAPPDATA%\MaxDevMod` os arquivos de antes têm o final `.antes-teclas-volume`
(`westplay.exe` e a pasta `ui`).
