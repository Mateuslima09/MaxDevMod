# Tela do caminhão: mapa do jogo preto ao esconder o MaxDevMod (2026-11-20)

## O problema

Com a resolução da tela ligada (Configurações, nitidez acima de "Padrão do jogo"), esconder o MaxDevMod
(botão do caminhão na barra lateral, ou a tecla W) deixava a tela do caminhão preta, em vez de mostrar o mapa
original do jogo. Desligando o MaxDevMod, o mapa aparecia normal.

Causa: para deixar a tela nítida, o plugin troca a imagem inteira da tela do caminhão pela nossa, em alta
resolução. Essa imagem não tem o mapa do jogo por baixo, então, com o nosso painel escondido (transparente),
sobrava preto. A tela dividida já desligava essa troca; esconder o painel não desligava.

## A correção

O app avisa o plugin quando o MaxDevMod está escondido (ponte 53), e a alta resolução fica de lado enquanto
isso: aparece a tela do próprio jogo, com o mapa. Ao abrir o MaxDevMod de novo, a alta resolução volta.

App e plugin foram instalados juntos: **reinicie o jogo** antes de testar.

## Testes

- [ ] Com a resolução da tela em Alta (ou Ultra), esconda o MaxDevMod pelo botão do caminhão: aparece o mapa
      original do jogo, não preto.
- [ ] O mesmo com a tecla W.
- [ ] Toque no botão do MaxDevMod para voltar: o MaxDevMod aparece nítido como antes.
- [ ] Com uma Proporção ajustada nesse caminhão, repita: o mapa aparece ao esconder.
- [ ] Tela dividida continua funcionando (MaxDevMod de um lado, mapa do outro).

Se ainda ficar preto, mande o `Documentos\WestPlay\logs\plugin.log` e diga o caminhão.

Para voltar à versão anterior: `westplay.dll.antes-mapa-escondido` (plugin) e `westplay.exe.antes-mapa-escondido`
(app), em `%LOCALAPPDATA%\MaxDevMod`.

## Scania 2025 (mod): painel de instrumentos piscando com a tela do MaxDevMod (21/11/2026)

O dono viu a nossa tela aparecer piscando dentro do painel de instrumentos de um Scania 2025 de mod; com a
resolução da tela em "Padrão do Jogo" parou. Causa: a tela nítida (`crates/plugin/src/hires.rs`) troca a imagem do
GPS em tudo que a usa, e o painel desse caminhão usa a mesma. Correção: a tela nítida fica desligada nos layouts
`scania-2025*` (`shares_its_target`); a tela deles já é 1024x512. Plugin trocado 21/11 21:02.

- [ ] Volte a resolução para Alta (ou Ultra): o painel não deve mais piscar, e a tela do meio continua nítida.
