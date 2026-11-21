# Tela flutuante do MaxDevMod: plano (2026-11-20)

O pedido: uma tela do próprio MaxDevMod que funcione em **qualquer caminhão**, sem acessório (no frete rápido não
dá para pôr o tablet). Na cabine, ela fica num lugar fixo, num tamanho bom, e dá para mover para frente e para trás,
girar e inclinar.

## Como funciona (protótipo instalado)

1. **Achar a cabine.** A cada desenho da vista principal, o jogo manda para a placa de vídeo um bloco com a posição
   do objeto vista da câmera (a "modelo-vista") e essa posição vezes a projeção. São os mesmos blocos que a câmera de
   ré já usa nos espelhos. O interior da cabine é desenhado no espaço do caminhão. Na vista de dentro, a câmera fica
   na cabeça do motorista, e o SDK diz onde fica a cabeça. Então o bloco que leva a cabeça para dentro da câmera (a
   menos de 35 cm) é da cabine. Entre esses, vence o mais repetido no quadro (a cabine tem muitas peças). Desse bloco
   saem a posição da cabine vista da câmera e a projeção do jogo. Assim a tela fica presa à cabine e acompanha o
   olhar e o balanço da cabeça.
   **Corrigido em 2026-11-20 (pela sonda):** o jogo desenha o interior com o centro na cabeça do motorista, não no
   centro do caminhão. O plugin reconhece a cabine pelas peças desenhadas a menos de 15 cm da câmera (o céu, que
   também fica na câmera, é deixado de fora), e põe a tela nesse espaço.
2. **Desenhar.** No fim de cada quadro (Present), o plugin desenha um retângulo de 30 cm de largura (formato 16:9, com
   uma borda escura de 1 cm), com a imagem da tela do app. Usa o mesmo jeito do celular na tela: um estado de
   desenho próprio, e o do jogo volta intacto.
3. **Lugar.** Sozinha, ela fica 70 cm à frente da cabeça, 28 cm abaixo, puxada para o meio do painel e virada para
   o motorista. O app Tela do celular move (2 cm por toque: frente/trás, sobe/desce, lados) e gira (2° por toque:
   girar, inclinar, tombar), na hora. O lugar fica guardado para cada caminhão em
   `Documentos\WestPlay\<jogo>\tela-flutuante.txt`, junto com ligada/desligada.
4. **Usar.** A tecla da tela e o mouse funcionam nela como na tela do caminhão (o cursor é o do app). Com ela ligada
   e aparecendo, o mouse vai para ela mesmo em caminhão que o MaxDevMod não suporta.
5. **Só quando ligada.** Começa desligada. Liga no celular: app **Tela** (segunda página). Some fora da cabine
   (câmera de fora), com o jogo pausado e com o app fechado.

A ponte agora é a **52** (o app e o plugin vão juntos).

## Riscos e o que falta

- **Escondida pelo volante: ainda não.** Ela é desenhada por cima de tudo, então o volante e os braços não passam
  na frente dela. Próximo passo: copiar a profundidade da cena no fim da passada principal e testar contra ela.
  Isso faz o volante esconder a tela direito. O risco é achar o momento certo da cópia, e se a resolução da cena
  for diferente da janela (escala de resolução).
- **Luz e cor.** Desenhada depois do tom do jogo, ela tem as cores do próprio app, como uma tela acesa de verdade.
  À noite ela fica bem clara; de dia, pode parecer "colada" (sem reflexo, sem sombra). Se incomodar, dá para pôr
  um brilho ajustável ou desenhá-la dentro da cena (aí a luz do jogo mexe nela, o que é difícil acertar).
- **Por cima do menu do jogo?** Ela some com o jogo pausado. Se aparecer por cima de algum menu que não pausa, é
  só avisar.
- **Achar a cabine pode falhar** em algum caminhão de mod (cabine desenhada de outro jeito) ou com a cabeça muito
  longe do lugar padrão. O log diz por quê, a cada 10 s: `tela flutuante: 10 s: ... nearest head N cm`.
- **Custo.** Até 600 blocos lidos por quadro, e para cedo quando acha a cabine. Deve ser leve, mas ver o FPS.
- **Serrilhado.** A imagem de 1280x720 é encolhida sem mipmaps. Se as letras tremerem, dá para melhorar.
- **Tamanho fixo** (30 cm). Se quiser maior ou menor, é uma constante (`WIDTH` em `float_screen.rs`).

## Decisão para o dono

Os controles de edição estão no app **Tela** do celular (setas e botões, como a câmera de ré). Se preferir mexer
com teclas (por exemplo, segurar uma tecla e usar as setas) ou arrastar com o mouse, me diga qual.

Código: `crates/plugin/src/float_screen.rs`, `ui/src/handset/FloatScreenApp.tsx`.
