# GPS melhor em mods de mapa pagos (tarefa futura)

Pedido do dono (2026-11-12): em mods de mapa pagos (protegidos), o Maps só tem uma noção da rota, sem os detalhes
do mapa. Ver se dá pra ler o que o próprio jogo lê, ou pelo menos ter mais detalhe no mapa.

Ainda **não começado**. Este arquivo guarda o plano.

## Como é hoje (lido no código, 2026-11-12)

- O mapa do Maps é montado dos arquivos do jogo e dos mods de mapa do perfil (`crates/setup/src/map.rs`,
  `crates/mapdata`). Um mod cujos arquivos não dá pra ler (arquivo danificado ou **protegido**) é **deixado de
  fora** (`LeftOut`, `left-out.json`) e o mapa é montado com o resto; o companion mostra o nome dele
  (`ui/src/companion/room/parts.tsx`, `leftOut`).
- O que não se lê hoje (`crates/mapdata/src/archive.rs`): entradas **criptografadas**, ZIP64, HashFS versão 2
  **com salt**, versões além de 1 e 2, métodos de hash desconhecidos.
- A **rota** vem da memória do jogo (`crates/host/src/route.rs`): os nós do navegador com posição e rotação. Isso
  funciona mesmo no mod protegido, porque é o jogo que já leu o mapa. Mas o **desenho entre os nós** (curvas,
  faixas dos cruzamentos: `westplay_mapdata::junctions`) e as estradas em volta vêm dos arquivos do mod, que não
  foram lidos. Resultado: a rota vira uma linha entre nós, sem estrada, cruzamento nem cidade em volta.

## Etapa 0 — descobrir (antes de qualquer código)

- Pegar 2 ou 3 mods pagos comuns e anotar **por que** cada um foi deixado de fora (a mensagem do `LeftOut`):
  criptografia, salt, versão, manifesto embaralhado… Pode ser que alguns sejam só um formato que ainda não lemos
  (ex.: HashFS v2 com salt) e não proteção de verdade. Esses são o caminho fácil.
- Ver se o mod fica de fora **inteiro** ou se dá pra ler parte (ex.: o `.sii` de definições sim e os setores não).
  Hoje um erro tira o mod todo; ler o que der já melhora.

## Etapa 1 — ler mais do que já está nos arquivos (fácil)

- Ler o que for só formato não suportado (salt na v2, ZIP64 etc.), se a etapa 0 mostrar que é isso.
- Deixar de tirar o mod inteiro por causa de uma entrada ruim: pular só a entrada, montar o resto do mod.

## Etapa 2 — ler o que o jogo já carregou (a ideia principal)

O jogo precisa abrir o mod pra rodar, então o mapa dele está **na memória do jogo**, já aberto. O host já lê a
memória do jogo só para leitura (`route.rs`, `ReadProcessMemory`), do mesmo jeito daria pra ler mais:

- **A tabela de nós do jogo** (a mesma que o `goto` do console e o `navigation.rs` do plugin usam): posição de
  todos os nós carregados, inclusive os do mod. Com os nós e as ligações entre eles já dá pra desenhar as
  estradas por cima do mapa base.
- **Os itens do mapa carregados** (estradas, prefabs/cruzamentos, cidades, empresas), pelo menos dos setores perto
  do caminhão. Ir guardando conforme o jogador passa (um "mapa que se descobre dirigindo"), num arquivo do
  WestPlay, por perfil.
- Os **pontos do próprio GPS do jogo** entre os nós (o caminho que ele desenha no mapa dele), se ficarem na
  memória, pra rota sair com curva mesmo sem os cruzamentos.

Como no resto: achar tudo pelo formato no código do jogo, nunca por endereço fixo, conferir, e não fazer nada se
não bater (trava só a função, não o app).

## Etapa 2b — usar o GPS original do jogo como base (pedido do dono, 2026-11-12)

O GPS que o jogo mostra na tela do caminhão já tem tudo desenhado, com o mod pago incluído, porque é o jogo que
desenha. Duas formas de aproveitar:

**A. Copiar a imagem do GPS do jogo** (a mais rápida de testar)
- O plugin já fica no meio do desenho dessa tela: o mod do WestPlay põe a nossa imagem como a **camada de cima** do
  GPS do jogo, e o plugin enche essa imagem quando o jogo vai desenhá-la (`crates/plugin/src/screens.rs`,
  `on_set_render_targets` em `d3d.rs`). Nesse momento o jogo **já desenhou o mapa dele por baixo**, no mesmo
  alvo. Dá pra copiar esse alvo antes da nossa camada e mandar a imagem pro app (como o app já manda as dele pro
  jogo, pelo bridge, só que no sentido contrário).
- Bom: é exatamente o que o jogo mostra, mod pago incluso, sem ler arquivo nenhum.
- Ruim: é uma foto. Zoom, ângulo, estilo e resolução são os do jogo (a tela do GPS é pequena); não dá pra deixar
  no visual do WestPlay nem mexer no mapa à vontade. Serve como um modo "GPS original" ou só onde o mod ficou de
  fora.
- A conferir: se o jogo desenha o GPS mesmo com a câmera fora da cabine, ou com a tela do caminhão desligada.

**B. Ler o que o jogo usa pra desenhar o GPS** (mais detalhe, mais trabalho)
- Pra desenhar o GPS o jogo monta, na memória, as estradas em volta e a linha da rota já com as curvas (os pontos
  que vão pra tela). Ler esses pontos (só leitura, como o `route.rs` já faz com a rota) e **desenhar com o nosso
  visual**, com zoom e rotação livres.
- Começar pela **linha da rota** (o que mais falta hoje), depois as estradas em volta.
- Achar pelo formato, nunca por endereço fixo; muda a cada versão do jogo, então tem que conferir e cair fora
  limpo se não bater.

**Juntando:** onde temos o mapa (mods abertos), segue o nosso; onde o mod ficou de fora, usar B (ou A enquanto B
não existir).

## Etapa 3 — mais detalhe sem ler o mod (se as de cima não derem)

- Desenhar a rota com curva suave entre os nós (usando a rotação de cada nó, que já é lida) em vez de linha reta.
- Mostrar no mapa que aquela região é de um mod que não deu pra ler ("detalhes indisponíveis"), em vez de parecer
  um erro.

## Cuidados

- Mods pagos são protegidos de propósito pelos autores. O que for lido serve **só pra desenhar no aparelho do
  próprio jogador**: nunca exportar, juntar num arquivo que dê pra passar adiante, nem quebrar a criptografia dos
  arquivos. Ler o que o jogo já abriu na memória, para uso local, é o caminho mais limpo. Vale conferir os termos
  dos mods mais usados antes.
- Ler a memória do jogo continua só leitura e de fora do jogo (sem nada injetado além do plugin que já existe).

## Pronto quando

- Num perfil com um mod pago, o Maps mostra as estradas do mod (ao menos as que o jogador já passou) e a rota com
  curvas, não só uma linha entre pontos.
