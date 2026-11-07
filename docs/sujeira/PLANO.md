# Caminhão sujo e lavagem (função futura)

Pedido do dono (2026-11-05): a lataria, os pneus e a carreta vão sujando conforme o caminhão roda, mais rápido em
estrada de terra, e um jeito de lavar quando estiver muito sujo. Com controles: ligar/desligar, intensidade etc.

Ainda **não começado**. Este arquivo guarda o plano.

## O que o jogo tem (lido no exe do ETS2 1.61, 2026-11-05)

- **Nenhuma sujeira visível nem lavagem** no ETS2/ATS: não há texto, unidade ou parâmetro de sujeira de veículo nem
  de lavar.
- O jogo **sabe o tipo de chão**: os parâmetros `dirt_magnitude`, `dirt_period`, `dirt_*_vibration_*` são o tremor
  do caminhão em chão de terra, e a física tem a tabela de materiais (`INVESTIGACAO.md` §7.7, `exe+0x3674630`: road,
  dirt, road_dirt, gravel, grass, snow, concrete…). Dá pra saber em que chão as rodas estão.
- Ou seja: a sujeira seria **toda do WestPlay** (a lógica e o desenho).

## Etapa 1 — a lógica e os controles (fácil, sem mexer no desenho)

- **Nível de sujeira** de 0 a 100 %, guardado pelo app (por caminhão do perfil, entre as sessões).
- **Sobe com a distância**: devagar no asfalto, mais rápido em terra/cascalho/grama, mais com chuva ou neve no chão;
  a carreta e os pneus sujam junto (os pneus mais rápido).
- **Chuva forte** limpa um pouco, sozinha (só a lataria, não os pneus).
- **Lavar**: um botão no celular e na tela do caminhão ("Lavar o caminhão"). Opção: só em posto ou garagem (o plugin
  sabe onde o caminhão está; a lista de postos sai do mapa que o app já monta). Uma pequena espera e um custo opcional
  (o plugin já mexe no dinheiro do jogo: `money.rs`).
- **Página no app**:
  - ligar/desligar;
  - intensidade (o quanto suja);
  - sujar a carreta junto (sim/não);
  - lavar só em posto (sim/não);
  - ver o nível agora.
- **Aviso** no canto da tela (o aviso do próprio jogo, `INVESTIGACAO.md` §7.9) quando ficar muito sujo, e a barra de
  teclas pode mostrar "Lavar" perto de um posto.

## Etapa 2 — a sujeira aparecer no caminhão (difícil, investigação nova)

Caminhos, do melhor pro pior:

1. **Pelo desenho do jogo (DirectX)**: na hora em que o jogo desenha a lataria, a carreta e os pneus, misturar por
   cima uma textura de sujeira (barro embaixo, poeira em cima), com força conforme o nível. O plugin já se mete no
   desenho do jogo pra tela do caminhão (`d3d.rs`, `screens.rs`); falta achar quais desenhos são os do caminhão e da
   carreta e como misturar (o jogo desenha em várias passadas, "deferred"). Fica bonito e serve pra qualquer caminhão,
   até os de mod. Investigação do tamanho da do personagem; precisa do jogo aberto pra testar.
2. **Pelo mod**: uma "camada de sujeira" nos materiais dentro do `westplay.scs`, em alguns níveis prontos (limpo,
   pouco, muito). Mais simples, mas o nível fica em degraus, troca só ao entrar no caminhão ou ao carregar, e não
   pega bem os caminhões de outros mods.
3. **Só no app/celular** (sem desenho): um medidor de sujeira e a lavagem como "minijogo". Serve de começo, enquanto
   o desenho não existe.

## Ordem sugerida

1. Etapa 1 inteira (lógica, página do app, botão de lavar, aviso) com o nível só no app e no celular.
2. Investigar o caminho 1 da etapa 2 (com o jogo aberto) e anotar em `INVESTIGACAO` próprio desta função.
3. Desenhar a sujeira, primeiro só na lataria do caminhão; depois carreta e pneus.
