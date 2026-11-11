# Abastecimento manual no posto (função futura)

Ideia combinada com o dono (2026-11-11): abastecer a pé, com o personagem segurando o bico, e pagar pelo celular.
Fecha a rotina do motorista começada no mesmo dia (nota fiscal → engate manual → abastecer a pé;
`docs/frete/TESTES.md`).

Ainda **não começado**, por pedido do dono. Este arquivo guarda o plano. Começar só depois de a nota fiscal e o
engate manual terem sido testados no jogo: o abastecimento usa a mesma técnica de reter teclas, e se ela precisar de
ajuste é melhor descobrir antes.

## Como funcionaria

- **Chegada:** parado na bomba, a tecla de abastecer não funciona mais de dentro da cabine; aviso "Desça e abasteça
  o caminhão na bomba".
- **A pé:** descer, ir até o tanque do caminhão; a barra de teclas mostra **E — Abastecer**. O personagem segura o
  bico no tanque, com o som da bomba, e o combustível sobe aos poucos enquanto a tecla fica apertada.
- **Pagamento:** ao soltar, o celular sobe com a cobrança no Nupay (litros, preço por litro, total na moeda do
  frete). Confirmado, pode ir embora.
- **Liga/desliga:** uma linha nos Ajustes do celular, como a da nota fiscal.

## O que já existe e seria reaproveitado

- A retenção de tecla do engate manual, a barra de dicas, o Modo Personagem, os avisos no canto da tela e o Nupay.
- Os dados do jogo: nível do tanque, capacidade e o evento de abastecimento pago (o valor cobrado no celular pode
  ser o real).

## A animação em terceira pessoa

Feita como o agachar e o celular: scripts no Blender (`tools/personagem/`) em cima de animações do Mixamo.

- **Três trechos:** pegar o bico, segurar o bico no tanque (parado, com um leve balanço), devolver.
- **O bico na mão:** um modelo simples, sem marca, preso ao osso da mão direita só nessas animações (como
  `celular_na_mao.py` faz com o celular).
- **Sons:** clique do gatilho, bomba rodando, o "clac" de devolver (como os de porta e de passos).

### A mangueira (a parte difícil)

Ela liga a bomba (parada no mundo) à mão (que se mexe), e o comprimento muda conforme onde o caminhão parou. Os
modelos do personagem não esticam assim. Três saídas:

1. **Mangueira curta presa ao bico** (uns 60 cm, saindo do bico e sumindo para trás do personagem). Simples; fica
   bom de longe em terceira pessoa, de perto se vê que não chega na bomba. **Começar por esta.**
2. **Mangueira desenhada pelo plugin**: uma curva caindo entre a bomba e a mão, redesenhada a cada quadro. A que
   fica bonita de verdade, mas não se sabe se o plugin consegue desenhar um objeto novo no mundo do jogo (hoje ele
   mostra pessoas, não formas livres). Precisa de investigação antes; só vale se a 1 ficar boa no jogo.
3. **Sem mangueira**, só o bico. O mais barato e o menos convincente.

## Pontos incertos e limites

- **Saber que está numa bomba:** o jogo não avisa diretamente. Dois caminhos: os postos dos dados do mapa que o
  WestPlay já lê, ou reter a tecla sempre que o caminhão estiver parado e deixar o jogo decidir se há bomba ali.
  Começar pelo segundo, mais simples.
- **Posição da bomba:** o plugin não enxerga os objetos do posto (o mesmo limite do conferente da nota fiscal). O
  personagem "pegaria" o bico ali mesmo, ao lado do tanque, sem andar até a bomba.
- **Lado e lugar do tanque:** varia por caminhão; uma tabela por modelo, ou sempre o mesmo lado, estimado pela
  posição do caminhão.
- **Ajuste fino** (a mão no bico, a altura no tanque): depende de fotos do dono dentro do jogo, como foi com o
  celular na mão.
- **Controle/volante:** como no engate, só as teclas do teclado seriam retidas.

## Outras ideias anotadas no mesmo dia

- **Inspeção pré-viagem:** dar a volta no caminhão apertando **E** nos pneus, luzes e engate antes de sair, com o
  desgaste real de cada parte mostrado no celular.
- **Descanso a pé:** em vez de dormir pelo menu, descer e ir até a porta da cabine ou do hotel, com o cansaço
  aparecendo no celular.
- **Porta abrindo, descer e subir a escada:** plano próprio em `docs/porta/PLANO.md`.
